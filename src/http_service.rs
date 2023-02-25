use std::{
    fmt::Debug,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{self, Context, Poll},
};

use http::{Request, Response};
use pin_project_lite::pin_project;
use tower::{util::Oneshot, ServiceExt};

use crate::{HttpBody04ToHttpBody1, HttpBody1ToHttpBody04};

// --- tower-service 0.3 (http) to hyper 1.0 (http) ---

/// Converts a [tower-service 0.3 HTTP `Service`] to a [hyper 1.0 HTTP `Service`].
///
/// An HTTP `Service` is a `Service` where the request is [`http::Request<_>`][Request] and the
/// response is [`http::Response<_>`][Response].
///
/// # Example
///
/// ```no_run
/// use http::{Request, Response, StatusCode};
/// use hyper_1::{server::conn::http1, service::service_fn, body, body::Bytes};
/// use std::{net::SocketAddr, convert::Infallible};
/// use tokio::net::TcpListener;
/// use tower_hyper_http_body_compat::TowerService03HttpServiceAsHyper1HttpService;
///
/// // a service function that uses hyper 0.14, tower-service 0.3, and http-body 0.4
/// async fn handler<B>(req: Request<B>) -> Result<Response<hyper_014::body::Body>, Infallible>
/// where
///     B: hyper_014::body::HttpBody<Data = hyper_014::body::Bytes>,
///     B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
/// {
///    let body = req.into_body();
///    let body = http_body_04::Limited::new(body, 1024);
///    let bytes = match hyper_014::body::to_bytes(body).await {
///        Ok(bytes) => bytes,
///        Err(err) => {
///            let res = Response::builder()
///                .status(StatusCode::BAD_REQUEST)
///                .body(hyper_014::body::Body::empty())
///                .unwrap();
///            return Ok(res)
///        }
///    };
///
///    let res = Response::builder()
///        .body(hyper_014::body::Body::from(format!("Received {} bytes", bytes.len())))
///        .unwrap();
///    Ok(res)
/// }
///
/// // run `handler` on hyper 1.0
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
///     let addr: SocketAddr = ([127, 0, 0, 1], 8080).into();
///
///     let service = tower::service_fn(handler);
///     let service = TowerService03HttpServiceAsHyper1HttpService::new(service);
///
///     let mut tcp_listener = TcpListener::bind(addr).await?;
///     loop {
///         let (tcp_stream, _) = tcp_listener.accept().await?;
///         let service = service.clone();
///         tokio::task::spawn(async move {
///             if let Err(http_err) = http1::Builder::new()
///                     .keep_alive(true)
///                     .serve_connection(tcp_stream, service)
///                     .await {
///                 eprintln!("Error while serving HTTP connection: {}", http_err);
///             }
///         });
///     }
/// }
/// ```
///
/// [tower-service 0.3 HTTP `Service`]: https://docs.rs/tower-service/latest/tower_service/trait.Service.html
/// [hyper 1.0 HTTP `Service`]: https://docs.rs/hyper/1.0.0-rc.3/hyper/service/trait.Service.html
pub struct TowerService03HttpServiceAsHyper1HttpService<S, B> {
    service: S,
    _marker: PhantomData<fn() -> B>,
}

impl<S, B> TowerService03HttpServiceAsHyper1HttpService<S, B> {
    /// Create a new `TowerService03HttpServiceAsHyper1HttpService`.
    #[inline]
    pub fn new(service: S) -> Self {
        Self {
            service,
            _marker: PhantomData,
        }
    }
}

impl<S, B> Copy for TowerService03HttpServiceAsHyper1HttpService<S, B> where S: Copy {}

impl<S, B> Clone for TowerService03HttpServiceAsHyper1HttpService<S, B>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            service: self.service.clone(),
            _marker: self._marker,
        }
    }
}

impl<S, B> Debug for TowerService03HttpServiceAsHyper1HttpService<S, B>
where
    S: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TowerService03HttpServiceAsHyper1HttpService")
            .field("service", &self.service)
            .finish()
    }
}

impl<S, ReqBody, ResBody> hyper_1::service::Service<Request<ReqBody>>
    for TowerService03HttpServiceAsHyper1HttpService<S, HttpBody1ToHttpBody04<ReqBody>>
where
    S: tower_service_03::Service<
            Request<HttpBody1ToHttpBody04<ReqBody>>,
            Response = Response<ResBody>,
        > + Clone,
{
    type Response = Response<HttpBody04ToHttpBody1<ResBody>>;
    type Error = S::Error;
    type Future = TowerService03HttpServiceAsHyper1HttpServiceFuture<
        S,
        Request<HttpBody1ToHttpBody04<ReqBody>>,
    >;

    #[inline]
    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let req = req.map(HttpBody1ToHttpBody04::new);
        TowerService03HttpServiceAsHyper1HttpServiceFuture {
            future: self.service.clone().oneshot(req),
        }
    }
}

pin_project! {
    /// Response future for [`TowerService03HttpServiceAsHyper1HttpService`].
    pub struct TowerService03HttpServiceAsHyper1HttpServiceFuture<S, R>
    where
        S: tower_service_03::Service<R>,
    {
        #[pin]
        future: Oneshot<S, R>,
    }
}

impl<S, R, B> Future for TowerService03HttpServiceAsHyper1HttpServiceFuture<S, R>
where
    S: tower_service_03::Service<R, Response = Response<B>>,
{
    type Output = Result<Response<HttpBody04ToHttpBody1<B>>, S::Error>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let res = task::ready!(self.project().future.poll(cx))?;
        Poll::Ready(Ok(res.map(HttpBody04ToHttpBody1::new)))
    }
}

// --- hyper 1.0 (http) to tower-service 0.3 (http) ---

/// Converts a [hyper 1.0 HTTP `Service`] to a [tower-service 0.3 HTTP `Service`].
///
/// An HTTP `Service` is a `Service` where the request is [`http::Request<_>`][Request] and the
/// response is [`http::Response<_>`][Response].
pub struct Hyper1HttpServiceAsTowerService03HttpService<S, B> {
    service: S,
    _marker: PhantomData<fn() -> B>,
}

impl<S, B> Hyper1HttpServiceAsTowerService03HttpService<S, B> {
    /// Create a new `Hyper1HttpServiceAsTowerService03HttpService`.
    pub fn new(service: S) -> Self {
        Self {
            service,
            _marker: PhantomData,
        }
    }
}

impl<S, B> Debug for Hyper1HttpServiceAsTowerService03HttpService<S, B>
where
    S: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Hyper1HttpServiceAsTowerService03HttpService")
            .field("service", &self.service)
            .finish()
    }
}

impl<S, B> Clone for Hyper1HttpServiceAsTowerService03HttpService<S, B>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            service: self.service.clone(),
            _marker: self._marker,
        }
    }
}

impl<S, B> Copy for Hyper1HttpServiceAsTowerService03HttpService<S, B> where S: Copy {}

impl<S, ReqBody, ResBody> tower_service_03::Service<Request<ReqBody>>
    for Hyper1HttpServiceAsTowerService03HttpService<S, ReqBody>
where
    S: hyper_1::service::Service<
        Request<HttpBody04ToHttpBody1<ReqBody>>,
        Response = Response<ResBody>,
    >,
{
    type Response = Response<HttpBody1ToHttpBody04<ResBody>>;
    type Error = S::Error;
    type Future = Hyper1HttpServiceAsTowerService03HttpServiceFuture<S::Future>;

    #[inline]
    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let req = req.map(HttpBody04ToHttpBody1::new);
        Hyper1HttpServiceAsTowerService03HttpServiceFuture {
            future: self.service.call(req),
        }
    }
}

pin_project! {
    /// Response future for [`Hyper1HttpServiceAsTowerService03HttpService`].
    pub struct Hyper1HttpServiceAsTowerService03HttpServiceFuture<F> {
        #[pin]
        future: F,
    }
}

impl<F, B, E> Future for Hyper1HttpServiceAsTowerService03HttpServiceFuture<F>
where
    F: Future<Output = Result<Response<B>, E>>,
{
    type Output = Result<Response<HttpBody1ToHttpBody04<B>>, E>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let res = task::ready!(self.project().future.poll(cx))?;
        Poll::Ready(Ok(res.map(HttpBody1ToHttpBody04::new)))
    }
}
