use crate::{Method, Request, RequestHeaders};
use std::path::PathBuf;

#[test]
fn from_body_parses_valid_json_value() {
    use crate::FromBody;
    let val: serde_json::Value =
        serde_json::Value::from_body(br#"{"key": "value", "num": 42}"#).unwrap();
    assert_eq!(val["key"], "value");
    assert_eq!(val["num"], 42);
}

#[test]
fn from_body_errors_on_invalid_json() {
    use crate::FromBody;
    assert!(serde_json::Value::from_body(b"not json at all {{{").is_err());
}

#[test]
fn request_buffer_fields_are_accessible() {
    let req: Request<Vec<u8>> = Request {
        method: Method::POST,
        path: PathBuf::from("/api/data"),
        authority: "api.example.com".to_string(),
        scheme: "https".to_string(),
        headers: RequestHeaders::default(),
        body: vec![1, 2, 3],
        body_assosiated: true,
    };

    assert_eq!(req.method, Method::POST);
    assert_eq!(req.path, PathBuf::from("/api/data"));
    assert_eq!(req.authority, "api.example.com");
    assert_eq!(req.scheme, "https");
    assert_eq!(req.body, vec![1u8, 2, 3]);
    assert!(req.body_assosiated);
}

#[test]
fn request_default_headers_are_empty() {
    let req: Request<Vec<u8>> = Request {
        method: Method::GET,
        path: PathBuf::from("/"),
        authority: "localhost".to_string(),
        scheme: "https".to_string(),
        headers: RequestHeaders::default(),
        body: vec![],
        body_assosiated: false,
    };

    assert!(req.headers.accept.is_empty());
    assert!(req.headers.accept_encoding.is_empty());
    assert!(req.headers.authorization.is_none());
    assert!(req.headers.content_type.is_none());
    assert!(req.headers.content_length.is_none());
    assert!(req.headers.origin.is_none());
    assert!(req.headers.user_agent.is_none());
    assert!(req.headers.extra.is_empty());
}
