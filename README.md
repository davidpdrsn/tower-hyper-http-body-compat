# tower-hyper-http-body-compat

`tower-hyper-http-body-compat` provides adapters between hyper 0.14-1.0,
http-body 0.4-1.0, and tower-service 0.3.

[![Build status](https://github.com/davidpdrsn/tower-hyper-http-body-compat/actions/workflows/CI.yml/badge.svg?branch=main)](https://github.com/davidpdrsn/tower-hyper-http-body-compat/actions/workflows/CI.yml)
[![Crates.io](https://img.shields.io/crates/v/tower-hyper-http-body-compat)](https://crates.io/crates/tower-hyper-http-body-compat)
[![Documentation](https://docs.rs/tower-hyper-http-body-compat/badge.svg)](https://docs.rs/tower-hyper-http-body-compat)

More information about this crate can be found in the [crate
documentation][docs].

## Example

Running an axum `Router` with hyper 1.0:

```rust
use axum::{Router, routing::get};
use hyper::server::conn::http1;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tower_hyper_http_body_compat::TowerService03HttpServiceAsHyper1HttpService;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
         // we can still add regular tower middleware
         .layer(TraceLayer::new_for_http());

    // `Router` implements tower-service 0.3's `Service` trait. Convert that to something
    // that implements hyper 1.0's `Service` trait.
    let service = TowerService03HttpServiceAsHyper1HttpService::new(app);

    let addr: SocketAddr = ([127, 0, 0, 1], 8080).into();

    let mut tcp_listener = TcpListener::bind(addr).await?;
    loop {
        let (tcp_stream, _) = tcp_listener.accept().await?;
        let service = service.clone();
        tokio::task::spawn(async move {
            if let Err(http_err) = http1::Builder::new()
                    .keep_alive(true)
                    .serve_connection(tcp_stream, service)
                    .await {
                eprintln!("Error while serving HTTP connection: {}", http_err);
            }
        });
    }
}
```

[docs]: https://docs.rs/tower-hyper-http-body-compat
