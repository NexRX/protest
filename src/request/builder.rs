use crate::{Method, ParseHeaderError, Request, RequestHeaders};
use std::fmt::Debug;
use std::path::PathBuf;

#[derive(Debug)]
pub struct RequestBuilder<T: Debug> {
    /// The HTTP method of the request (e.g., GET, POST).
    pub method: Method,
    /// The path of the request (e.g., "/index.html").
    pub path: Option<PathBuf>,
    /// The authority of the request, essentially the server host (e.g., "www.example.com").
    pub authority: Option<String>,
    /// The scheme of the request, almost always "https".
    pub scheme: Option<String>,
    /// Other standard and non-standard headers associated with the request.
    pub headers: RequestHeaders,
    pub body: Option<T>,
    pub body_assosiated: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum RequestBuilderError {
    #[error("Missing required field: {0}")]
    Missing(&'static str),
    #[error("{0}")]
    Header(ParseHeaderError),
}

impl From<ParseHeaderError> for RequestBuilderError {
    fn from(e: ParseHeaderError) -> Self {
        Self::Header(e)
    }
}

impl From<&'static str> for RequestBuilderError {
    fn from(s: &'static str) -> Self {
        Self::Missing(s)
    }
}

impl<T: Debug> RequestBuilder<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn method(&mut self, method: impl Into<Method>) -> &mut Self {
        self.method = method.into();
        self
    }

    pub fn path(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.path = Some(path.into());
        self
    }

    pub fn authority(&mut self, authority: impl Into<String>) -> &mut Self {
        self.authority = Some(authority.into());
        self
    }

    pub fn scheme(&mut self, scheme: impl Into<String>) -> &mut Self {
        self.scheme = Some(scheme.into());
        self
    }

    pub fn headers(&mut self, headers: impl Into<RequestHeaders>) -> &mut Self {
        self.headers = headers.into();
        self
    }

    pub fn body(&mut self, body: impl Into<T>) -> &mut Self {
        self.body = Some(body.into());
        self
    }

    pub fn body_assosiated(&mut self, value: bool) -> &mut Self {
        self.body_assosiated = value;
        self
    }

    pub fn build(self) -> Result<Request<T>, RequestBuilderError> {
        Ok(Request {
            method: self.method,
            path: self.path.ok_or("Path is required")?,
            authority: self.authority.ok_or("Authority is required")?,
            scheme: self.scheme.ok_or("Scheme is required")?,
            headers: self.headers,
            body: self.body.ok_or("Body is required")?,
            body_assosiated: self.body_assosiated,
        })
    }
}

impl<T: Debug> Default for RequestBuilder<T> {
    fn default() -> Self {
        Self {
            method: Method::GET,
            path: None,
            authority: None,
            scheme: None,
            headers: RequestHeaders::default(),
            body: None,
            body_assosiated: false,
        }
    }
}
