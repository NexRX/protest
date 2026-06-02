use crate::ProtestError;
use mime::Mime;
use serde_json::Value;
use std::fmt::Debug;

pub trait FromBody: Sized + Debug {
    fn from_body(bytes: &[u8]) -> Result<Self, ProtestError>;

    fn content_type() -> Option<Mime>;

    fn content_type_name() -> Option<String> {
        Self::content_type().map(|mime| mime.to_string())
    }
}

impl FromBody for Value {
    fn from_body(bytes: &[u8]) -> Result<Self, ProtestError> {
        serde_json::from_slice(bytes).map_err(ProtestError::from)
    }

    fn content_type() -> Option<Mime> {
        Some(mime::APPLICATION_JSON)
    }
}

impl FromBody for String {
    fn from_body(bytes: &[u8]) -> Result<Self, ProtestError> {
        Ok(String::from_utf8_lossy(bytes).to_string())
    }

    fn content_type() -> Option<Mime> {
        Some(mime::TEXT_PLAIN)
    }
}
