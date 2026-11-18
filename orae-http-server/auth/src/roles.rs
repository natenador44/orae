use std::fmt::Display;

use axum::{
    body::Body,
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use serde_json::Value;
use tracing::{debug, error, warn};

use crate::{AuthProvider, user::User};

pub trait Roles: From<Vec<Value>> + Display + Clone + Send + Sync + 'static {
    fn none() -> Self;
    fn is_none(&self) -> bool;
    fn contains(&self, other: Self) -> bool;
    fn add(&mut self, other: Self);
}

pub async fn require_roles<P, R>(
    State(required_roles): State<R>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode>
where
    R: Roles,
    P: AuthProvider<R> + Send + Sync + 'static,
{
    let user = req.extensions().get::<User<P, R>>().ok_or_else(|| {
        error!("endpoint requires authorized user, none was found");
        StatusCode::UNAUTHORIZED
    })?;

    debug!("required roles: {required_roles}");
    if required_roles.is_none() || user.has_roles(required_roles) {
        Ok(next.run(req).await)
    } else {
        warn!("User {} does not have the authority! (🧙‍♂️🚫➡️)", user.id);
        Err(StatusCode::FORBIDDEN)
    }
}
