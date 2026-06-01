use crate::{ByteCounter, FutureResult, Response};
use bytes::Bytes;
use futures_util::{SinkExt as _, stream::BoxStream};
use mime::Mime;
use tokio_quiche::http3::driver::{OutboundFrame, OutboundFrameSender};
use tokio_stream::StreamExt as _;

pub trait ResponseSender {
    fn send(self, send: &mut OutboundFrameSender) -> FutureResult<'_, ()>;
}

impl<T> ResponseSender for Response<T>
where
    T: ResponseBody,
{
    fn send(mut self, send: &mut OutboundFrameSender) -> FutureResult<'_, ()> {
        Box::pin(async move {
            self.send_headers(send, self.body.default_content_type())
                .await?;
            self.body.send(send).await?;
            Ok(())
        })
    }
}

// ---------- ResponseBody ----------

pub trait ResponseBody: Send + 'static {
    /// Sends the body to the client.
    fn send(self, send: &mut OutboundFrameSender) -> FutureResult<'_, ()>;
    /// Size in bytes of the body.
    fn size(&self) -> Option<usize>;
    /// Default content type of the body. Send in response if not sent manually.
    fn default_content_type(&self) -> Option<Mime>;
}

impl ResponseBody for BoxStream<'static, Bytes> {
    fn send(self, send: &mut OutboundFrameSender) -> FutureResult<'_, ()> {
        Box::pin(async move {
            let mut peekable = self.peekable();
            while let Some(chunk) = peekable.next().await {
                let fin = peekable.peek().await.is_none();
                send.send(OutboundFrame::Body(chunk, fin)).await?;
            }
            Ok(())
        })
    }

    fn size(&self) -> Option<usize> {
        None
    }

    fn default_content_type(&self) -> Option<Mime> {
        Some(mime::APPLICATION_OCTET_STREAM)
    }
}

impl ResponseBody for Vec<u8> {
    fn send(self, send: &mut OutboundFrameSender) -> FutureResult<'_, ()> {
        Box::pin(async move {
            let body = serde_json::to_string(&self)
                .map_err(|err| format!("Failed to serialize JSON body: {err}"))?;
            send.send(OutboundFrame::Body(body.into(), true)).await?;
            Ok(())
        })
    }

    fn size(&self) -> Option<usize> {
        Some(self.len())
    }

    fn default_content_type(&self) -> Option<Mime> {
        Some(mime::APPLICATION_OCTET_STREAM)
    }
}

impl ResponseBody for serde_json::Value {
    fn send(self, send: &mut OutboundFrameSender) -> FutureResult<'_, ()> {
        Box::pin(async move {
            let body = serde_json::to_vec(&self)
                .map_err(|err| format!("Failed to serialize JSON body: {err}"))?;
            send.send(OutboundFrame::Body(body.into(), true)).await?;
            Ok(())
        })
    }

    fn size(&self) -> Option<usize> {
        ByteCounter::serializable(self)
    }

    fn default_content_type(&self) -> Option<Mime> {
        Some(mime::APPLICATION_JSON)
    }
}

impl ResponseBody for String {
    fn send(self, send: &mut OutboundFrameSender) -> FutureResult<'_, ()> {
        Box::pin(async move {
            send.send(OutboundFrame::Body(self.into(), true)).await?;
            Ok(())
        })
    }

    fn size(&self) -> Option<usize> {
        Some(self.len())
    }

    fn default_content_type(&self) -> Option<Mime> {
        Some(mime::TEXT_PLAIN_UTF_8)
    }
}

impl ResponseBody for &'static str {
    fn send(self, send: &mut OutboundFrameSender) -> FutureResult<'_, ()> {
        Box::pin(async move {
            send.send(OutboundFrame::Body(self.into(), true)).await?;
            Ok(())
        })
    }

    fn size(&self) -> Option<usize> {
        Some(self.len())
    }

    fn default_content_type(&self) -> Option<Mime> {
        Some(mime::TEXT_PLAIN_UTF_8)
    }
}

impl ResponseBody for () {
    fn send(self, send: &mut OutboundFrameSender) -> FutureResult<'_, ()> {
        Box::pin(async move {
            send.send(OutboundFrame::Body("".into(), true)).await?;
            Ok(())
        })
    }

    fn size(&self) -> Option<usize> {
        None
    }

    fn default_content_type(&self) -> Option<Mime> {
        Some(mime::APPLICATION_OCTET_STREAM)
    }
}

impl<T: ResponseBody, E: ResponseBody> ResponseBody for Result<T, E> {
    fn send(self, send: &mut OutboundFrameSender) -> FutureResult<'_, ()> {
        Box::pin(async move {
            match self {
                Ok(ok) => ok.send(send).await,
                Err(err) => err.send(send).await,
            }
        })
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
