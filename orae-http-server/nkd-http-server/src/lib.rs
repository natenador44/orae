use axum::Router;
pub use macros::{controller, delete, get, main, patch, post, put};
use tracing_error::ErrorLayer;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;

use crate::responses::HandlerResult;

pub mod context;
pub mod responses;

// TODO put behind some tracing subscriber feature
pub fn setup_logging() {
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .with(tracing_subscriber::fmt::layer())
        .with(ErrorLayer::default()) // <- required for SpanTrace::capture() to work
        .init();
}

pub struct ServerConfig {
    pub addr: std::net::SocketAddr,
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
    server_config: Option<ServerConfig>,
}

impl<T> AppBuilder<T>
where
    T: Clone + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            router: Router::new(),
            server_config: None,
        }
    }

    pub fn controller<C>(mut self, controller: C) -> Self
    where
        C: Controller<State = T>,
    {
        let ctx = C::context();
        let controller_router = if ctx.is_empty() || ctx == "/" {
            controller.router()
        } else {
            Router::new().nest(C::context(), controller.router())
        };

        self.router = self.router.merge(controller_router);

        self
    }

    pub fn server_config(mut self, server_config: ServerConfig) -> Self {
        self.server_config = Some(server_config);
        self
    }

    pub fn build(self, state: T) -> App {
        App {
            router: self.router.with_state(state),
            server_config: self.server_config.unwrap_or_default(),
        }
    }
}

pub trait Controller {
    type State;
    fn context() -> &'static str;
    fn router(&self) -> Router<Self::State>;
}

pub trait Route {
    type State;
    fn init(router: Router<Self::State>) -> Router<Self::State>;
    fn method() -> &'static str;
    fn path() -> &'static str;
}

pub trait RouteHandler {
    type Ok;
    fn handle(self) -> impl std::future::Future<Output = HandlerResult<Self::Ok>> + Send;
}
