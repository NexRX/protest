use tokio_quiche::http3::driver::OutboundFrame;
use tokio_util::sync::PollSendError;

use crate::RequestBuilderError;

#[derive(Debug, thiserror::Error)]
pub enum ProtestError {
    #[error("Router couldn't find any matching handlers")]
    NoRoute,
    #[error("{0}")]
    Request(RequestBuilderError),
    #[error("Failed to (de)serialize while processing the request: {0}")]
    Serde(serde_json::error::Error),
    #[error("Failed to send response object: {0}")]
    Response(PollSendError<OutboundFrame>),
    #[error("{0}")]
    Generic(Box<dyn std::error::Error>),
}

impl From<RequestBuilderError> for ProtestError {
    fn from(value: RequestBuilderError) -> Self {
        Self::Request(value)
    }
}

impl From<Box<dyn std::error::Error>> for ProtestError {
    fn from(value: Box<dyn std::error::Error>) -> Self {
        Self::Generic(value)
    }
}

impl From<&'static str> for ProtestError {
    fn from(value: &'static str) -> Self {
        Self::Generic(value.into())
    }
}

impl From<serde_json::error::Error> for ProtestError {
    fn from(value: serde_json::error::Error) -> Self {
        Self::Serde(value)
    }
}

impl From<PollSendError<OutboundFrame>> for ProtestError {
    fn from(value: PollSendError<OutboundFrame>) -> Self {
        Self::Response(value)
    }
}
