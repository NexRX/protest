use protest::{IntegrationTest, Method, assert_response};
use protest_macros::service;
use test_context::test_context;

#[derive(Debug)]
pub struct GenericsService<T> {
    generic: T,
}

#[service]
impl<T: std::fmt::Debug + Send + Sync + 'static> GenericsService<T> {
    pub fn new(generic: T) -> Self {
        Self { generic }
    }

    #[service(method = GET, path = "/hello")]
    async fn debug_world(&self) -> String {
        format!("Debug, World: {:?}", self.generic)
    }
}

#[test_context(IntegrationTest)]
#[tokio::test]
async fn integration_test(test: &mut IntegrationTest) {
    test.server().routes(GenericsService::new("🌍"));

    let res = test.send(Method::GET, "/hello", None::<String>, 0).await;
    assert_response!(res, OK, [], "Debug, World: \"🌍\"");
}
