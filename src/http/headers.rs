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
