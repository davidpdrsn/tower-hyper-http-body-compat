//! Adapters between hyper 0.14-1.0, http-body 0.4-1.0, and tower-service 0.3.
//!
//! The required release candidates are:
//!
//! - hyper 1.0.0-rc.3
//! - http-body 1.0.0-rc.2
//!
//! # Example
//!
//! Running an axum `Router` with hyper 1.0:
//!
//! ```no_run
//! # use hyper_1 as hyper;
//! use axum::{Router, routing::get};
//! use hyper::server::conn::http1;
//! use std::net::SocketAddr;
//! use tokio::net::TcpListener;
//! use tower_http::trace::TraceLayer;
//! use tower_hyper_http_body_compat::TowerService03HttpServiceAsHyper1HttpService;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!     let app = Router::new()
//!         .route("/", get(|| async { "Hello, World!" }))
//!         // we can still add regular tower middleware
//!         .layer(TraceLayer::new_for_http());
//!
//!     // `Router` implements tower-service 0.3's `Service` trait. Convert that to something
//!     // that implements hyper 1.0's `Service` trait.
//!     let service = TowerService03HttpServiceAsHyper1HttpService::new(app);
//!
//!     let addr: SocketAddr = ([127, 0, 0, 1], 8080).into();
//!
//!     let mut tcp_listener = TcpListener::bind(addr).await?;
//!     loop {
//!         let (tcp_stream, _) = tcp_listener.accept().await?;
//!         let service = service.clone();
//!         tokio::task::spawn(async move {
//!             if let Err(http_err) = http1::Builder::new()
//!                     .keep_alive(true)
//!                     .serve_connection(tcp_stream, service)
//!                     .await
//!             {
//!                 eprintln!("Error while serving HTTP connection: {}", http_err);
//!             }
//!         });
//!     }
//! }
//! ```
//!
//! Note that this library doesn't require axum. Its supports any [`tower::Service`].
//!
//! # Feature flags
//!
//! To enable the `Service` adapters you must enable either `http1` or `http2` and `server` or
//! `client` (i.e. `(http1 || http2) && (server || client)`).
//!
//! The `Body` adapters are always enabled.

#![warn(
    clippy::all,
    clippy::dbg_macro,
    clippy::todo,
    clippy::empty_enum,
    clippy::enum_glob_use,
    clippy::mem_forget,
    clippy::unused_self,
    clippy::filter_map_next,
    clippy::needless_continue,
    clippy::needless_borrow,
    clippy::match_wildcard_for_single_variants,
    clippy::if_let_mutex,
    clippy::mismatched_target_os,
    clippy::await_holding_lock,
    clippy::match_on_vec_items,
    clippy::imprecise_flops,
    clippy::suboptimal_flops,
    clippy::lossy_float_literal,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::fn_params_excessive_bools,
    clippy::exit,
    clippy::inefficient_to_string,
    clippy::linkedlist,
    clippy::macro_use_imports,
    clippy::option_option,
    clippy::verbose_file_reads,
    clippy::unnested_or_patterns,
    clippy::str_to_string,
    rust_2018_idioms,
    future_incompatible,
    nonstandard_style,
    missing_debug_implementations,
    missing_docs
)]
#![deny(unreachable_pub, private_in_public)]
#![allow(elided_lifetimes_in_paths, clippy::type_complexity)]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]
#![cfg_attr(test, allow(clippy::float_cmp))]

macro_rules! cfg_service {
    ($($item:item)*) => {
        $(
            #[cfg(all(
                any(feature = "http1", feature = "http2"),
                any(feature = "server", feature = "client")
            ))]
            $item
        )*
    };
}

cfg_service! {
    mod service;
    mod http_service;

    pub use service::{Hyper1ServiceAsTowerService03Service, TowerService03ServiceAsHyper1Service};
    pub use http_service::{
        Hyper1HttpServiceAsTowerService03HttpService, TowerService03HttpServiceAsHyper1HttpService,
    };
}

mod body;

pub use body::{HttpBody04ToHttpBody1, HttpBody1ToHttpBody04};

#[cfg(test)]
mod tests;

pub mod future {
    //! Future types.

    cfg_service! {
        pub use crate::http_service::{
            Hyper1HttpServiceAsTowerService03HttpServiceFuture,
            TowerService03HttpServiceAsHyper1HttpServiceFuture,
        };
        pub use crate::service::TowerService03ServiceAsHyper1ServiceFuture;
    }
}
