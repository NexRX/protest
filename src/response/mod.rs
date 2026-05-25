#[cfg(test)]
mod mod_test;

mod body;
pub use body::*;

use crate::{B_CONTENT_TYPE, B_STATUS, ResponseHeaders, Status};
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

pub struct Response<T> {
    pub status: Status,
    pub headers: ResponseHeaders,
    pub body: T,
}

impl<T> Response<T> {
    pub fn new(status: impl Into<Status>, body: impl Into<T>) -> Self {
        Self::new_with_headers(status, ResponseHeaders::default(), body)
    }

    pub fn new_with_headers(
        status: impl Into<Status>,
        headers: ResponseHeaders,
        body: impl Into<T>,
    ) -> Self {
        Self {
            status: status.into(),
            headers,
            body: body.into(),
        }
    }

    async fn send_headers(
        &mut self,
        send: &mut OutboundFrameSender,
        default_content_type: Mime,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut headers = self.headers.clone().into_sendable();
        headers.insert(0, h3::Header::new(B_STATUS, self.status.bytes().as_ref()));

        if self.headers.content_type.is_none() {
            headers.push(h3::Header::new(
                B_CONTENT_TYPE,
                default_content_type.to_string().as_bytes(),
            ));
        }

        send.send(OutboundFrame::Headers(headers, None)).await?;
        Ok(())
    }
}

impl<T: Debug> Debug for Response<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Response")
            .field("status", &self.status)
            .field("headers", &self.headers)
            .field("body", &self.body)
            .finish()
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

pub struct TypedResponse<T> {
    pub status: Status,
    pub body: T,
}
