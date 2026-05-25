mod body;
mod builder;

pub use body::*;

#[cfg(test)]
mod builder_test;
#[cfg(test)]
mod mod_test;

use crate::{
    B_ACCEPT_ENCODING, B_AUTHORITY, B_METHOD, B_PATH, B_SCHEME, Encoding, Method, RequestHeaders,
};
use builder::*;
use quiche::h3::NameValue as _;
use serde::{Deserialize, de::DeserializeOwned};
use std::{fmt::Debug, path::PathBuf};
use tokio_quiche::http3::driver::{
    InboundFrame, InboundFrameStream, IncomingH3Headers, OutboundFrameSender,
};
use tracing::{debug, trace};

pub type RequestStream = Request<InboundFrameStream>;
pub type RequestBuffer = Request<Vec<u8>>;

#[derive(Debug, Deserialize)]
pub struct Request<T: Debug> {
    /// The HTTP method of the request (e.g., GET, POST).
    pub method: Method,
    /// The path of the request (e.g., "/index.html").
    pub path: PathBuf,
    /// The authority of the request, essentially the server host (e.g., "www.example.com").
    pub authority: String,
    /// The scheme of the request, almost always "https".
    pub scheme: String,
    /// Other standard and non-standard headers associated with the request.
    pub headers: RequestHeaders,
    /// Request body
    pub body: T,
    /// If there is a body assosiated with the request
    pub(crate) body_assosiated: bool,
}

impl RequestStream {
    pub fn builder() -> RequestBuilder<InboundFrameStream> {
        RequestBuilder::new()
    }

    /// Converts body from streamed i-> buffered and deserializes into type
    pub async fn into_buffered(
        mut self,
        capacity: usize,
    ) -> Result<RequestBuffer, serde_json::Error> {
        debug!("Converting body into buffered with capacity {capacity}");

        let mut body: Vec<u8> = Vec::with_capacity(capacity);
        while let Some(InboundFrame::Body(bytes, fin)) = self.body.recv().await {
            trace!(fin, "Consume body bytes from stream");
            body.extend(bytes.to_vec());
            if fin {
                break;
            }
        }

        Ok(Request {
            method: self.method,
            path: self.path,
            authority: self.authority,
            body,
            body_assosiated: self.body_assosiated,
            headers: self.headers,
            scheme: self.scheme,
        })
    }

    /// Converts body from streamed i-> buffered and deserializes into type
    pub async fn into_typed_with_json<T>(
        self,
        capacity: usize,
    ) -> Result<Request<T>, serde_json::Error>
    where
        T: FromBody + DeserializeOwned,
        serde_json::Error: From<<T as FromBody>::Error>,
    {
        debug!("Converting body into buffered with capacity {capacity}");

        let request = self.into_buffered(capacity).await?;

        trace!(?request.body, "Deserialzing body");
        let body = T::from_body(&request.body)?;

        Ok(Request {
            method: request.method,
            path: request.path,
            authority: request.authority,
            body,
            body_assosiated: request.body_assosiated,
            headers: request.headers,
            scheme: request.scheme,
        })
    }

    pub fn try_from_incoming(
        incoming: IncomingH3Headers,
    ) -> Result<(Self, OutboundFrameSender), RequestBuilderError> {
        let mut request = Self::builder();
        request
            .body(incoming.recv)
            .body_assosiated(incoming.read_fin);

        for header in incoming.headers {
            match header.name() {
                // Required headers
                B_AUTHORITY => {
                    request.authority(RequestHeaders::try_to_owned(&header)?.1);
                }
                B_METHOD => {
                    request.method(Method::try_from(&header)?);
                }
                B_PATH => {
                    request.path(PathBuf::from(RequestHeaders::try_to_owned(&header)?.1));
                }
                B_SCHEME => {
                    request.scheme(RequestHeaders::try_to_owned(&header)?.1);
                }
                B_ACCEPT_ENCODING => {
                    request.headers.accept_encoding = Encoding::try_from_header(&header)?;
                }
                _extra => {
                    request.headers.try_insert(&header)?;
                }
            }
        }
        request.build().map(|req| (req, incoming.send))
    }
}
