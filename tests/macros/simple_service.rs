use protest::{IntegrationTest, Method, assert_response};
use protest_macros::service;
use test_context::test_context;

#[derive(Debug)]
pub struct SimpleService;

#[service]
impl SimpleService {
    #[service(method = GET, path = "/hello")]
    fn hello_world() -> &'static str {
        "hello"
    }
}

#[test]
fn smoke_hello_world() {
    assert_eq!(SimpleService::hello_world(), "hello");
}

#[test_context(IntegrationTest)]
#[tokio::test]
async fn integration_test(test: &mut IntegrationTest) {
    test.server().routes(SimpleService);

    let res = test.send(Method::GET, "/hello", None::<String>, 0).await;
    assert_response!(res, OK, [], "hello")
}
