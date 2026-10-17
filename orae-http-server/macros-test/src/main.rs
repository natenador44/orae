use axum::extract::{FromRequestParts, Query};
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use orae_http_server::responses::HandlerResult;
use orae_http_server::{AppBuilder, RouteHandler, ServerConfig, get, router};
use serde::Deserialize;

#[derive(Clone)]
struct AppState {}

#[tokio::main]
async fn main() {
    let app_builder = AppBuilder::new();
    app_builder
        .router_group(TestRouter)
        .server_config(ServerConfig::default())
        .build(AppState {})
        .run()
        .await
}

#[router(
    routes = [
        GetTest
    ],
    context = "/"
)]
struct TestRouter;

#[derive(Debug, serde::Deserialize)]
struct Pagination {
    page: usize,
    page_len: usize,
}
#[derive(Debug, serde::Deserialize)]
struct MyQuery {
    name: String,
}
#[derive(Debug)]
struct Paginated<T> {
    pub filters: T,
    pub pagination: Pagination,
}
impl<S, T> FromRequestParts<S> for Paginated<T>
where
    T: for<'de> Deserialize<'de> + Send + Sync + 'static,
    S: Send + Sync + 'static,
{
    type Rejection = Response;
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(filters) = Query::<T>::from_request_parts(parts, state)
            .await
            .map_err(|err| err.into_response())?;
        let Query(pagination) = Query::<Pagination>::from_request_parts(parts, state)
            .await
            .map_err(|err| err.into_response())?;
        Ok(Paginated {
            filters,
            pagination,
        })
    }
}

#[get("/test/{test_id}")]
struct GetTest {
    #[path]
    test_id: u32,
    search: Paginated<MyQuery>,
}

impl RouteHandler for GetTest {
    type Ok = ();

    async fn handle(self) -> HandlerResult<Self::Ok> {
        todo!()
    }
}
//
// use axum::extract::FromRequestParts;
// use axum::http::request::Parts;
// use axum::{
//     Json,
//     http::StatusCode,
//     response::{IntoResponse, Response},
// };
// use axum_extra::extract::Query;
// use orae_http_server::responses::ResponseVariants;
// use serde::Deserialize;
// use tracing::instrument;
//
// struct NoContent;
// impl IntoResponse for NoContent {
//     fn into_response(self) -> Response {
//         StatusCode::NO_CONTENT.into_response()
//     }
// }

// #[derive(ResponseVariants)]
// enum ListResponse {
//     Empty(NoContent),
//     Items(Json<Vec<String>>),
// }
//
// #[derive(ResponseVariants)]
// enum JsonTag {
//     Empty(NoContent),
//     #[json]
//     Items(Vec<String>),
// }
//
// fn list(empty: bool) -> ListResponse {
//     if empty {
//         NoContent.into()
//     } else {
//         Json(vec!["a".to_string()]).into()
//     }
// }
//
// #[derive(ResponseVariants)]
// enum Mixed {
//     Created { reply: (StatusCode, &'static str) },
//     Gone(NoContent),
// }
//
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

// #[derive(ResponseVariants)]
// enum WithWhere<T>
// where
//     T: Send,
// {
//     Only(T),
// }

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
