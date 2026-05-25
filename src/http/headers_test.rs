use super::*;
use quiche::h3;
use std::collections::HashMap;

fn make_header(name: &[u8], value: &[u8]) -> h3::Header {
    h3::Header::new(name, value)
}

fn find_header<'a>(headers: &'a [h3::Header], name: &[u8]) -> Option<&'a [u8]> {
    use quiche::h3::NameValue as _;
    headers.iter().find(|h| h.name() == name).map(|h| h.value())
}

#[test]
fn try_to_owned_valid_ascii() {
    let result =
        RequestHeaders::try_to_owned(&make_header(b"content-type", b"application/json")).unwrap();
    assert_eq!(
        result,
        ("content-type".to_string(), "application/json".to_string())
    );
}

#[test]
fn try_to_owned_invalid_utf8_in_name() {
    let result = RequestHeaders::try_to_owned(&make_header(&[0xFF, 0xFE], b"value"));
    assert!(matches!(result, Err(ParseHeaderError::BadKey(_, _))));
}

#[test]
fn try_to_owned_invalid_utf8_in_value() {
    let result = RequestHeaders::try_to_owned(&make_header(b"content-type", &[0xFF, 0xFE]));
    assert!(matches!(result, Err(ParseHeaderError::BadValue(_, _))));
}

#[test]
fn insert_authorization() {
    let mut headers = RequestHeaders::default();
    headers
        .try_insert(&make_header(b"authorization", b"Bearer token123"))
        .unwrap();
    assert_eq!(headers.authorization, Some("Bearer token123".to_string()));
}

#[test]
fn insert_cache_control() {
    let mut headers = RequestHeaders::default();
    headers
        .try_insert(&make_header(b"cache-control", b"no-cache"))
        .unwrap();
    assert_eq!(headers.cache_control, Some("no-cache".to_string()));
}

#[test]
fn insert_content_length_valid() {
    let mut headers = RequestHeaders::default();
    headers
        .try_insert(&make_header(b"content-length", b"1024"))
        .unwrap();
    assert_eq!(headers.content_length, Some(1024));
}

#[test]
fn insert_content_length_non_numeric() {
    let mut headers = RequestHeaders::default();
    let result = headers.try_insert(&make_header(b"content-length", b"abc"));
    assert!(matches!(result, Err(ParseHeaderError::ExpectedInt(_, _))));
}

#[test]
fn insert_content_type_json() {
    let mut headers = RequestHeaders::default();
    headers
        .try_insert(&make_header(b"content-type", b"application/json"))
        .unwrap();
    assert_eq!(headers.content_type, Some(mime::APPLICATION_JSON));
}

#[test]
fn insert_encoding_type() {
    let mut headers = RequestHeaders::default();
    headers
        .try_insert(&make_header(b"encoding-type", b"chunked"))
        .unwrap();
    assert_eq!(headers.encoding_type, Some("chunked".to_string()));
}

#[test]
fn insert_origin() {
    let mut headers = RequestHeaders::default();
    headers
        .try_insert(&make_header(b"origin", b"https://example.com"))
        .unwrap();
    assert_eq!(headers.origin, Some("https://example.com".to_string()));
}

#[test]
fn insert_referer() {
    let mut headers = RequestHeaders::default();
    headers
        .try_insert(&make_header(b"referer", b"https://example.com/page"))
        .unwrap();
    assert_eq!(
        headers.referer,
        Some("https://example.com/page".to_string())
    );
}

#[test]
fn insert_user_agent() {
    let mut headers = RequestHeaders::default();
    headers
        .try_insert(&make_header(b"user-agent", b"TestAgent/1.0"))
        .unwrap();
    assert_eq!(headers.user_agent, Some("TestAgent/1.0".to_string()));
}

#[test]
fn insert_unknown_header_goes_to_extra() {
    let mut headers = RequestHeaders::default();
    headers
        .try_insert(&make_header(b"x-custom-header", b"custom-value"))
        .unwrap();
    assert_eq!(
        headers.extra.get("x-custom-header"),
        Some(&"custom-value".to_string())
    );
}

#[test]
fn insert_accept_encoding_gzip() {
    let mut headers = RequestHeaders::default();
    headers
        .try_insert(&make_header(b"accept-encoding", b"gzip"))
        .unwrap();
    assert_eq!(headers.accept_encoding.len(), 1);
    assert!(matches!(headers.accept_encoding[0], Encoding::Gzip(None)));
}

#[test]
fn try_insert_extra_always_goes_to_extra() {
    let mut headers = RequestHeaders::default();
    headers
        .try_insert_extra(&make_header(b"content-type", b"text/plain"))
        .unwrap();
    assert_eq!(
        headers.extra.get("content-type"),
        Some(&"text/plain".to_string())
    );
    assert!(headers.content_type.is_none());
}

#[test]
fn sendable_always_includes_content_encoding() {
    let sendable = ResponseHeaders::default().into_sendable();
    assert!(find_header(&sendable, b"content-encoding").is_some());
}

#[test]
fn sendable_content_length() {
    let sendable = ResponseHeaders {
        content_length: Some(42),
        ..Default::default()
    }
    .into_sendable();
    assert_eq!(
        find_header(&sendable, b"content-length"),
        Some(b"42".as_ref())
    );
}

#[test]
fn sendable_content_type_json() {
    let sendable = ResponseHeaders {
        content_type: Some(mime::APPLICATION_JSON),
        ..Default::default()
    }
    .into_sendable();
    let value = find_header(&sendable, b"content-type").expect("expected a content-type header");
    assert_eq!(value, mime::APPLICATION_JSON.to_string().as_bytes());
}

#[test]
fn sendable_extra_header() {
    let mut extra = HashMap::new();
    extra.insert("x-foo".to_string(), "bar".to_string());
    let sendable = ResponseHeaders {
        extra,
        ..Default::default()
    }
    .into_sendable();
    assert_eq!(find_header(&sendable, b"x-foo"), Some(b"bar".as_ref()));
}

#[test]
fn insert_content_type_invalid() {
    let mut headers = RequestHeaders::default();
    let result = headers.try_insert(&make_header(b"content-type", b"not/a/valid/mime"));
    assert!(matches!(result, Err(ParseHeaderError::Unexpected(_, _))));
}

#[test]
fn insert_content_type_text_html() {
    let mut headers = RequestHeaders::default();
    headers
        .try_insert(&make_header(b"content-type", b"text/html; charset=utf-8"))
        .unwrap();
    assert_eq!(headers.content_type, Some(mime::TEXT_HTML_UTF_8));
}

#[test]
fn insert_accept_encoding_multiple() {
    let mut headers = RequestHeaders::default();
    headers
        .try_insert(&make_header(b"accept-encoding", b"gzip, br"))
        .unwrap();
    assert_eq!(headers.accept_encoding.len(), 2);
}

#[test]
fn insert_multiple_extra_headers() {
    let mut headers = RequestHeaders::default();
    headers
        .try_insert(&make_header(b"x-request-id", b"abc-123"))
        .unwrap();
    headers
        .try_insert(&make_header(b"x-trace-id", b"xyz-456"))
        .unwrap();
    assert_eq!(
        headers.extra.get("x-request-id"),
        Some(&"abc-123".to_string())
    );
    assert_eq!(
        headers.extra.get("x-trace-id"),
        Some(&"xyz-456".to_string())
    );
}

#[test]
fn sendable_cache_control() {
    use crate::http::cache_control::{Cachability, CacheControl};
    use std::time::Duration;
    let cc = CacheControl {
        cachability: Some(Cachability::Public),
        max_age: Some(Duration::from_secs(3600)),
        ..Default::default()
    };
    let sendable = ResponseHeaders {
        cache_control: Some(cc),
        ..Default::default()
    }
    .into_sendable();
    let value = find_header(&sendable, b"cache-control").expect("expected a cache-control header");
    let s = std::str::from_utf8(value).unwrap();
    assert!(s.contains("public"));
    assert!(s.contains("max-age=3600"));
}

#[test]
fn sendable_omits_absent_optional_fields() {
    let sendable = ResponseHeaders::default().into_sendable();
    assert!(find_header(&sendable, b"cache-control").is_none());
    assert!(find_header(&sendable, b"content-length").is_none());
    assert!(find_header(&sendable, b"content-type").is_none());
    assert!(find_header(&sendable, b"encoding-type").is_none());
}

#[test]
fn serde_request_headers_content_type() {
    let json = r#"{"content_type": "application/json"}"#;
    let headers: RequestHeaders = serde_json::from_str(json).unwrap();
    assert_eq!(headers.content_type, Some(mime::APPLICATION_JSON));
}

#[test]
fn serde_request_headers_content_type_absent() {
    let json = r#"{}"#;
    let headers: RequestHeaders = serde_json::from_str(json).unwrap();
    assert!(headers.content_type.is_none());
}

#[test]
fn serde_request_headers_accept_single() {
    let json = r#"{"accept": "text/html"}"#;
    let headers: RequestHeaders = serde_json::from_str(json).unwrap();
    assert_eq!(headers.accept.len(), 1);
    assert_eq!(headers.accept[0], mime::TEXT_HTML);
}

#[test]
fn serde_request_headers_accept_multiple() {
    let json = r#"{"accept": "text/html, application/json"}"#;
    let headers: RequestHeaders = serde_json::from_str(json).unwrap();
    assert_eq!(headers.accept.len(), 2);
    assert!(headers.accept.contains(&mime::TEXT_HTML));
    assert!(headers.accept.contains(&mime::APPLICATION_JSON));
}

#[test]
fn sendable_encoding_type() {
    let sendable = ResponseHeaders {
        encoding_type: Some("chunked".to_string()),
        ..Default::default()
    }
    .into_sendable();
    assert_eq!(
        find_header(&sendable, b"encoding-type"),
        Some(b"chunked".as_ref())
    );
}
