use crate::ParseHeaderError;
use derive_more::Eq;
use quiche::h3::{self, NameValue};

#[derive(
    Debug,
    Clone,
    Copy,
    Eq,
    PartialEq,
    serde::Deserialize,
    strum::Display,
    strum::AsRefStr,
    strum::EnumString,
)]
#[strum(serialize_all = "UPPERCASE")]
pub enum Method {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    OPTIONS,
    HEAD,
    CONNECT,
    TRACE,
}

impl Method {
    pub fn as_bytes(&self) -> &[u8] {
        self.as_ref().as_bytes()
    }
}

impl TryFrom<&h3::Header> for Method {
    type Error = ParseHeaderError;

    fn try_from(value: &h3::Header) -> Result<Self, Self::Error> {
        let method_str = std::str::from_utf8(value.value())
            .map_err(|e| ParseHeaderError::BadValue("Method".to_string(), e))?;

        Self::try_from(method_str)
            .map_err(|e| ParseHeaderError::Unexpected("Method".to_string(), e.to_string()))
    }
}
