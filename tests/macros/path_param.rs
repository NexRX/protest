use protest::{IntegrationTest, Method, assert_response};
use protest_macros::service;
use serde::{Deserialize, Serialize};
use test_context::test_context;

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    name: String,
    age: u16,
}

#[derive(Debug, Default)]
pub struct DatabaseService;

#[service]
impl DatabaseService {
    #[service(method = GET, path = "/user/string/:name")]
    fn age_via_string(&self, name: String) -> usize {
        match &*name {
            "john" => 18,
            "smith" => 20,
            _ => 0,
        }
    }

    #[service(method = GET, path = "/user/str/:name")]
    fn age_via_str(&self, name: &str) -> usize {
        match &*name {
            "john" => 18,
            "smith" => 20,
            _ => 0,
        }
    }
}

#[test_context(IntegrationTest)]
#[tokio::test]
async fn integration_test(test: &mut IntegrationTest) {
    test.server().routes(DatabaseService::default());

    let res = test
        .send(Method::GET, "/user/string/john", None::<String>, 0)
        .await;
    assert_response!(res, OK, [], "18");

    let res = test
        .send(Method::GET, "/user/string/smith", None::<String>, 0)
        .await;
    assert_response!(res, OK, [], "20");
}
