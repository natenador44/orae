use axum::{http::request::Parts, response::Response};

use crate::{AuthProvider, Roles};

pub struct BasicAuth {}

impl<R> AuthProvider<R> for BasicAuth {
    async fn retrieve(&self, parts: &mut Parts) -> Result<crate::User<Self, R>, Response>
    where
        Self: Sized,
        R: Roles,
    {
        todo!()
    }
}
