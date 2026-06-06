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
    /// The GET method requests a representation of the specified resource. Requests using GET should only retrieve data and should not contain a request content.
    GET,
    /// The POST method submits an entity to the specified resource, often causing a change in state or side effects on the server.
    POST,
    /// The PUT method replaces all current representations of the target resource with the request content.
    PUT,
    /// The DELETE method deletes the specified resource.
    DELETE,
    /// The PATCH method applies partial modifications to a resource.
    PATCH,
    /// The OPTIONS method describes the communication options for the target resource.
    OPTIONS,
    /// The HEAD method asks for a response identical to a GET request, but without a response body.
    HEAD,
    /// The CONNECT method establishes a tunnel to the server identified by the target resource.
    CONNECT,
    /// The TRACE method performs a message loop-back test along the path to the target resource.
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
