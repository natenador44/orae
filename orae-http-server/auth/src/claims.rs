use serde::{Deserialize, Serialize};

use crate::{oauth::OAuth2, roles::Roles, user::User};

pub trait Claims<'a>: Deserialize<'a> + Serialize + Clone {
    fn into_authed_user<R>(self, roles_path: &str) -> User<OAuth2<Self>, R>
    where
        R: Roles;
}
