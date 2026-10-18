use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use crate::responses::HandlerError;

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

pub struct BadRequest(Option<String>);

impl BadRequest {
    pub fn new() -> Self {
        Self(None)
    }

    pub fn with_msg(self, msg: impl Into<String>) -> Self {
        Self(Some(msg.into()))
    }
}
impl IntoResponse for BadRequest {
    fn into_response(self) -> Response {
        match self.0 {
            Some(msg) => (StatusCode::BAD_REQUEST, msg).into_response(),
            None => StatusCode::BAD_REQUEST.into_response(),
        }
    }
}