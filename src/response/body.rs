use crate::{ByteCounter, ProtestError};
use bytes::Bytes;
use futures_util::{SinkExt, stream::BoxStream};
use mime::Mime;
use std::future::Future;
use tokio_quiche::http3::driver::{OutboundFrame, OutboundFrameSender};
use tokio_stream::StreamExt as _;

pub trait ResponseBody: Send + 'static {
    /// Sends the body to the client.
    fn send(
        self,
        send: &mut OutboundFrameSender,
    ) -> impl Future<Output = Result<(), ProtestError>> + Send;
    /// Size in bytes of the body.
    fn size(&self) -> Option<usize>;
    /// Default content type of the body. Send in response if not sent manually.
    fn default_content_type(&self) -> Option<Mime>;

    fn is_empty(&self) -> bool {
        self.size().is_none_or(|s| s == 0)
    }
}

impl ResponseBody for BoxStream<'static, Bytes> {
    async fn send(self, send: &mut OutboundFrameSender) -> Result<(), ProtestError> {
        let mut peekable = self.peekable();
        while let Some(chunk) = peekable.next().await {
            let fin = peekable.peek().await.is_none();
            send.send(OutboundFrame::Body(chunk, fin)).await?
        }
        Ok(())
    }

    fn size(&self) -> Option<usize> {
        None
    }

    fn default_content_type(&self) -> Option<Mime> {
        Some(mime::APPLICATION_OCTET_STREAM)
    }
}

impl ResponseBody for Vec<u8> {
    async fn send(self, send: &mut OutboundFrameSender) -> Result<(), ProtestError> {
        send.send(OutboundFrame::Body(self.into(), true)).await?;
        Ok(())
    }

    fn size(&self) -> Option<usize> {
        Some(self.len())
    }

    fn default_content_type(&self) -> Option<Mime> {
        Some(mime::APPLICATION_OCTET_STREAM)
    }
}

impl ResponseBody for serde_json::Value {
    async fn send(self, send: &mut OutboundFrameSender) -> Result<(), ProtestError> {
        let body = serde_json::to_vec(&self)?;
        send.send(OutboundFrame::Body(body.into(), true)).await?;
        Ok(())
    }

    fn size(&self) -> Option<usize> {
        ByteCounter::serializable(self)
    }

    fn default_content_type(&self) -> Option<Mime> {
        Some(mime::APPLICATION_JSON)
    }
}

impl ResponseBody for String {
    async fn send(self, send: &mut OutboundFrameSender) -> Result<(), ProtestError> {
        send.send(OutboundFrame::Body(self.into(), true)).await?;
        Ok(())
    }

    fn size(&self) -> Option<usize> {
        Some(self.len())
    }

    fn default_content_type(&self) -> Option<Mime> {
        Some(mime::TEXT_PLAIN_UTF_8)
    }
}

impl ResponseBody for &'static str {
    async fn send(self, send: &mut OutboundFrameSender) -> Result<(), ProtestError> {
        send.send(OutboundFrame::Body(self.into(), true)).await?;
        Ok(())
    }

    fn size(&self) -> Option<usize> {
        Some(self.len())
    }

    fn default_content_type(&self) -> Option<Mime> {
        Some(mime::TEXT_PLAIN_UTF_8)
    }
}

impl ResponseBody for () {
    async fn send(self, send: &mut OutboundFrameSender) -> Result<(), ProtestError> {
        send.send(OutboundFrame::Body("".into(), true)).await?;
        Ok(())
    }

    fn size(&self) -> Option<usize> {
        None
    }

    fn default_content_type(&self) -> Option<Mime> {
        Some(mime::APPLICATION_OCTET_STREAM)
    }
}

impl<T: ResponseBody, E: ResponseBody> ResponseBody for Result<T, E> {
    async fn send(self, send: &mut OutboundFrameSender) -> Result<(), ProtestError> {
        match self {
            Ok(ok) => ok.send(send).await,
            Err(err) => err.send(send).await,
        }
    }

    fn size(&self) -> Option<usize> {
        match self {
            Ok(ok) => ok.size(),
            Err(err) => err.size(),
        }
    }

    fn default_content_type(&self) -> Option<Mime> {
        match self {
            Ok(ok) => ok.default_content_type(),
            Err(err) => err.default_content_type(),
        }
    }
}
macro_rules! impl_for_to_string {
    ($self:ty) => {
        impl ResponseBody for $self {
            async fn send(self, send: &mut OutboundFrameSender) -> Result<(), ProtestError> {
                send.send(OutboundFrame::Body(self.to_string().into(), true))
                    .await?;
                Ok(())
            }

            fn size(&self) -> Option<usize> {
                Some(self.to_string().len())
            }

            fn default_content_type(&self) -> Option<Mime> {
                Some(mime::TEXT_PLAIN_UTF_8)
            }
        }
    };
}

impl_for_to_string!(usize);
impl_for_to_string!(u16);
impl_for_to_string!(u32);
impl_for_to_string!(u64);
impl_for_to_string!(u128);
impl_for_to_string!(i8);
impl_for_to_string!(i16);
impl_for_to_string!(i32);
impl_for_to_string!(i64);
impl_for_to_string!(i128);
