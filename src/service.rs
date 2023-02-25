use std::{
    fmt::Debug,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use tower::{util::Oneshot, ServiceExt};

// --- tower-service 0.3 to hyper 1.0 ---

/// Converts a [tower-service 0.3 `Service`] to a [hyper 1.0 `Service`].
///
/// If you have a service that uses [`http::Request`] and [`http::Response`] then you probaby need
/// [`TowerService03HttpServiceAsHyper1HttpService`] instead of this.
///
/// [tower-service 0.3 `Service`]: https://docs.rs/tower-service/latest/tower_service/trait.Service.html
/// [hyper 1.0 `Service`]: https://docs.rs/hyper/1.0.0-rc.3/hyper/service/trait.Service.html
/// [`TowerService03HttpServiceAsHyper1HttpService`]: crate::TowerService03HttpServiceAsHyper1HttpService
#[derive(Clone, Copy, Debug)]
pub struct TowerService03ServiceAsHyper1Service<S>(S);

impl<S> TowerService03ServiceAsHyper1Service<S> {
    /// Create a new `TowerService03ServiceAsHyper1Service`.
    pub fn new(inner: S) -> Self {
        Self(inner)
    }
}

impl<S, R> hyper_1::service::Service<R> for TowerService03ServiceAsHyper1Service<S>
where
    S: tower_service_03::Service<R> + Clone,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = TowerService03ServiceAsHyper1ServiceFuture<S, R>;

    #[inline]
    fn call(&mut self, req: R) -> Self::Future {
        TowerService03ServiceAsHyper1ServiceFuture {
            // have to drive backpressure in the future
            future: self.0.clone().oneshot(req),
        }
    }
}

pin_project! {
    /// Response future for [`TowerService03ServiceAsHyper1Service`].
    pub struct TowerService03ServiceAsHyper1ServiceFuture<S, R>
    where
        S: tower_service_03::Service<R>,
    {
        #[pin]
        future: Oneshot<S, R>,
    }
}

impl<S, R> Future for TowerService03ServiceAsHyper1ServiceFuture<S, R>
where
    S: tower_service_03::Service<R>,
{
    type Output = Result<S::Response, S::Error>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.project().future.poll(cx)
    }
}

// --- hyper 1.0 to tower-service 0.3 ---

/// Converts a [hyper 1.0 `Service`] to a [tower-service 0.3 `Service`].
///
/// If you have a service that uses [`http::Request`] and [`http::Response`] then you probaby need
/// [`Hyper1HttpServiceAsTowerService03HttpService`] instead of this.
///
/// [tower-service 0.3 `Service`]: https://docs.rs/tower-service/latest/tower_service/trait.Service.html
/// [hyper 1.0 `Service`]: https://docs.rs/hyper/1.0.0-rc.3/hyper/service/trait.Service.html
/// [`Hyper1HttpServiceAsTowerService03HttpService`]: crate::Hyper1HttpServiceAsTowerService03HttpService
#[derive(Clone, Copy, Debug)]
pub struct Hyper1ServiceAsTowerService03Service<S>(S);

impl<S> Hyper1ServiceAsTowerService03Service<S> {
    /// Create a new `Hyper1ServiceAsTowerService03Service`.
    #[inline]
    pub fn new(inner: S) -> Self {
        Self(inner)
    }
}

impl<S, R> tower_service_03::Service<R> for Hyper1ServiceAsTowerService03Service<S>
where
    S: hyper_1::service::Service<R>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    #[inline]
    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, req: R) -> Self::Future {
        self.0.call(req)
    }
}
