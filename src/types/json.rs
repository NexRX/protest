use crate::{ByteCounter, FromBody, FromBodyError, FutureResult, ResponseBody};
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
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            pretty: false,
        }
    }

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
    fn from_body(bytes: &[u8]) -> Result<Self, FromBodyError> {
        serde_json::from_slice(bytes).map_err(FromBodyError::from)
    }

    fn content_type() -> Option<Mime> {
        Some(mime::APPLICATION_JSON)
    }
}

impl<T: Serialize + Send + 'static> ResponseBody for Json<T> {
    fn send(self, send: &mut OutboundFrameSender) -> FutureResult<'_, ()> {
        Box::pin(async move { serde_json::to_value(&self.inner)?.send(send).await })
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
