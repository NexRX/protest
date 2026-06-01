mod handlers;
#[cfg(test)]
mod mod_test;


use crate::error::ServerError;
use crate::{FutureResult, RequestStream};
use std::fmt::Debug;
use tokio_quiche::http3::driver::OutboundFrameSender;

pub trait TRouter: Debug + Send + Sync + 'static {
    fn can_handle_request(&self, request: &RequestStream) -> bool;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn handle_request<'a>(
        &'a self,
        request: RequestStream,
        send: &'a mut OutboundFrameSender,
    ) -> FutureResult<'a, (), ServerError>;
}
