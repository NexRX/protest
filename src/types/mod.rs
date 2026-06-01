mod json;

pub use json::*;
use serde::Serialize;

use std::{io, pin::Pin};

pub type AnyError = Box<dyn std::error::Error>;

pub type FutureT<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub type FutureResult<'a, T, E = AnyError> = FutureT<'a, Result<T, E>>;

// ---------- Utilities ----------
/// A no-alloc writer that only counts bytes written to it.
pub(crate) struct ByteCounter(usize);

impl ByteCounter {
    /// Returns the number of bytes that would be written if the value were serialized to JSON.
    pub fn serializable(value: &impl Serialize) -> Option<usize> {
        let mut counter = ByteCounter(0);
        serde_json::to_writer(&mut counter, value).ok()?;
        Some(counter.0)
    }
}

impl io::Write for ByteCounter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0 += buf.len();
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
