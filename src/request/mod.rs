mod body;
mod builder;
mod path;

pub use body::*;
pub use builder::*;
pub use path::*;
use strum::Display;

use crate::{
    B_ACCEPT_ENCODING, B_AUTHORITY, B_METHOD, B_PATH, B_SCHEME, Encoding, Method, ProtestError,
    RequestHeaders,
};
use quiche::h3::NameValue as _;
use serde::{Deserialize, de::DeserializeOwned};
use std::fmt::Debug;
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
    pub path: String,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
#[strum(serialize_all = "lowercase")]
pub enum RequestParamKind {
    Query,
    Path,
    Header,
    Body,
}

impl RequestStream {
    pub fn builder() -> RequestBuilder<InboundFrameStream> {
        RequestBuilder::new()
    }

    pub fn path_str(&self) -> &str {
        self.path.as_str()
    }

    /// Converts body from streamed i-> buffered and deserializes into type
    pub async fn into_buffered(mut self, capacity: usize) -> RequestBuffer {
        debug!("Converting body into buffered with capacity {capacity}");

        let mut body: Vec<u8> = Vec::with_capacity(capacity);
        while let Some(InboundFrame::Body(bytes, fin)) = self.body.recv().await {
            trace!(fin, "Consume body bytes from stream");
            body.extend(bytes.to_vec());
            if fin {
                break;
            }
        }

        Request {
            method: self.method,
            path: self.path,
            authority: self.authority,
            body,
            body_assosiated: self.body_assosiated,
            headers: self.headers,
            scheme: self.scheme,
        }
    }

    /// Converts body from streamed -> buffered -> T (deserialized)
    pub async fn into_buffered_typed<T>(self, capacity: usize) -> Result<Request<T>, ProtestError>
    where
        T: FromBody + DeserializeOwned,
    {
        let request = self.into_buffered(capacity).await;

        trace!(?request.body, ty=T::content_type_name(), "Deserialzing body");
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
            .body_assosiated(!incoming.read_fin);

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
                    request.path(String::from(RequestHeaders::try_to_owned(&header)?.1));
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

#[cfg(test)]
mod tests {
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
            path: "/api/data".to_string(),
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
            path: "/".to_string(),
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
}
