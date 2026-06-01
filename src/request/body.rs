use mime::Mime;
use serde_json::Value;
use std::fmt::Debug;

#[derive(Debug, thiserror::Error)]
#[error("Failed to parse body: {message}")]
pub struct FromBodyError {
    message: String,
}

impl From<serde_json::Error> for FromBodyError {
    fn from(err: serde_json::Error) -> Self {
        Self {
            message: err.to_string(),
        }
    }
}

pub trait FromBody: Sized + Debug {
    fn from_body(bytes: &[u8]) -> Result<Self, FromBodyError>;

    fn content_type() -> Option<Mime>;

    fn content_type_name() -> Option<String> {
        Self::content_type().map(|mime| mime.to_string())
    }
}

impl FromBody for Value {
    fn from_body(bytes: &[u8]) -> Result<Self, FromBodyError> {
        serde_json::from_slice(bytes).map_err(FromBodyError::from)
    }

    fn content_type() -> Option<Mime> {
        Some(mime::APPLICATION_JSON)
    }
}

impl FromBody for String {
    fn from_body(bytes: &[u8]) -> Result<Self, FromBodyError> {
        Ok(String::from_utf8_lossy(bytes).to_string())
    }

    fn content_type() -> Option<Mime> {
        Some(mime::TEXT_PLAIN)
    }
}
