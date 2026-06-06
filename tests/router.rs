use protest::{
    FutureResult, IntegrationTest,
    Method::{GET, POST},
    ProtestError, RequestStream, Response, ResponseSender as _, Status, TRouter, assert_response,
};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use test_context::test_context;

#[derive(Debug)]
pub struct ManualRouter {
    context_data: Mutex<String>,
}

impl ManualRouter {
    pub fn new() -> Self {
        Self {
            context_data: Mutex::new(String::new()),
        }
    }

    pub fn sync_route(&self) -> String {
        self.context_data.lock().unwrap().clone()
    }

    pub async fn async_route(&self, body: String) -> Result<(), String> {
        *self.context_data.lock().unwrap() = body;
        Ok(())
    }
}

impl TRouter for ManualRouter {
    fn can_handle_request(&self, request: &RequestStream) -> bool {
        match (request.path_str(), request.method) {
            ("/sync", GET) => true,
            ("/async", POST) => true,
            _ => false,
        }
    }

    fn len(&self) -> usize {
        2
    }

    fn handle_request<'a>(
        &'a self,
        request: RequestStream,
        send: &'a mut tokio_quiche::http3::driver::OutboundFrameSender,
    ) -> FutureResult<'a, (), ProtestError> {
        Box::pin(async move {
            match (request.path_str(), request.method) {
                ("/sync", GET) => {
                    let response_body = self.sync_route();
                    Response::new(Status::OK, response_body).send(send).await?;
                    Ok(())
                }
                ("/async", POST) => {
                    let request = request.into_buffered_typed::<String>(0).await?;
                    let response_body = self.async_route(request.body).await;
                    Response::new(Status::OK, response_body).send(send).await?;
                    Ok(())
                }
                _ => Err(ProtestError::NoRoute),
            }
        })
    }
}

#[test_context(IntegrationTest)]
#[tokio::test]
async fn test_chunked_post_body_is_received(test: &mut IntegrationTest) {
    test.server().routes(ManualRouter::new());

    // Send a POST body split into 5-byte chunks.  This forces multiple
    // BodyBytesReceived events with fin=false before the final fin=true,
    // exercising the (true, false) arm in the controller.
    let body = "hello world, this is a chunked body!";
    let res = test.send_chunked(POST, "/async", body, 5, 1).await;
    assert_response!(res, OK, [], "");

    // Read back the stored body to prove every chunk was collected.
    let res = test.send(GET, "/sync", None::<&[u8]>, 2).await;
    assert_response!(res, OK, [], body);
}

/// A router with a `/slow` endpoint that sleeps, used to prove the
/// controller serialises request handling instead of multiplexing.
#[derive(Debug)]
struct SlowRouter;

impl TRouter for SlowRouter {
    fn can_handle_request(&self, request: &RequestStream) -> bool {
        matches!(request.path_str(), "/slow" | "/fast")
    }

    fn len(&self) -> usize {
        2
    }

    fn handle_request<'a>(
        &'a self,
        request: RequestStream,
        send: &'a mut tokio_quiche::http3::driver::OutboundFrameSender,
    ) -> FutureResult<'a, (), ProtestError> {
        Box::pin(async move {
            match request.path_str() {
                "/slow" => {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    Response::new(Status::OK, "slow".to_string())
                        .send(send)
                        .await?;
                }
                "/fast" => {
                    Response::new(Status::OK, "fast".to_string())
                        .send(send)
                        .await?;
                }
                _ => unreachable!(),
            }
            Ok(())
        })
    }
}

/// HTTP/3 multiplexes streams on a single QUIC connection.  Two slow
/// handlers should run concurrently and finish in roughly the cost of a
/// single sleep (~1 s), not twice that (~2 s back-to-back).
///
/// This guards against regressions to serial dispatch in the controller.
#[test_context(IntegrationTest)]
#[tokio::test]
async fn test_h3_streams_are_multiplexed(test: &mut IntegrationTest) {
    test.server().routes(SlowRouter);

    let start = Instant::now();
    let responses = test
        .send_concurrent(vec![(GET, "/slow", None), (GET, "/slow", None)])
        .await;
    let elapsed = start.elapsed();

    assert_eq!(responses.len(), 2);
    for res in &responses {
        assert_eq!(res.status, Status::OK);
    }

    // Two 1 s handlers running in parallel should finish in ~1 s.
    // If they are serialised the wall-time will be ~2 s.
    assert!(
        elapsed < Duration::from_millis(1500),
        "Expected < 1500 ms with multiplexed streams, but took {:?}. \
         The two /slow handlers ran sequentially instead of concurrently \
         — streams are serialised, not multiplexed.",
        elapsed,
    );
}

#[test_context(IntegrationTest)]
#[tokio::test]
async fn test_server_with_manual_route(test: &mut IntegrationTest) {
    test.server().routes(ManualRouter::new());

    let res = test.send(GET, "/sync", None::<&[u8]>, 1).await;
    assert_response!(res, OK, [], "");

    let res = test.send(POST, "/async", Some("async body"), 2).await;
    assert_response!(res, OK, [], "");

    let res = test.send(GET, "/sync", None::<&[u8]>, 1).await;
    assert_response!(res, OK, [], "async body");
}
