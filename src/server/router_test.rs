use super::*;
use crate::{Method, Request, RequestHeaders, Response, ResponseHeaders, Status};
use std::path::PathBuf;

fn ok_echo_route(method: Method, path: &str) -> TypedRoute<String, String> {
    TypedRoute {
        method,
        path: PathBuf::from(path),
        handler: RouteHandler::Sync(|x: Request<String>| Response {
            status: Status::OK,
            headers: ResponseHeaders::default(),
            body: x.body,
        }),
    }
}

fn make_request(body: &str) -> Request<String> {
    Request {
        method: Method::GET,
        path: PathBuf::from("/"),
        authority: "localhost".to_string(),
        scheme: "https".to_string(),
        headers: RequestHeaders::default(),
        body: body.to_string(),
        body_assosiated: false,
    }
}

#[test]
fn default_router_is_empty() {
    assert_eq!(Router::default().len(), 0);
}

#[test]
fn add_one_route_len_is_one() {
    let mut router = Router::default();
    router.add_route(ok_echo_route(Method::GET, "/"));
    assert_eq!(router.len(), 1);
}

#[test]
fn add_two_routes_len_is_two() {
    let mut router = Router::default();
    router.add_route(ok_echo_route(Method::GET, "/"));
    router.add_route(ok_echo_route(Method::POST, "/submit"));
    assert_eq!(router.len(), 2);
}

#[test]
fn nest_merges_routes() {
    let mut base = Router::default();
    base.add_route(ok_echo_route(Method::GET, "/a"));
    base.add_route(ok_echo_route(Method::GET, "/b"));

    let mut extra = Router::default();
    extra.add_route(ok_echo_route(Method::GET, "/c"));

    base.nest(extra);
    assert_eq!(base.len(), 3);
}

#[test]
fn typed_route_new_has_correct_method_and_path() {
    let route = TypedRoute::new(
        Method::GET,
        PathBuf::from("/"),
        RouteHandler::Sync(|x: Request<String>| Response {
            status: Status::OK,
            headers: ResponseHeaders::default(),
            body: x.body,
        }),
    );
    assert_eq!(route.method(), Method::GET);
    assert_eq!(route.path(), PathBuf::from("/").as_path());
}

#[tokio::test]
async fn sync_handler_is_called_via_handle() {
    let handler: RouteHandler<String, String> =
        RouteHandler::Sync(|req: Request<String>| Response {
            status: Status::OK,
            headers: ResponseHeaders::default(),
            body: req.body,
        });

    let response = handler.handle(make_request("hello")).await;
    assert_eq!(response.body, "hello");
    assert_eq!(response.status, Status::OK);
}

#[tokio::test]
async fn async_handler_is_called_via_handle() {
    let handler: RouteHandler<String, String> =
        RouteHandler::Async(Box::new(|req: Request<String>| {
            Box::pin(async move {
                Response {
                    status: Status::OK,
                    headers: ResponseHeaders::default(),
                    body: req.body,
                }
            })
        }));

    let response = handler.handle(make_request("hello")).await;
    assert_eq!(response.body, "hello");
    assert_eq!(response.status, Status::OK);
}
