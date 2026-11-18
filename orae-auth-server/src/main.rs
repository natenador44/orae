use orae_http_server::{AppBuilder, LoggingConfig};

mod login;
mod user;

#[tokio::main]
async fn main() {
    let app_builder = AppBuilder::new();

    app_builder
        .enable_logging(LoggingConfig::default())
        .router_group(router)
}
