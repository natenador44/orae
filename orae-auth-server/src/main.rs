use orae_http_server::{
    AppBuilder, LoggingConfig,
    auth::{OAuth2, OAuth2State},
};

mod login;
mod user;

#[derive(Clone)]
struct State {
    oauth2: OAuth2,
}

#[tokio::main]
async fn main() {
    let app_builder = AppBuilder::new();

    app_builder
        .enable_logging(LoggingConfig::default())
        .router_group(login::LoginRouterGroup)
        .build(())
        .run()
        .await
}
