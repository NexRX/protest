use super::*;
use quiche::h3;

fn method_header(value: &[u8]) -> h3::Header {
    h3::Header::new(b":method", value)
}

#[test]
fn parse_get() {
    assert_eq!(
        Method::try_from(&method_header(b"GET")).unwrap(),
        Method::GET
    );
}

#[test]
fn parse_post() {
    assert_eq!(
        Method::try_from(&method_header(b"POST")).unwrap(),
        Method::POST
    );
}

#[test]
fn parse_put() {
    assert_eq!(
        Method::try_from(&method_header(b"PUT")).unwrap(),
        Method::PUT
    );
}

#[test]
fn parse_delete() {
    assert_eq!(
        Method::try_from(&method_header(b"DELETE")).unwrap(),
        Method::DELETE
    );
}

#[test]
fn parse_patch() {
    assert_eq!(
        Method::try_from(&method_header(b"PATCH")).unwrap(),
        Method::PATCH
    );
}

#[test]
fn parse_options() {
    assert_eq!(
        Method::try_from(&method_header(b"OPTIONS")).unwrap(),
        Method::OPTIONS
    );
}

#[test]
fn parse_head() {
    assert_eq!(
        Method::try_from(&method_header(b"HEAD")).unwrap(),
        Method::HEAD
    );
}

#[test]
fn parse_connect() {
    assert_eq!(
        Method::try_from(&method_header(b"CONNECT")).unwrap(),
        Method::CONNECT
    );
}

#[test]
fn parse_trace() {
    assert_eq!(
        Method::try_from(&method_header(b"TRACE")).unwrap(),
        Method::TRACE
    );
}

#[test]
fn unknown_method_returns_unexpected() {
    assert!(matches!(
        Method::try_from(&method_header(b"PURGE")),
        Err(ParseHeaderError::Unexpected(_, _))
    ));
}

#[test]
fn empty_method_returns_unexpected() {
    assert!(matches!(
        Method::try_from(&method_header(b"")),
        Err(ParseHeaderError::Unexpected(_, _))
    ));
}

#[test]
fn invalid_utf8_returns_bad_value() {
    assert!(matches!(
        Method::try_from(&method_header(&[0xFF, 0xFE])),
        Err(ParseHeaderError::BadValue(_, _))
    ));
}
