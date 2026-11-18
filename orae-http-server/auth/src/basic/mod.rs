use axum::{http::request::Parts, response::Response};

use crate::{AuthProvider, Roles};

pub struct BasicAuthProvider {}

impl<R> AuthProvider<R> for BasicAuthProvider {
    async fn authenticate(&self, parts: &mut Parts) -> Result<crate::User<Self, R>, Response>
    where
        Self: Sized,
        R: Roles,
    {
        todo!()
    }
}
