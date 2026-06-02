use crate::{ProtestError, Response, ResponseBody};
use std::future::Future;
use tokio_quiche::http3::driver::OutboundFrameSender;

pub trait ResponseSender {
    fn send(
        self,
        send: &mut OutboundFrameSender,
    ) -> impl Future<Output = Result<(), ProtestError>> + Send;
}

impl<T> ResponseSender for Response<T>
where
    T: ResponseBody,
{
    #[allow(clippy::manual_async_fn)] // Reason: manually produces comp errors with reference values
    fn send(
        mut self,
        send: &mut OutboundFrameSender,
    ) -> impl Future<Output = Result<(), ProtestError>> + Send {
        async move {
            let content_type = self.body.default_content_type();
            self.send_headers(send, content_type).await?;
            self.body.send(send).await?;
            Ok(())
        }
    }
}
