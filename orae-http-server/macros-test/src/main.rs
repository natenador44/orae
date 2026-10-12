use std::{collections::HashMap, convert::Infallible};
use orae_http_server::{router, AppBuilder, RouteHandler, get, ServerConfig};
use orae_http_server::responses::HandlerResult;

#[derive(Clone)]
struct AppState {}

#[orae_http_server::main(state = AppState)]
async fn main(app_builder: AppBuilder<AppState>) {
    app_builder.router(TestRouter)
        .server_config(ServerConfig::default())
        .build(AppState {}).run()
        .await
}

#[router(
    routes = [
        GetTest
    ],
    context = "/"
)]
struct TestRouter;

#[get("/test/{test_id}")]
#[instrument]
struct GetTest {
    #[path]
    test_id: u32
}
impl RouteHandler for GetTest {
    type Ok = ();

    async fn handle(self) -> HandlerResult<Self::Ok> {
        todo!()
    }
}

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use tracing::instrument;
use orae_http_server::responses::{ResponseVariants};

struct NoContent;
impl IntoResponse for NoContent {
    fn into_response(self) -> Response {
        StatusCode::NO_CONTENT.into_response()
    }
}

#[derive(ResponseVariants)]
enum ListResponse {
    Empty(NoContent),
    Items(Json<Vec<String>>),
}

#[derive(ResponseVariants)]
enum JsonTag {
    Empty(NoContent),
    #[json]
    Items(Vec<String>)
}

fn list(empty: bool) -> ListResponse {
    if empty {
        NoContent.into()
    } else {
        Json(vec!["a".to_string()]).into()
    }
}


#[derive(ResponseVariants)]
enum Mixed {
    Created { reply: (StatusCode, &'static str) },
    Gone(NoContent),
}

// #[test]
// fn named_field_variants() {
//     let m: Mixed = (StatusCode::CREATED, "made").into();
//     assert_eq!(m.into_response().status(), StatusCode::CREATED);
//     let g: Mixed = NoContent.into();
//     assert_eq!(g.into_response().status(), StatusCode::NO_CONTENT);
// }

// #[test]
// fn generic_enum() {
//     let l: Either<NoContent, Json<u8>> = Either::Left(NoContent);
//     let r: Either<NoContent, Json<u8>> = Either::Right(Json(1));
//     assert_eq!(l.into_response().status(), StatusCode::NO_CONTENT);
//     assert_eq!(r.into_response().status(), StatusCode::OK);
// }

#[derive(ResponseVariants)]
enum WithWhere<T>
where
    T: Send,
{
    Only(T),
}

// #[test]
// fn existing_where_clause_is_kept() {
//     let w: WithWhere<NoContent> = WithWhere::Only(NoContent);
//     assert_eq!(w.into_response().status(), StatusCode::NO_CONTENT);
// }
//
// use orae_http_server::{
//     AppBuilder, RouteHandler,
//     axum::{extract::FromRef, http::StatusCode},
//     context::RequestContext,
//     controller, get, post,
//     responses::HandlerResult,
// };
// use serde::Deserialize;
//
// #[nkd_http_server::main(
//     state = AppState,
// )]
// async fn main(app_builder: AppBuilder<AppState>) {
//     let state = AppState {
//         test_service: TestService,
//     };
//     let app = app_builder.build(state);
//
//     app.run().await;
// }
//
// #[derive(Debug, Clone)]
// pub struct AppState {
//     test_service: TestService,
// }
//
// #[derive(Debug, Clone)]
// struct TestService;
// impl TestService {
//     async fn do_stuff(&self) {}
// }
// impl FromRef<AppState> for TestService {
//     fn from_ref(input: &AppState) -> Self {
//         input.test_service.clone()
//     }
// }
//
// #[controller(
//     context = "/device",
//     routes = [
//         GetDevice,
//         CreateDevice,
//     ]
// )]
// pub struct DeviceController;
//
// // annotated and filled out by user
// #[get("/{device_id}")] // get, post, put, delete, patch are all options
// pub struct GetDevice {
//     #[state]
//     test_service: TestService, // must be impl axum::extract::FromRef<Controller::State>
//     #[context]
//     ctx: RequestContext,
//     #[path]
//     device_id: u64, // type must work with axum::extract::Path
//     #[query_params]
//     params: HashMap<String, String>, // type must work with axum::extract::Query
// }
//
// // implemented by user
// impl RouteHandler for GetDevice {
//     type Ok = StatusCode;
//     async fn handle(self) -> HandlerResult<Self::Ok> {
//         nkd_http_server::tracing::debug!("getting device {}", self.device_id);
//         self.test_service.do_stuff().await;
//         Ok(StatusCode::OK)
//     }
// }
//
// #[derive(Deserialize)]
// pub struct CreateDeviceRequest {
//     name: String,
//     serial_number: String,
// }
//
// #[post("/")]
// pub struct CreateDevice {
//     #[state]
//     test_service: TestService,
//     #[request_body]
//     body: CreateDeviceRequest,
// }
//
// impl RouteHandler for CreateDevice {
//     type Ok = StatusCode;
//     type Err = Infallible;
//     async fn handle(self) -> Result<Self::Ok, Self::Err> {
//         nkd_http_server::tracing::info!("created device");
//         Ok(StatusCode::CREATED)
//     }
// }
