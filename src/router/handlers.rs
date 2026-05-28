use serde::de::DeserializeOwned;

use crate::*;
use std::fmt::Debug;
use std::pin::Pin;

pub type SyncHandler<T, R> = fn(Request<T>) -> Response<R>;
pub type SyncWithCtxHandler<T, R, Ctx> = fn(Request<T>, &Ctx) -> Response<R>;

pub type AsyncHandler<T, R> =
    Box<dyn Fn(Request<T>) -> Pin<Box<dyn Future<Output = Response<R>> + Send>> + Send + Sync>;
pub type AsyncWithCtxHandler<T, R, Ctx> = Box<
    dyn for<'a> Fn(Request<T>, &'a Ctx) -> Pin<Box<dyn Future<Output = Response<R>> + Send + 'a>>
        + Send
        + Sync,
>;

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
            Self::Sync(h) => write!(f, "Sync({:?})", h as *const _),
            Self::Async(_) => write!(f, "Async(<handler>)"),
        }
    }
}

impl<T: Debug, R> RouteHandler<T, R> {
    pub async fn handle(&self, request: Request<T>) -> Response<R> {
        match self {
            Self::Sync(handler) => handler(request),
            Self::Async(handler) => handler(request).await,
        }
    }
}

pub enum RouteWithCtxHandler<T: Debug, R, Ctx> {
    Sync(SyncWithCtxHandler<T, R, Ctx>),
    Async(AsyncWithCtxHandler<T, R, Ctx>),
}

impl<T: Debug, R, Ctx> From<SyncWithCtxHandler<T, R, Ctx>> for RouteWithCtxHandler<T, R, Ctx> {
    fn from(handler: SyncWithCtxHandler<T, R, Ctx>) -> Self {
        Self::Sync(handler)
    }
}

impl<T, R, F, Fut, Ctx> From<F> for RouteWithCtxHandler<T, R, Ctx>
where
    T: FromBody + DeserializeOwned + Send + 'static,
    serde_json::Error: From<<T as FromBody>::Error>,
    R: Debug + Send + 'static,
    Response<R>: ResponseSender,
    F: Fn(Request<T>, &Ctx) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Response<R>> + Send + 'static,
{
    fn from(f: F) -> Self {
        Self::Async(Box::new(
            move |req, ctx| -> Pin<Box<dyn Future<Output = Response<R>> + Send + '_>> {
                Box::pin(f(req, ctx))
            },
        ))
    }
}

impl<T: Debug, R, Ctx: Debug> Debug for RouteWithCtxHandler<T, R, Ctx> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sync(h) => write!(f, "SyncWithCtx({:?})", h as *const _),
            Self::Async(_) => write!(f, "AsyncWithCtx(<handler>)"),
        }
    }
}

impl<T: Debug, R, Ctx> RouteWithCtxHandler<T, R, Ctx> {
    pub async fn handle(&self, request: Request<T>, ctx: &Ctx) -> Response<R> {
        match self {
            Self::Sync(handler) => handler(request, ctx),
            Self::Async(handler) => handler(request, ctx).await,
        }
    }
}
