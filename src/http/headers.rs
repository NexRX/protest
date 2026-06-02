use crate::{Encoding, http::cache_control::CacheControl};
use mime::Mime;
use quiche::h3::{self, NameValue as _};
use serde::{Deserialize, Deserializer};
use std::{collections::HashMap, str::FromStr};

// Required
pub const B_AUTHORITY: &[u8] = b":authority";
pub const B_METHOD: &[u8] = b":method";
pub const B_PATH: &[u8] = b":path";
pub const B_SCHEME: &[u8] = b":scheme";
pub const B_STATUS: &[u8] = b":status"; // Response only

// Optional (non-extra)
pub const ACCEPT_ENCODING: &str = "accept-encoding";
pub const AUTHORIZATION: &str = "authorization";
pub const CACHE_CONTROL: &str = "cache-control";
pub const CONTENT_LENGTH: &str = "content-length";
pub const CONTENT_TYPE: &str = "content-type";
pub const CONTENT_ENCODING: &str = "content-encoding";
pub const ENCODING_TYPE: &str = "encoding-type";
pub const ORIGIN: &str = "origin";
pub const REFERER: &str = "referer";
pub const USER_AGENT: &str = "user-agent";

pub const B_ACCEPT_ENCODING: &[u8] = b"accept-encoding";
pub const B_AUTHORIZATION: &[u8] = b"authorization";
pub const B_CACHE_CONTROL: &[u8] = b"cache-control";
pub const B_CONTENT_LENGTH: &[u8] = b"content-length";
pub const B_CONTENT_TYPE: &[u8] = b"content-type";
pub const B_CONTENT_ENCODING: &[u8] = b"content-encoding";
pub const B_ENCODING_TYPE: &[u8] = b"encoding-type";
pub const B_ORIGIN: &[u8] = b"origin";
pub const B_REFERER: &[u8] = b"referer";
pub const B_USER_AGENT: &[u8] = b"user-agent";

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct RequestHeaders {
    /// Acceptable MIME types acceptable, empty if not specified or *any \*/\** is given.
    #[serde(deserialize_with = "parse_mimes")]
    pub accept: Vec<Mime>,
    /// Acceptable encodings acceptable, empty if not specified.
    pub accept_encoding: Vec<Encoding>,
    /// The authorization credentials for the request, e.g. `Bearer <token>`.
    pub authorization: Option<String>,
    /// The cache control directives for the request, e.g. `no-cache`.
    pub cache_control: Option<String>,
    /// The length of the request body in bytes, e.g. `1024`.
    pub content_length: Option<usize>,
    /// The MIME type of the request body, e.g. `application/json`.
    #[serde(deserialize_with = "parse_mime")]
    pub content_type: Option<Mime>,
    /// The encoding type of the request body, e.g. `chunked`.
    pub encoding_type: Option<String>,
    /// The origin of the request, e.g. `https://example.com`.
    pub origin: Option<String>,
    /// The URL of the page that linked to the requested resource, e.g. `https://example.com/page`.
    pub referer: Option<String>,
    /// The user agent string of the client, e.g. `Mozilla/5.0`.
    pub user_agent: Option<String>,
    /// Any additional headers not covered by the other fields, stored as key-value pairs.
    pub extra: HashMap<String, String>,
    // TODO: cookie?
}

impl RequestHeaders {
    // --------- Methods ----------

    pub fn try_insert(&mut self, header: &h3::Header) -> Result<&mut Self, ParseHeaderError> {
        let (name, value) = Self::try_to_owned(header)?;
        match header.name() {
            B_ACCEPT_ENCODING => {
                self.accept_encoding = Encoding::try_from_header(header)?;
            }
            B_AUTHORIZATION => {
                self.authorization = Some(value);
            }
            B_CACHE_CONTROL => {
                self.cache_control = Some(value);
            }
            B_CONTENT_LENGTH => {
                self.content_length = Some(
                    value
                        .parse()
                        .map_err(|e| ParseHeaderError::ExpectedInt(name, e))?,
                );
            }
            B_CONTENT_TYPE => {
                self.content_type = Some(
                    Mime::from_str(&value)
                        .map_err(|e| ParseHeaderError::Unexpected(name, e.to_string()))?,
                );
            }
            B_ENCODING_TYPE => {
                self.encoding_type = Some(value);
            }
            B_ORIGIN => {
                self.origin = Some(value);
            }
            B_REFERER => {
                self.referer = Some(value);
            }
            B_USER_AGENT => {
                self.user_agent = Some(value);
            }
            _ => {
                self.extra.insert(name, value);
            }
        }
        Ok(self)
    }

    pub fn try_insert_extra(&mut self, value: &h3::Header) -> Result<&mut Self, ParseHeaderError> {
        let (name, value) = Self::try_to_owned(value)?;
        self.extra.insert(name, value);
        Ok(self)
    }

    // --------- Functions ----------

    pub fn try_to_owned(value: &h3::Header) -> Result<(String, String), ParseHeaderError> {
        let name = str::from_utf8(value.name())
            .map_err(|e| ParseHeaderError::BadKey(value.name().to_owned(), e))?
            .to_string();
        let value = str::from_utf8(value.value())
            .map_err(|e| ParseHeaderError::BadValue(name.clone(), e))?
            .to_string();

        Ok((name, value))
    }
}

#[derive(Debug, Clone, Default)]
pub struct ResponseHeaders {
    /// The encodings used to encode the response body, e.g. `gzip`.
    pub content_encoding: Encoding,
    /// The cache control directives for the response, e.g. `no-cache`.
    pub cache_control: Option<CacheControl>,
    /// The length of the response body in bytes, e.g. `1024`.
    pub content_length: Option<usize>,
    /// The MIME type of the response body, e.g. `application/json`.
    pub content_type: Option<Mime>,
    /// The encoding type of the response body, e.g. `chunked`.
    pub encoding_type: Option<String>,
    /// Any additional headers not covered by the other fields, stored as key-value pairs.
    pub extra: HashMap<String, String>,
}

impl ResponseHeaders {
    pub fn into_sendable(self) -> Vec<h3::Header> {
        let mut headers = vec![h3::Header::new(
            B_CONTENT_ENCODING,
            self.content_encoding.to_string().as_bytes(),
        )];

        if let Some(cache_control) = self.cache_control {
            headers.push(h3::Header::new(
                B_CACHE_CONTROL,
                cache_control.to_string().as_bytes(),
            ));
        }

        if let Some(content_length) = self.content_length {
            headers.push(h3::Header::new(
                B_CONTENT_LENGTH,
                content_length.to_string().as_bytes(),
            ));
        }

        if let Some(content_type) = self.content_type {
            headers.push(h3::Header::new(
                B_CONTENT_TYPE,
                content_type.to_string().as_bytes(),
            ));
        }

        if let Some(encoding_type) = self.encoding_type {
            headers.push(h3::Header::new(B_ENCODING_TYPE, encoding_type.as_bytes()));
        }

        for header in self.extra {
            headers.push(h3::Header::new(header.0.as_bytes(), header.1.as_bytes()));
        }

        headers
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseHeaderError {
    #[error("Failed to parse header {0} float: {1}")]
    ExpectedFloat(String, std::num::ParseFloatError),
    #[error("Failed to parse header {0} int: {1}")]
    ExpectedInt(String, std::num::ParseIntError),
    #[error("Unexpected {0}: {1}")]
    Unexpected(String, String),
    #[error("Failed to parse {0:?} key as UTF-8: {1}")]
    BadKey(Vec<u8>, std::str::Utf8Error),
    #[error("Failed to parse {0} value as UTF-8: {1}")]
    BadValue(String, std::str::Utf8Error),
}

// Custom deserialization function
fn parse_mime<'de, D>(deserializer: D) -> Result<Option<Mime>, D::Error>
where
    D: Deserializer<'de>,
{
    let mime: Option<String> = Deserialize::deserialize(deserializer)?;
    mime.map(|s| Mime::from_str(&s).map_err(serde::de::Error::custom))
        .transpose()
}

fn parse_mimes<'de, D>(deserializer: D) -> Result<Vec<Mime>, D::Error>
where
    D: Deserializer<'de>,
{
    let mimes: String = Deserialize::deserialize(deserializer)?;
    mimes
        .split(',')
        .map(|s| s.trim())
        .map(|mime| {
            Mime::from_str(mime).map_err(|e| {
                serde::de::Error::custom(format!("Failed to parse MIME type '{mime}': {e}"))
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
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
            RequestHeaders::try_to_owned(&make_header(b"content-type", b"application/json"))
                .unwrap();
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
        let value =
            find_header(&sendable, b"content-type").expect("expected a content-type header");
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
        let value =
            find_header(&sendable, b"cache-control").expect("expected a cache-control header");
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
}
