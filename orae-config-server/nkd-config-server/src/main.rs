use nkd_http_server::AppBuilder;

use crate::{controller::ConfigController, state::AppState};

mod controller;
mod service;
mod state;

#[nkd_http_server::main(
    state = AppState,
)]
async fn main(app_builder: AppBuilder<AppState>) {
    let state = AppState::new();
    app_builder
        .controller(ConfigController)
        .build(state)
        .run()
        .await
}
