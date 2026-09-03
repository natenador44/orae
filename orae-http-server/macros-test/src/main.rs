use std::{collections::HashMap, convert::Infallible};

use nkd_http_server::{
    AppBuilder, RouteHandler,
    axum::{extract::FromRef, http::StatusCode},
    context::RequestContext,
    controller, get, post,
    responses::HandlerResult,
};
use serde::Deserialize;

#[nkd_http_server::main(
    state = AppState,
)]
async fn main(app_builder: AppBuilder<AppState>) {
    let state = AppState {
        test_service: TestService,
    };
    let app = app_builder.build(state);

    app.run().await;
}

#[derive(Debug, Clone)]
pub struct AppState {
    test_service: TestService,
}

#[derive(Debug, Clone)]
struct TestService;
impl TestService {
    async fn do_stuff(&self) {}
}
impl FromRef<AppState> for TestService {
    fn from_ref(input: &AppState) -> Self {
        input.test_service.clone()
    }
}

#[controller(
    context = "/device",
    routes = [
        GetDevice,
        CreateDevice,
    ]
)]
pub struct DeviceController;

// annotated and filled out by user
#[get("/{device_id}")] // get, post, put, delete, patch are all options
pub struct GetDevice {
    #[state]
    test_service: TestService, // must be impl axum::extract::FromRef<Controller::State>
    #[context]
    ctx: RequestContext,
    #[path]
    device_id: u64, // type must work with axum::extract::Path
    #[query_params]
    params: HashMap<String, String>, // type must work with axum::extract::Query
}

// implemented by user
impl RouteHandler for GetDevice {
    type Ok = StatusCode;
    async fn handle(self) -> HandlerResult<Self::Ok> {
        nkd_http_server::tracing::debug!("getting device {}", self.device_id);
        self.test_service.do_stuff().await;
        Ok(StatusCode::OK)
    }
}

#[derive(Deserialize)]
pub struct CreateDeviceRequest {
    name: String,
    serial_number: String,
}

#[post("/")]
pub struct CreateDevice {
    #[state]
    test_service: TestService,
    #[request_body]
    body: CreateDeviceRequest,
}

impl RouteHandler for CreateDevice {
    type Ok = StatusCode;
    type Err = Infallible;
    async fn handle(self) -> Result<Self::Ok, Self::Err> {
        nkd_http_server::tracing::info!("created device");
        Ok(StatusCode::CREATED)
    }
}
