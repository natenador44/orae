use orae_http_server::AppBuilder;

use crate::{controller::ConfigRouterGroup, state::AppState};

mod controller;
mod service;
mod state;

#[tokio::main]
async fn main() {
    let app_builder = AppBuilder::new();
    let state = AppState::new();
    app_builder
        .router_group(ConfigRouterGroup)
        .build(state)
        .run()
        .await
}
