use serde::de::DeserializeOwned;
use std::fmt::Debug;

pub trait FromBody: Sized + Debug {
    type Error: Debug + Send + 'static;
    fn from_body(bytes: &[u8]) -> Result<Self, Self::Error>;
}

impl<T: Debug + DeserializeOwned> FromBody for T {
    type Error = serde_json::Error;
    fn from_body(bytes: &[u8]) -> Result<Self, Self::Error> {
        serde_json::from_slice(bytes)
    }
}
