use std::pin::Pin;

pub type AnyError = Box<dyn std::error::Error>;

pub type FutureT<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub type FutureResult<'a, T, E = AnyError> = FutureT<'a, Result<T, E>>;
