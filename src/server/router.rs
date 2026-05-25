use crate::{FromBody, Method, Request, RequestStream, Response, ResponseSender};
use derive_more::Constructor;
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use tokio_quiche::http3::driver::OutboundFrameSender;
use tracing::{error, warn};

pub type AsyncRequestHandlerResult<'a> =
    Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + Send + 'a>>;

pub trait Route: Debug + Send + Sync {
    fn method(&self) -> Method;
    fn path(&self) -> &Path;
    fn handle<'a>(
        &'a self,
        request: RequestStream,
        send: &'a mut OutboundFrameSender,
    ) -> AsyncRequestHandlerResult<'a>;

    fn is_match(&self, request: &RequestStream) -> bool {
        self.method() == request.method && self.path() == request.path
    }
}

pub type SyncHandler<T, R> = fn(Request<T>) -> Response<R>;
pub type AsyncHandler<T, R> =
    Box<dyn Fn(Request<T>) -> Pin<Box<dyn Future<Output = Response<R>> + Send>> + Send + Sync>;

#[derive(Debug, Constructor)]
pub struct TypedRoute<T: FromBody, R> {
    pub method: Method,
    pub path: PathBuf,
    pub handler: RouteHandler<T, R>,
}

impl<T, R> Route for TypedRoute<T, R>
where
    T: FromBody + DeserializeOwned + Send,
    serde_json::Error: From<<T as FromBody>::Error>,
    R: Debug + Send,
    Response<R>: ResponseSender,
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
    ) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + Send + 'a>> {
        Box::pin(async move {
            let request = match request.into_typed_with_json(0).await {
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

pub enum RouteHandler<T: Debug, R> {
    Sync(SyncHandler<T, R>),
    Async(AsyncHandler<T, R>),
}

impl<T: Debug, R> From<SyncHandler<T, R>> for RouteHandler<T, R> {
    fn from(handler: SyncHandler<T, R>) -> Self {
        Self::Sync(handler)
    }
}

impl<T, R, F, Fut> From<F> for RouteHandler<T, R>
where
    T: FromBody + DeserializeOwned + Send + 'static,
    serde_json::Error: From<<T as FromBody>::Error>,
    R: Debug + Send + 'static,
    Response<R>: ResponseSender,
    F: Fn(Request<T>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Response<R>> + Send + 'static,
{
    fn from(f: F) -> Self {
        Self::Async(Box::new(move |req| Box::pin(f(req))))
    }
}

impl<T: Debug, R> Debug for RouteHandler<T, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RouteHandler::Sync(h) => write!(f, "Sync({:?})", h as *const _),
            RouteHandler::Async(_) => write!(f, "Async(<handler>)"),
        }
    }
}

impl<T: Debug, R> RouteHandler<T, R> {
    pub async fn handle(&self, request: Request<T>) -> Response<R> {
        match self {
            RouteHandler::Sync(handler) => handler(request),
            RouteHandler::Async(handler) => handler(request).await,
        }
    }
}

#[derive(Debug, Default)]
pub struct Router(Vec<Box<dyn Route>>);

impl Router {
    pub fn add_route<T, R>(&mut self, route: TypedRoute<T, R>)
    where
        T: FromBody + DeserializeOwned + Send + 'static,
        serde_json::Error: From<<T as FromBody>::Error>,
        R: Debug + Send + 'static,
        Response<R>: ResponseSender,
    {
        self.0.push(Box::new(route));
    }

    pub fn nest(&mut self, router: Self) {
        self.0.extend(router.0);
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    pub fn find_route(&self, request: &RequestStream) -> Option<&dyn Route> {
        if self.0.is_empty() {
            warn!("No routes in router, impossible to find a route");
            return None;
        }

        self.0
            .iter()
            .find(|route| route.is_match(request))
            .map(Box::as_ref)
    }

    pub(crate) async fn handle_request(
        &self,
        request: RequestStream,
        mut send: OutboundFrameSender,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match self.find_route(&request) {
            Some(route) => route.handle(request, &mut send).await,
            None => Response::route_not_found().send(&mut send).await,
        }
    }
}
