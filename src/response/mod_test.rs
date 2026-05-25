use super::{Response, ResponseStream};
use crate::{ResponseHeaders, Status};

#[test]
fn response_new_sets_status_and_body() {
    let resp: Response<String> = Response::new(Status::OK, "hello".to_string());
    assert_eq!(resp.status, Status::OK);
    assert_eq!(resp.body, "hello");
}

#[test]
fn response_new_with_headers_stores_headers() {
    let headers = ResponseHeaders {
        content_length: Some(99),
        encoding_type: Some("chunked".to_string()),
        ..Default::default()
    };
    let resp: Response<Vec<u8>> = Response::new_with_headers(Status::OK, headers, vec![1, 2, 3]);
    assert_eq!(resp.status, Status::OK);
    assert_eq!(resp.headers.content_length, Some(99));
    assert_eq!(resp.headers.encoding_type, Some("chunked".to_string()));
    assert_eq!(resp.body, vec![1u8, 2, 3]);
}

#[test]
fn route_not_found_has_404_status() {
    assert_eq!(ResponseStream::route_not_found().status, Status::NotFound);
}

#[test]
fn route_not_found_has_default_headers() {
    let resp = ResponseStream::route_not_found();
    assert!(resp.headers.content_type.is_none());
    assert!(resp.headers.content_length.is_none());
}

#[test]
fn fallback_has_500_status() {
    assert_eq!(
        ResponseStream::fallback().status,
        Status::InternalServerError
    );
}

#[test]
fn fallback_has_default_headers() {
    let resp = ResponseStream::fallback();
    assert!(resp.headers.content_type.is_none());
    assert!(resp.headers.content_length.is_none());
}
