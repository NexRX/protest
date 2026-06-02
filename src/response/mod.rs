mod body;
mod sender;

pub use body::*;
pub use sender::*;

use crate::{B_CONTENT_LENGTH, B_CONTENT_TYPE, B_STATUS, ResponseHeaders, Status};
use bytes::Bytes;
use futures_util::{
    SinkExt as _, StreamExt as _,
    stream::{self, BoxStream},
};
use mime::Mime;
use quiche::h3;
use std::fmt::Debug;
use tokio_quiche::http3::driver::{OutboundFrame, OutboundFrameSender};

pub type ResponseStream = Response<BoxStream<'static, Bytes>>;

#[derive(Debug)]
pub struct Response<T: ResponseBody> {
    pub status: Status,
    pub headers: ResponseHeaders,
    pub body: T,
}

impl<T: ResponseBody> Response<T> {
    pub fn new(status: impl Into<Status>, body: T) -> Self {
        Self::new_with_headers(status, ResponseHeaders::default(), body)
    }

    pub fn new_with_headers(status: impl Into<Status>, headers: ResponseHeaders, body: T) -> Self {
        Self {
            status: status.into(),
            headers,
            body,
        }
    }

    async fn send_headers(
        &mut self,
        send: &mut OutboundFrameSender,
        default_content_type: Option<Mime>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut headers = self.headers.clone().into_sendable();
        headers.insert(0, h3::Header::new(B_STATUS, self.status.bytes().as_ref()));

        if self.headers.content_type.is_none()
            && let Some(content_type) = default_content_type
        {
            headers.push(h3::Header::new(
                B_CONTENT_TYPE,
                content_type.to_string().as_bytes(),
            ));
        }

        if let Some(size) = self.body.size() {
            headers.push(h3::Header::new(
                B_CONTENT_LENGTH,
                size.to_string().as_bytes(),
            ));
        }

        send.send(OutboundFrame::Headers(headers, None)).await?;
        Ok(())
    }
}

impl ResponseStream {
    pub fn route_not_found() -> Self {
        Self {
            status: Status::NotFound,
            headers: ResponseHeaders::default(),
            body: stream::once(async { Bytes::from_static(b"Route not found") }).boxed(),
        }
    }

    pub fn fallback() -> Self {
        Self {
            status: Status::InternalServerError,
            headers: ResponseHeaders::default(),
            body: stream::once(async { Bytes::from_static(b"Internal server error") }).boxed(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let resp: Response<Vec<u8>> =
            Response::new_with_headers(Status::OK, headers, vec![1, 2, 3]);
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
}
