use axum::http::StatusCode;
use axum::Json;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

pub struct NoContent;
impl IntoResponse for NoContent {
    fn into_response(self) -> Response {
        StatusCode::NO_CONTENT.into_response()
    }
}

pub struct Accepted;
impl IntoResponse for Accepted {
    fn into_response(self) -> Response {
        StatusCode::ACCEPTED.into_response()
    }
}

pub struct Created<T>(pub T);
impl<T: IntoResponse> IntoResponse for Created<T> {
    fn into_response(self) -> Response {
        (StatusCode::CREATED, self.0).into_response()
    }
}