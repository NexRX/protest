use crate::RequestBuilderError;

#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("Router couldn't find any matching handlers")]
    NoRoute,
    #[error("{0}")]
    Request(RequestBuilderError),
    #[error("{0}")]
    Generic(Box<dyn std::error::Error>),
}

impl From<RequestBuilderError> for ServerError {
    fn from(value: RequestBuilderError) -> Self {
        Self::Request(value)
    }
}

impl From<Box<dyn std::error::Error>> for ServerError {
    fn from(value: Box<dyn std::error::Error>) -> Self {
        Self::Generic(value)
    }
}

impl From<&'static str> for ServerError {
    fn from(value: &'static str) -> Self {
        Self::Generic(value.into())
    }
}
