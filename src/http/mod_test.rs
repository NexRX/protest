use super::cache_control::{Cachability, CacheControl};
use super::*;
use quiche::h3::NameValue as _;
use std::time::Duration;

#[test]
fn response_headers_with_cache_control_produces_header() {
    let cc = CacheControl {
        cachability: Some(Cachability::Public),
        max_age: Some(Duration::from_secs(3600)),
        immutable: true,
        ..Default::default()
    };
    let sendable = ResponseHeaders {
        cache_control: Some(cc),
        ..Default::default()
    }
    .into_sendable();
    let value = sendable
        .iter()
        .find(|h| h.name() == B_CACHE_CONTROL)
        .map(|h| std::str::from_utf8(h.value()).unwrap())
        .expect("cache-control header should be present");
    assert!(value.contains("public"));
    assert!(value.contains("max-age=3600"));
    assert!(value.contains("immutable"));
}

#[test]
fn response_headers_without_cache_control_omits_header() {
    let sendable = ResponseHeaders::default().into_sendable();
    assert!(!sendable.iter().any(|h| h.name() == B_CACHE_CONTROL));
}

#[test]
fn parse_header_error_unexpected_display() {
    let err = ParseHeaderError::Unexpected("Method".to_string(), "BOGUS".to_string());
    assert!(err.to_string().contains("BOGUS"));
}

#[test]
fn parse_header_error_expected_int_display() {
    let inner: std::num::ParseIntError = "abc".parse::<u64>().unwrap_err();
    let err = ParseHeaderError::ExpectedInt("content-length".to_string(), inner);
    assert!(err.to_string().contains("content-length"));
}

#[test]
fn request_headers_accept_encoding_stores_multiple() {
    use quiche::h3;
    let mut req_headers = RequestHeaders::default();
    req_headers
        .try_insert(&h3::Header::new(b"accept-encoding", b"gzip br"))
        .unwrap();
    assert_eq!(req_headers.accept_encoding.len(), 2);
    assert!(matches!(
        req_headers.accept_encoding[0],
        Encoding::Gzip(None)
    ));
    assert!(matches!(req_headers.accept_encoding[1], Encoding::Br(None)));
}
