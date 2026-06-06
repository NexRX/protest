use protest::{IntegrationTest, Method, RequestError, assert_response};
use protest_macros::service;
use serde::{Deserialize, Serialize};
use test_context::test_context;
use uuid::{Uuid, uuid};

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    name: String,
    age: u16,
}

#[derive(Debug, Default)]
pub struct DatabaseService;

#[service]
impl DatabaseService {
    const JOHN_UUID: Uuid = uuid!("00000000-0000-0000-0000-000000000000");
    const SMITH_UUID: Uuid = uuid!("00000000-0000-0000-0000-000000000001");

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

    #[service(method = GET, path = "/user/uuid/:uuid")]
    fn age_via_uuid(&self, uuid: Uuid) -> usize {
        match uuid {
            JOHN_UUID => 18,
            SMITH_UUID => 20,
            _ => 0,
        }
    }
}

#[test_context(IntegrationTest)]
#[tokio::test]
async fn integration_test(test: &mut IntegrationTest) {
    test.server().routes(DatabaseService::default());

    // let value = "john".to_string();
    // let u =
    //     <String as TryInto<Uuid>>::try_into(value.clone()).map_err(|err| RequestError::Invalid {
    //         name: "uuid".into(),
    //         kind: protest::RequestParamKind::Path,
    //         raw_value: Some(value),
    //         target_type: "Uuid".into(),
    //         message: err.to_string(),
    //     });

    let res = test
        .send(Method::GET, "/user/string/john", None::<String>, 0)
        .await;
    assert_response!(res, OK, [], "18");

    let res = test
        .send(Method::GET, "/user/string/smith", None::<String>, 0)
        .await;
    assert_response!(res, OK, [], "20");
}
