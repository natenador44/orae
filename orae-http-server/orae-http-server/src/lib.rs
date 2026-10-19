use axum::Router;
use axum::response::Response;
pub use macros::{delete, get, patch, post, put, router_group};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::{Span, info};
use tracing_error::ErrorLayer;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;

use crate::responses::HandlerResult;

pub mod context;
pub mod responses;

// TODO put behind some tracing subscriber feature
pub fn setup_logging(_logging_config: &LoggingConfig) {
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .with(tracing_subscriber::fmt::layer().with_ansi(false))
        .with(ErrorLayer::default()) // <- required for SpanTrace::capture() to work
        .init();
}

pub struct LoggingConfig {
    outputs: Vec<LoggingOutput>,
    input: LoggingInput,
    http_logging: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            outputs: vec![LoggingOutput::Console],
            input: LoggingInput::Env,
            http_logging: true,
        }
    }
}

pub enum LoggingOutput {
    Console,
}

pub enum LoggingInput {
    Env,
}

pub struct ServerConfig {
    pub addr: std::net::SocketAddr,
}
impl ServerConfig {
    pub fn from_ip4_port(ip4: Ipv4Addr, port: u16) -> Self {
        Self {
            addr: SocketAddr::V4(SocketAddrV4::new(ip4, port)),
        }
    }

    pub fn unspecified_with_port(port: u16) -> Self {
        Self {
            addr: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port)),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            addr: "0.0.0.0:3000".parse().unwrap(),
        }
    }
}

pub struct App {
    router: Router,
    server_config: ServerConfig,
    logging_config: Option<LoggingConfig>,
}

impl App {
    pub async fn run(self) {
        let listener = tokio::net::TcpListener::bind(self.server_config.addr)
            .await
            .unwrap();
        tracing::info!(
            "starting server - listening on port {}",
            self.server_config.addr.port()
        );
        axum::serve(listener, self.router).await.unwrap();
    }
}

pub struct AppBuilder<T> {
    router: Router<T>,
    route_mounts: Vec<Box<dyn FnOnce() -> axum::Router<T> + Send>>,
    server_config: Option<ServerConfig>,
    logging: Option<LoggingConfig>,
}

impl<T> AppBuilder<T>
where
    T: Clone + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            router: Router::new(),
            route_mounts: Vec::new(),
            server_config: None,
            logging: None,
        }
    }

    pub fn router_group<R>(mut self, router: R) -> Self
    where
        R: RouterGroup<T> + Send + Sync + 'static,
    {
        self.route_mounts.push(Box::new(move || {
            let ctx = R::context();
            if ctx.is_empty() || ctx == "/" {
                router.router()
            } else {
                Router::new().nest(R::context(), router.router())
            }
        }));

        self
    }

    pub fn server_config(mut self, server_config: ServerConfig) -> Self {
        self.server_config = Some(server_config);
        self
    }

    pub fn enable_logging(mut self, logging_config: LoggingConfig) -> Self {
        self.logging = Some(logging_config);
        self
    }

    pub fn build(self, state: T) -> App {
        let mut router = if let Some(logging_config) = &self.logging {
            setup_logging(logging_config);
            if logging_config.http_logging {
                let http_handler = |res: &Response, latency: Duration, _span: &Span| {
                    info!(
                        status = res.status().as_u16(),
                        duration_ms = latency.as_millis(),
                        "returned"
                    );
                };
                let layer = ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http().on_response(http_handler));
                self.router.layer(layer)
            } else {
                self.router
            }
        } else {
            self.router
        };

        for route_mount in self.route_mounts {
            router = router.merge(route_mount());
        }

        App {
            router: router.with_state(state),
            server_config: self.server_config.unwrap_or_default(),
            logging_config: self.logging,
        }
    }
}

pub trait RouterGroup<S> {
    fn context() -> &'static str;
    fn router(&self) -> axum::Router<S>;
}

pub trait Route<T> {
    fn init(router: axum::Router<T>) -> axum::Router<T>;
    fn method() -> &'static str;
    fn path() -> &'static str;
}

pub trait RouteHandler {
    type Ok;
    fn handle(self) -> impl std::future::Future<Output = HandlerResult<Self::Ok>> + Send;
}
