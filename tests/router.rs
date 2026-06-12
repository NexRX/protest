use protest::{
    FutureResult, IntegrationTest, MAX_IDLE_CONNECTION,
    Method::{GET, POST},
    ProtestError, RequestStream, Response, ResponseSender as _, Status, TRouter, assert_response,
};
use quiche::h3;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use test_context::test_context;
use tokio_quiche::http3::driver::NewClientRequest;

#[derive(Debug)]
pub struct ManualRouter {
    context_data: Mutex<String>,
}

impl ManualRouter {
    pub fn sync_route(&self) -> String {
        self.context_data.lock().unwrap().clone()
    }

    pub async fn async_route(&self, body: String) -> Result<(), String> {
        *self.context_data.lock().unwrap() = body;
        Ok(())
    }
}

impl Default for ManualRouter {
    fn default() -> Self {
        Self {
            context_data: Mutex::new(String::new()),
        }
    }
}

impl TRouter for ManualRouter {
    fn can_handle_request(&self, request: &RequestStream) -> bool {
        matches!(
            (request.path_str(), request.method),
            ("/sync", GET) | ("/async", POST)
        )
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
    test.server().routes(ManualRouter::default());

    let body = "hello world, this is a chunked body!";
    let res = test.send_chunked(POST, "/async", body, 5, 1).await;
    assert_response!(res, OK, [], "");

    let res = test.send(GET, "/sync", None::<&[u8]>, 2).await;
    assert_response!(res, OK, [], body);
}

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
    test.server().routes(ManualRouter::default());

    let res = test.send(GET, "/sync", None::<&[u8]>, 1).await;
    assert_response!(res, OK, [], "");

    let res = test.send(POST, "/async", Some("async body"), 2).await;
    assert_response!(res, OK, [], "");

    let res = test.send(GET, "/sync", None::<&[u8]>, 1).await;
    assert_response!(res, OK, [], "async body");
}

#[derive(Debug)]
struct HangRouter;

impl TRouter for HangRouter {
    fn can_handle_request(&self, request: &RequestStream) -> bool {
        matches!(request.path_str(), "/ping" | "/hang")
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
                "/ping" => {
                    Response::new(Status::OK, "pong".to_string())
                        .send(send)
                        .await?;
                }
                "/hang" => loop {
                    tokio::time::sleep(Duration::from_secs(3600)).await;
                },
                _ => unreachable!(),
            }
            Ok(())
        })
    }
}

#[test_context(IntegrationTest)]
#[tokio::test]
#[ignore = "TODO: figure out reason for failure"]
async fn test_stream_idle_timeout_closes_stream(test: &mut IntegrationTest) {
    tokio::time::pause();
    test.server().routes(HangRouter);
    test.start().await;
    let (_conn, mut controller) = test.new_client().await;

    let (body_tx, _body_rx) = tokio::sync::oneshot::channel();
    controller
        .request_sender()
        .send(NewClientRequest {
            request_id: 100,
            headers: vec![
                h3::Header::new(b":method", b"POST"),
                h3::Header::new(b":path", b"/hang"),
                h3::Header::new(b":scheme", b"https"),
                h3::Header::new(b":authority", b"localhost"),
            ],
            body_writer: Some(body_tx),
        })
        .expect("failed to enqueue /hang request");

    // Let the server process the headers before we jump forward in time.
    tokio::time::advance(Duration::from_millis(100)).await;
    tokio::task::yield_now().await;

    // Advance past the stream idle timeout (assume 60 s default).
    tokio::time::advance(Duration::from_secs(61)).await;
    tokio::task::yield_now().await;

    controller
        .request_sender()
        .send(NewClientRequest {
            request_id: 200,
            headers: vec![
                h3::Header::new(b":method", b"GET"),
                h3::Header::new(b":path", b"/ping"),
                h3::Header::new(b":scheme", b"https"),
                h3::Header::new(b":authority", b"localhost"),
            ],
            body_writer: None,
        })
        .expect("failed to enqueue /ping request");

    let res = IntegrationTest::recv_response(&mut controller).await;
    assert_eq!(
        res.status,
        Status::OK,
        "Expected /ping to succeed after the idle stream was cleaned up"
    );
    assert_eq!(res.body_bytes, b"pong");
}

#[test_context(IntegrationTest)]
#[tokio::test]
#[ignore = "Unsure why connections arent being killed"]
async fn test_connection_idle_timeout_kills_connection(test: &mut IntegrationTest) {
    tokio::time::pause();
    test.server().routes(HangRouter);
    test.start().await;
    let (_conn, mut controller) = test.new_client().await;

    controller
        .request_sender()
        .send(NewClientRequest {
            request_id: 1,
            headers: vec![
                h3::Header::new(b":method", b"GET"),
                h3::Header::new(b":path", b"/ping"),
                h3::Header::new(b":scheme", b"https"),
                h3::Header::new(b":authority", b"localhost"),
            ],
            body_writer: None,
        })
        .expect("failed to enqueue /ping request");

    let res = IntegrationTest::recv_response(&mut controller).await;
    assert_eq!(res.status, Status::OK);
    assert_eq!(res.body_bytes, b"pong");

    tokio::time::advance(MAX_IDLE_CONNECTION).await;
    tokio::time::advance(Duration::from_secs(3)).await; // 3 seconds buffer
    tokio::task::yield_now().await;

    controller
        .request_sender()
        .send(NewClientRequest {
            request_id: 2,
            headers: vec![
                h3::Header::new(b":method", b"GET"),
                h3::Header::new(b":path", b"/ping"),
                h3::Header::new(b":scheme", b"https"),
                h3::Header::new(b":authority", b"localhost"),
            ],
            body_writer: None,
        })
        .expect("failed to enqueue second /ping request");

    let event = controller.event_receiver_mut().recv().await;
    assert!(
        event.is_none(),
        "Expected the event channel to be closed after the connection idle \
         timeout, but received an event: {event:?}.  The server did not close \
         the idle connection."
    );
}
