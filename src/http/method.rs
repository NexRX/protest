use crate::ParseHeaderError;
use derive_more::Eq;
use quiche::h3::{self, NameValue};

#[derive(Debug, Clone, Copy, Eq, PartialEq, serde::Deserialize)]
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

impl TryFrom<&h3::Header> for Method {
    type Error = ParseHeaderError;

    fn try_from(value: &h3::Header) -> Result<Self, Self::Error> {
        let method_str = std::str::from_utf8(value.value())
            .map_err(|e| ParseHeaderError::BadValue("Method".to_string(), e))?;

        match method_str {
            "GET" => Ok(Method::GET),
            "POST" => Ok(Method::POST),
            "PUT" => Ok(Method::PUT),
            "DELETE" => Ok(Method::DELETE),
            "PATCH" => Ok(Method::PATCH),
            "OPTIONS" => Ok(Method::OPTIONS),
            "HEAD" => Ok(Method::HEAD),
            "CONNECT" => Ok(Method::CONNECT),
            "TRACE" => Ok(Method::TRACE),
            _ => Err(ParseHeaderError::Unexpected(
                "Method".to_string(),
                method_str.to_string(),
            )),
        }
    }
}
