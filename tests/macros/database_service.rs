use protest::{IntegrationTest, Json, Method, assert_response};
use protest_macros::service;
use serde::{Deserialize, Serialize};
use sqlx::{
    Row as _, SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode},
};
use std::str::FromStr as _;
use test_context::test_context;

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    name: String,
    age: u16,
}

#[derive(Debug)]
pub struct DatabaseService {
    pool: sqlx::Pool<sqlx::Sqlite>,
}

#[service]
impl DatabaseService {
    pub fn new(pool: sqlx::Pool<sqlx::Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn new_in_memory() -> Self {
        let opts = SqliteConnectOptions::from_str("sqlite::memory:")
            .unwrap()
            .journal_mode(SqliteJournalMode::Wal)
            .read_only(false);

        let pool = SqlitePool::connect_with(opts).await.unwrap();

        sqlx::query("CREATE TABLE user (name TEXT, age INTEGER)")
            .execute(&pool)
            .await
            .unwrap();

        Self::new(pool)
    }

    #[service(method = POST, path = "/user")]
    async fn create_user(&self, body: Json<User>) {
        sqlx::query("INSERT INTO user VALUES (?, ?)")
            .bind(&body.name)
            .bind(body.age)
            .execute(&self.pool)
            .await
            .unwrap();
    }

    #[service(method = GET, path = "/user")]
    async fn list_users(&self) -> Json<Vec<String>> {
        Json::new(
            sqlx::query("SELECT name FROM user")
                .fetch_all(&self.pool)
                .await
                .unwrap()
                .into_iter()
                .map(|r| r.get::<String, _>("name"))
                .collect::<Vec<_>>(),
        )
    }
}

#[test_context(IntegrationTest)]
#[tokio::test]
async fn integration_test(test: &mut IntegrationTest) {
    test.server().routes(DatabaseService::new_in_memory().await);

    let body = Some(Json::new(User {
        name: "example".into(),
        age: 18,
    }));

    let res = test.send(Method::GET, "/example", None::<String>, 0).await;
    assert_response!(res, NotFound, [], "Route not found");

    let res = test.send(Method::GET, "/user", None::<String>, 0).await;
    assert_response!(res, OK, [], "[]");

    let res = test.send(Method::POST, "/user", body, 0).await;
    assert_response!(res, OK, []);

    let res = test.send(Method::GET, "/user", None::<String>, 0).await;
    assert_response!(res, OK, [], "[\"example\"]")
}
