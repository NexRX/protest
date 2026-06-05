use crate::{Method, ParseHeaderError, Request, RequestHeaders};
use std::fmt::Debug;

#[derive(Debug)]
pub struct RequestBuilder<T: Debug> {
    /// The HTTP method of the request (e.g., GET, POST).
    pub method: Method,
    /// The path of the request (e.g., "/index.html").
    pub path: Option<String>,
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

    pub fn path(&mut self, path: impl Into<String>) -> &mut Self {
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

#[cfg(test)]
mod tests {
    use super::*;
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
}
