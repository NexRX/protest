use crate::*;
use derive_more::Constructor;
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use tokio_quiche::http3::driver::OutboundFrameSender;
use tracing::error;

#[derive(Debug, Constructor)]
pub struct TypedRoute<T: FromBody, R> {
    pub method: Method,
    pub path: PathBuf,
    pub handler: RouteHandler<T, R>,
}

impl<T, R, Ctx> Route<Ctx> for TypedRoute<T, R>
where
    T: FromBody + DeserializeOwned + Send,
    serde_json::Error: From<<T as FromBody>::Error>,
    R: Debug + Send,
    Response<R>: ResponseSender,
    Ctx: Debug + Send + Sync,
{
    fn method(&self) -> Method {
        self.method
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn handle<'a>(
        &'a self,
        request: RequestStream,
        send: &'a mut OutboundFrameSender,
        _ctx: &'a Ctx,
    ) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + Send + 'a>> {
        Box::pin(async move {
            let request = match request.into_buffered_typed(0).await {
                Ok(r) => r,
                Err(err) => {
                    error!(?err, "Failed to convert request body");
                    return Response::fallback().send(send).await;
                }
            };
            self.handler.handle(request).await.send(send).await
        })
    }
}
