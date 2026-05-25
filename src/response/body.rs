use crate::{AsyncRequestHandlerResult, Response};
use bytes::Bytes;
use derive_more::{Constructor, Deref, DerefMut};
use futures_util::{SinkExt as _, stream::BoxStream};
use mime::Mime;
use serde::Serialize;
use std::future::Future;
use std::pin::Pin;
use tokio_quiche::http3::driver::{OutboundFrame, OutboundFrameSender};
use tokio_stream::StreamExt as _;

// ---------- ResponseSender ----------

pub trait ResponseSender {
    fn send(self, send: &mut OutboundFrameSender) -> AsyncRequestHandlerResult<'_>;
}

impl<T> ResponseSender for Response<T>
where
    T: ResponseBody,
{
    fn send(mut self, send: &mut OutboundFrameSender) -> AsyncRequestHandlerResult<'_> {
        Box::pin(async move {
            self.send_headers(send, T::default_content_type()).await?;
            self.body.send(send).await?;
            Ok(())
        })
    }
}

// ---------- ResponseBody ----------

pub trait ResponseBody: Send + 'static {
    fn send(self, send: &mut OutboundFrameSender) -> AsyncRequestHandlerResult<'_>;
    fn default_content_type() -> Mime;
}

impl ResponseBody for BoxStream<'static, Bytes> {
    fn send(
        self,
        send: &mut OutboundFrameSender,
    ) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + Send + '_>> {
        Box::pin(async move {
            let mut peekable = self.peekable();
            while let Some(chunk) = peekable.next().await {
                let fin = peekable.peek().await.is_none();
                send.send(OutboundFrame::Body(chunk, fin)).await?;
            }
            Ok(())
        })
    }

    fn default_content_type() -> Mime {
        mime::APPLICATION_OCTET_STREAM
    }
}

impl ResponseBody for serde_json::Value {
    fn send(
        self,
        send: &mut OutboundFrameSender,
    ) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + Send + '_>> {
        Box::pin(async move {
            let body = serde_json::to_string(&self)
                .map_err(|err| format!("Failed to serialize JSON body: {err}"))?;
            send.send(OutboundFrame::Body(body.into(), true)).await?;
            Ok(())
        })
    }

    fn default_content_type() -> Mime {
        mime::APPLICATION_JSON
    }
}

#[derive(Constructor, Deref, DerefMut)]
pub struct Json<T: Serialize + Send + 'static>(pub T);

impl<T: Serialize + Send + 'static> ResponseBody for Json<T> {
    fn send(
        self,
        send: &mut OutboundFrameSender,
    ) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + Send + '_>> {
        Box::pin(async move { serde_json::to_value(&self.0)?.send(send).await })
    }

    fn default_content_type() -> Mime {
        serde_json::Value::default_content_type()
    }
}

impl ResponseBody for String {
    fn send(
        self,
        send: &mut OutboundFrameSender,
    ) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + Send + '_>> {
        Box::pin(async move {
            let body = serde_json::to_string(&self)
                .map_err(|err| format!("Failed to serialize JSON body: {err}"))?;
            send.send(OutboundFrame::Body(body.into(), true)).await?;
            Ok(())
        })
    }

    fn default_content_type() -> Mime {
        mime::TEXT_PLAIN_UTF_8
    }
}

// Impl for some useful types, File (tokio and std), PathBuf, Vec<u8>, Bytes, etc. can be added as needed.
