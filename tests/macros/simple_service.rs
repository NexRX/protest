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

    #[service(method = POST, path = "/hello/alloc", alloc_body)]
    fn hello_world_alloc(body: String) -> String {
        format!("hello {body}")
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
    assert_response!(res, OK, [], "hello");

    let res = test
        .send(Method::POST, "/hello/alloc", None::<String>, 0)
        .await;
    assert_response!(res, OK, [], "hello alloc");
}
