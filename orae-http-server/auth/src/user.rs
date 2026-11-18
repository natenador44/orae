use std::{marker::PhantomData, sync::Arc};

use axum::{
    RequestPartsExt,
    extract::{FromRef, FromRequestParts, State},
    response::{IntoResponse, Response},
};

use crate::{AuthProvider, roles::Roles};

#[derive(Debug, Clone)]
pub struct User<S, R> {
    pub id: Arc<str>,
    pub email: Option<Arc<str>>,
    pub roles: R,
    auth_source: PhantomData<S>,
}

impl<A, S, R> FromRequestParts<S> for User<A, R>
where
    S: Send + Sync,
    A: FromRef<S> + AuthProvider<R> + Send + Sync + 'static,
    R: Roles,
{
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let State(auth_state) = parts
            .extract_with_state::<State<A>, S>(state)
            .await
            .map_err(|e| e.into_response())?;

        AuthProvider::authenticate(&auth_state, parts).await
    }
}

impl<S, R> User<S, R>
where
    R: Roles,
{
    pub fn new(id: Arc<str>, email: Option<Arc<str>>, roles: R) -> Self {
        Self {
            id,
            email,
            roles,
            auth_source: PhantomData,
        }
    }

    pub fn has_roles<I: Into<R>>(&self, role: I) -> bool {
        self.roles.contains(role.into())
    }
}
