mod handlers;
#[cfg(test)]
mod mod_test;
mod typed_route;
mod typed_route_with_ctx;

pub use handlers::*;
pub use typed_route::*;
pub use typed_route_with_ctx::*;

use crate::error::ServerError;
use crate::{FutureResult, Method, RequestStream, Response, ResponseSender};
use std::fmt::Debug;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use tokio_quiche::http3::driver::OutboundFrameSender;
use tracing::warn;

pub type AsyncRequestHandlerResult<'a> =
    Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + Send + 'a>>;

pub trait Route<Ctx = ()>: Debug + Send + Sync {
    fn method(&self) -> Method;
    fn path(&self) -> &Path;
    fn handle<'a>(
        &'a self,
        request: RequestStream,
        send: &'a mut OutboundFrameSender,
        ctx: &'a Ctx,
    ) -> AsyncRequestHandlerResult<'a>;

    fn is_match(&self, request: &RequestStream) -> bool {
        self.method() == request.method && self.path() == request.path
    }
}

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

#[derive(Debug, Default)]
pub struct Router<Ctx = ()>(Vec<Box<dyn Route<Ctx>>>, Ctx);

impl<Ctx> Router<Ctx> {
    pub fn new(ctx: Ctx) -> Self {
        Self(Vec::new(), ctx)
    }

    pub fn add<T>(&mut self, route: T)
    where
        T: Route<Ctx> + Send + Sync + 'static,
    {
        self.0.push(Box::new(route));
    }

    pub fn extend(&mut self, router: Self) {
        self.0.extend(router.0);
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    pub fn find_route(&self, request: &RequestStream) -> Option<&dyn Route<Ctx>> {
        if self.0.is_empty() {
            warn!("No routes in router, impossible to find a route");
            return None;
        }

        self.0
            .iter()
            .find(|route| route.is_match(request))
            .map(Box::as_ref)
    }
}

impl<Ctx: Debug + Send + Sync + 'static> TRouter for Router<Ctx> {
    fn handle_request<'a>(
        &'a self,
        request: RequestStream,
        send: &'a mut OutboundFrameSender,
    ) -> FutureResult<'a, (), ServerError> {
        Box::pin(async move {
            let _: () = match self.find_route(&request) {
                Some(route) => route.handle(request, send, &self.1).await?,
                None => Response::route_not_found().send(send).await?,
            };
            Ok(())
        })
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn can_handle_request(&self, request: &RequestStream) -> bool {
        self.find_route(request).is_some()
    }
}
