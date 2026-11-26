use axum::{
    extract::FromRequestParts,
    http::request::Parts,
    response::{IntoResponse, Response},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use reqwest::{StatusCode, header::AUTHORIZATION};
use tracing::error;

const AUTH_TYPE: &str = "Basic";

pub struct BasicAuth {
    pub username: String,
    pub password: String,
}

impl<S> FromRequestParts<S> for BasicAuth
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok());

        let Some(auth_header) = auth_header else {
            return Err(StatusCode::PRECONDITION_FAILED.into_response());
        };

        if !auth_header.starts_with(AUTH_TYPE) {
            return Err(StatusCode::UNAUTHORIZED.into_response());
        }

        let creds = &auth_header[AUTH_TYPE.len() + 1..];

        let decoded = STANDARD.decode(creds).map_err(|e| {
            error!("failed to decode basic auth header: {e}");
            StatusCode::PRECONDITION_FAILED.into_response()
        })?;

        let decoded = String::from_utf8(decoded).map_err(|e| {
            error!("failed to create utf8 string from basic auth header: {e}");
            StatusCode::PRECONDITION_FAILED.into_response()
        })?;

        let Some((username, password)) = decoded.split_once(':') else {
            error!("basic auth header creds not separated by ':'");
            return Err(StatusCode::PRECONDITION_FAILED.into_response());
        };

        Ok(Self {
            username: username.into(),
            password: password.into(),
        })
    }
}
