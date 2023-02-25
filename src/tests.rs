use std::convert::Infallible;

use bytes::Bytes;
use http::{Request, Response, StatusCode};
use http_body_util::BodyExt;
use hyper_1::server::conn::http1;
use tokio::net::TcpListener;

use crate::*;

#[tokio::test]
async fn tower_service_03_service_to_hyper_1_service() {
    async fn handle<B>(req: Request<B>) -> Result<Response<hyper_014::Body>, Infallible>
    where
        B: http_body_04::Body,
    {
        let bytes = hyper_014::body::to_bytes(req)
            .await
            .unwrap_or_else(|_| panic!());
        assert_eq!(bytes, "in");
        Ok(Response::new(hyper_014::Body::from("out")))
    }

    let svc = tower::service_fn(handle);
    let svc = TowerService03HttpServiceAsHyper1HttpService::new(svc);

    let tcp_listener = TcpListener::bind("0.0.0.0:0").await.unwrap();
    let addr = tcp_listener.local_addr().unwrap();
    tokio::task::spawn(async move {
        loop {
            let (tcp_stream, _) = tcp_listener.accept().await.unwrap();
            tokio::spawn(async move {
                http1::Builder::new()
                    .serve_connection(tcp_stream, svc)
                    .await
                    .unwrap();
            });
        }
    });

    let client = hyper_014::Client::builder().build_http();
    let mut res = client
        .request(
            Request::builder()
                .uri(format!("http://{addr}"))
                .body(hyper_014::Body::from("in"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    let bytes = hyper_014::body::to_bytes(&mut res).await.unwrap();
    assert_eq!(bytes, "out");
}

#[tokio::test]
async fn hyper_1_service_to_tower_service_03_service() {
    async fn handle<B>(req: Request<B>) -> Result<Response<http_body_util::Full<Bytes>>, Infallible>
    where
        B: http_body_1::Body,
    {
        let collected = req.into_body().collect().await.unwrap_or_else(|_| panic!());
        assert_eq!(collected.to_bytes(), "in");

        Ok(Response::new(http_body_util::Full::new(Bytes::from("out"))))
    }

    let svc = hyper_1::service::service_fn(handle);
    let svc = Hyper1HttpServiceAsTowerService03HttpService::new(svc);

    let tcp_listener = std::net::TcpListener::bind("0.0.0.0:0").unwrap();
    let addr = tcp_listener.local_addr().unwrap();
    tokio::spawn(async move {
        hyper_014::Server::from_tcp(tcp_listener)
            .unwrap()
            .serve(tower::make::Shared::new(svc))
            .await
            .unwrap();
    });

    let client = hyper_014::Client::builder().build_http();
    let mut res = client
        .request(
            Request::builder()
                .uri(format!("http://{addr}"))
                .body(hyper_014::Body::from("in"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    let bytes = hyper_014::body::to_bytes(&mut res).await.unwrap();
    assert_eq!(bytes, "out");
}
