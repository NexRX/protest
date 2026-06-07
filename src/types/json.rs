use crate::{ByteCounter, FromBody, ProtestError, RequestError, ResponseBody, ResponseError};
use derive_more::{Deref, DerefMut};
use mime::Mime;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::fmt::Debug;
use tokio_quiche::http3::driver::OutboundFrameSender;

#[derive(Debug, Deref, DerefMut, Serialize, Deserialize)]
pub struct Json<T> {
    #[serde(flatten)]
    #[deref]
    #[deref_mut]
    inner: T,
    #[serde(skip)]
    pretty: bool,
}

impl<T: Serialize + Send + 'static> Json<T> {
    /// Contructs a JSON value which is minified on release builds and pretty formatted on debug
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            pretty: cfg!(debug_assertions),
        }
    }

    /// Constructs a JSON value which is always minified
    pub fn minified(inner: T) -> Self {
        Self {
            inner,
            pretty: false,
        }
    }

    /// Constructs a JSON value which is always pretty formatted
    pub fn pretty(inner: T) -> Self {
        Self {
            inner,
            pretty: true,
        }
    }

    pub fn is_pretty(&self) -> bool {
        self.pretty
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T: Debug + DeserializeOwned> FromBody for Json<T> {
    fn from_body(bytes: &[u8]) -> Result<Self, RequestError> {
        serde_json::from_slice(bytes).map_err(RequestError::from)
    }

    fn content_type() -> Option<Mime> {
        Some(mime::APPLICATION_JSON)
    }
}

impl<T: Serialize + Send + 'static> ResponseBody for Json<T> {
    async fn send(self, send: &mut OutboundFrameSender) -> Result<(), ProtestError> {
        serde_json::to_value(&self.inner)
            .map_err(ResponseError::from)?
            .send(send)
            .await
    }

    fn size(&self) -> Option<usize> {
        ByteCounter::serializable(&self.inner)
    }

    fn default_content_type(&self) -> Option<Mime> {
        Some(mime::APPLICATION_JSON)
    }
}

impl<T: Serialize> From<Json<T>> for bytes::Bytes {
    fn from(value: Json<T>) -> Self {
        serde_json::to_vec(&value.inner).unwrap().into()
    }
}

impl<T: Serialize + Send + 'static> From<T> for Json<T> {
    fn from(value: T) -> Self {
        Json::new(value)
    }
}
