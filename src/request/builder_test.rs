use super::builder::{RequestBuilder, RequestBuilderError};
use crate::{Method, Request};
use std::path::PathBuf;

fn full_builder() -> RequestBuilder<Vec<u8>> {
    let mut b = RequestBuilder::new();
    b.method(Method::POST)
        .path("/index.html")
        .authority("www.example.com")
        .scheme("https")
        .body(vec![1u8, 2, 3]);
    b
}

#[test]
fn build_fully_valid() {
    let req: Request<Vec<u8>> = full_builder().build().unwrap();
    assert_eq!(req.method, Method::POST);
    assert_eq!(req.path, PathBuf::from("/index.html"));
    assert_eq!(req.authority, "www.example.com");
    assert_eq!(req.scheme, "https");
    assert_eq!(req.body, vec![1u8, 2, 3]);
}

#[test]
fn build_fails_missing_path() {
    let mut b: RequestBuilder<Vec<u8>> = RequestBuilder::new();
    b.authority("www.example.com").scheme("https").body(vec![]);
    assert!(matches!(b.build(), Err(RequestBuilderError::Missing(_))));
}

#[test]
fn build_fails_missing_authority() {
    let mut b: RequestBuilder<Vec<u8>> = RequestBuilder::new();
    b.path("/").scheme("https").body(vec![]);
    assert!(matches!(b.build(), Err(RequestBuilderError::Missing(_))));
}

#[test]
fn build_fails_missing_scheme() {
    let mut b: RequestBuilder<Vec<u8>> = RequestBuilder::new();
    b.path("/").authority("www.example.com").body(vec![]);
    assert!(matches!(b.build(), Err(RequestBuilderError::Missing(_))));
}

#[test]
fn build_fails_missing_body() {
    let mut b: RequestBuilder<Vec<u8>> = RequestBuilder::new();
    b.path("/").authority("www.example.com").scheme("https");
    assert!(matches!(b.build(), Err(RequestBuilderError::Missing(_))));
}

#[test]
fn default_builder_method_and_body_assosiated() {
    let builder: RequestBuilder<Vec<u8>> = RequestBuilder::default();
    assert_eq!(builder.method, Method::GET);
    assert!(!builder.body_assosiated);
}

#[test]
fn body_assosiated_flag_is_set() {
    let mut b: RequestBuilder<Vec<u8>> = RequestBuilder::new();
    b.path("/")
        .authority("localhost")
        .scheme("https")
        .body(vec![])
        .body_assosiated(true);
    let req = b.build().unwrap();
    assert!(req.body_assosiated);
}

#[test]
fn method_setter_changes_method() {
    let mut builder: RequestBuilder<Vec<u8>> = RequestBuilder::new();
    assert_eq!(builder.method, Method::GET);
    builder.method(Method::POST);
    assert_eq!(builder.method, Method::POST);
}

#[test]
fn missing_error_display_contains_expected_prefix() {
    let msg = RequestBuilderError::Missing("Path is required").to_string();
    assert!(msg.contains("Missing required field"));
}
