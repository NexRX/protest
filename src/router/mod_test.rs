use crate::{
    Method::{GET, POST},
    Response, Status, TRouter,
    error::ServerError,
    integration_test::IntegrationTest,
};
use crate::{ResponseSender as _, assert_response};
use std::sync::Mutex;
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
    fn can_handle_request(&self, request: &crate::RequestStream) -> bool {
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
        request: crate::RequestStream,
        send: &'a mut tokio_quiche::http3::driver::OutboundFrameSender,
    ) -> crate::FutureResult<'a, (), crate::error::ServerError> {
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
                _ => Err(ServerError::NoRoute),
            }
        })
    }
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
