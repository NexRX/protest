use tokio_quiche::http3::driver::OutboundFrame;
use tokio_util::sync::PollSendError;

use crate::{RequestBuilderError, RequestParamKind};

#[derive(Debug, thiserror::Error)]
pub enum ProtestError {
    #[error("Router couldn't find any matching handlers")]
    NoRoute,
    #[error("{0}")]
    Response(#[from] ResponseError),
    #[error("{0}")]
    Request(#[from] RequestError),
    #[error("Unhandled/unexpected error: {0}")]
    Generic(String),
}

#[derive(Debug, thiserror::Error)]
pub enum RequestError {
    #[error("{0}")]
    BuildFromHttp(#[from] RequestBuilderError),
    #[error("Failed to deserialize while processing the request: {0}")]
    Deserialize(#[from] serde_json::error::Error),
    #[error(
        "Failed to convert parameter '{kind}' named '{name}' to {conversion_type:?} for value {raw_value:?}: {message}"
    )]
    Invalid {
        /// The name associated with the parameter that caused the error.
        name: String,
        /// The kind of parameter that caused the error.
        kind: RequestParamKind,
        /// If a raw value was provided, this is the value that caused the error.
        raw_value: Option<String>,
        /// If the error is caused by a conversion error, this is the stringified type that was expected.
        conversion_type: Option<String>,
        /// The error message, ideally only for internal usage/logging
        message: String,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum ResponseError {
    #[error("Failed to deserialize while processing the request: {0}")]
    Serialize(#[from] serde_json::error::Error),
    #[error("Failed to send response object: {0}")]
    Send(#[from] PollSendError<OutboundFrame>),
}

impl From<&'static str> for ProtestError {
    fn from(value: &'static str) -> Self {
        Self::Generic(value.into())
    }
}

impl From<Box<dyn std::error::Error>> for ProtestError {
    fn from(value: Box<dyn std::error::Error>) -> Self {
        Self::Generic(value.to_string())
    }
}

impl From<RequestBuilderError> for ProtestError {
    fn from(value: RequestBuilderError) -> Self {
        Self::Request(RequestError::BuildFromHttp(value))
    }
}

impl From<PollSendError<OutboundFrame>> for ProtestError {
    fn from(value: PollSendError<OutboundFrame>) -> Self {
        Self::Response(ResponseError::Send(value))
    }
}
