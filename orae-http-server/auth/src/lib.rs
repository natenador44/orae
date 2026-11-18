use std::sync::Arc;

use axum::{http::request::Parts, response::Response};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

mod basic;
mod claims;
mod oauth;
mod roles;
mod token;
mod user;

pub use roles::Roles;
pub use token::OAuthState;
pub use user::User;

pub trait AuthProvider<R> {
    fn authenticate(
        &self,
        request_parts: &mut Parts,
    ) -> impl std::future::Future<Output = Result<User<Self, R>, Response>> + Send
    where
        Self: Sized,
        R: Roles;
}

#[derive(Debug, Clone, Default)]
pub struct ArwLock<T>(Arc<RwLock<T>>);
impl<T> ArwLock<T> {
    pub fn new(data: T) -> Self {
        Self(Arc::new(RwLock::new(data)))
    }
    pub async fn read(&self) -> RwLockReadGuard<'_, T> {
        self.0.read().await
    }

    pub async fn write(&self) -> RwLockWriteGuard<'_, T> {
        self.0.write().await
    }
}
