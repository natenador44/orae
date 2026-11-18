use std::marker::PhantomData;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use jsonwebtoken::{Algorithm, DecodingKey, Validation};
use reqwest::header::AUTHORIZATION;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, error};

use crate::{ArwLock, AuthProvider, OAuth2State, Roles, User};

#[derive(Debug, thiserror::Error)]
pub enum MissingOAuthProperty {
    #[error("missing jwks url")]
    JwksUrl,
    #[error("missing issuer url")]
    IssueUrl,
    #[error("missing roles path")]
    RolesPath,
    #[error("audience")]
    Audience,
}

#[derive(Debug, Clone)]
pub struct OAuthConfig {
    /// Where to fetch public keys
    pub jwks_url: String,
    /// Who issued the token (the URL of the issuer)
    pub issuer_url: String,
    /// Key in JWT for roles
    pub roles_claims_path: String,
    /// this api's identifier.
    // might all be the same, could maybe hard code this, but also maybe it's best to pass that info in via the env
    pub audience: String,
}

const OAUTH_JWKS_URL: &str = "OAUTH_JWKS_URL";
const OAUTH_ISSUER_URL: &str = "OAUTH_ISSUER_URL";
const OAUTH_ROLES_JWT_PATH: &str = "OAUTH_ROLES_JWT_PATH";
const OAUTH_AUDIENCE: &str = "OAUTH_AUDIENCE";

impl OAuthConfig {
    pub fn from_env() -> Result<Self, MissingOAuthProperty> {
        Ok(Self {
            jwks_url: std::env::var(OAUTH_JWKS_URL).map_err(|_| MissingOAuthProperty::JwksUrl)?,
            issuer_url: std::env::var(OAUTH_ISSUER_URL)
                .map_err(|_| MissingOAuthProperty::IssueUrl)?,
            roles_claims_path: std::env::var(OAUTH_ROLES_JWT_PATH)
                .map_err(|_| MissingOAuthProperty::RolesPath)?,
            audience: std::env::var(OAUTH_AUDIENCE).map_err(|_| MissingOAuthProperty::Audience)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct JwksState {
    pub keys: ArwLock<Vec<Jwk>>,
}

impl JwksState {
    pub async fn find_key(&self, kid: &str) -> Option<Jwk> {
        let keys = self.keys.read().await;
        keys.iter().find(|k| k.kid == kid).cloned()
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Jwks {
    pub keys: Vec<Jwk>,
}

// I understood this that night when I was using claude to figure out how this all works.
// felt like a fever dream.
// figure this out again
#[derive(Debug, Deserialize, Clone)]
pub struct Jwk {
    pub kid: String,
    // pub kty: String,
    // #[serde(rename = "use")]
    // pub key_use: Option<String>,
    // modulus
    pub n: String,
    // exponent
    pub e: String,
    // pub alg: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    sub: String,
    email: Option<String>,
    // apparently this can be a string or an array
    #[serde(default)]
    aud: Value,
    /// expiration
    exp: usize,
    /// issued?
    iss: String,
    #[serde(flatten)]
    extra: serde_json::Map<String, Value>,
}

impl<'a> crate::claims::Claims<'a> for Claims {
    fn into_authed_user<R>(self, roles_path: &str) -> User<OAuth2, R>
    where
        R: Roles,
    {
        let parts = roles_path.split('.');

        let mut current = Value::Object(self.extra);

        for part in parts {
            current = match current.get(part) {
                Some(val) => val.clone(),
                None => {
                    return User::new(self.sub.into(), self.email.map(|e| e.into()), R::none());
                }
            }
        }

        let roles = match current {
            Value::Array(roles) => R::from(roles),
            _ => R::none(),
        };

        User::new(self.sub.into(), self.email.map(|e| e.into()), roles)
    }
}

#[derive(Clone)]
pub struct OAuth2<C = Claims> {
    state: OAuth2State,
    claim_type: PhantomData<C>,
}

impl<C, R> AuthProvider<R> for OAuth2<C>
where
    C: for<'a> crate::claims::Claims<'a> + Send + Sync + 'static,
{
    async fn authenticate(
        &self,
        parts: &mut axum::http::request::Parts,
    ) -> Result<crate::User<Self, R>, Response>
    where
        Self: Sized,
        R: Roles,
    {
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok());

        let Some(auth_header) = auth_header else {
            return Err(StatusCode::PRECONDITION_FAILED.into_response());
        };

        if !auth_header.starts_with("Bearer ") {
            error!("invalid authorization type");
            return Err(StatusCode::UNAUTHORIZED.into_response());
        }

        let token = &auth_header["Bearer ".len()..];

        let header = jsonwebtoken::decode_header(token).map_err(|_| {
            error!("JWT token decoding (without verification) failed");
            StatusCode::UNAUTHORIZED.into_response()
        })?;
        let kid = header.kid.ok_or_else(|| {
            error!("invalid token: kid missing");
            StatusCode::UNAUTHORIZED.into_response()
        })?;

        let jwk = self.state.jwks.find_key(&kid).await.ok_or_else(|| {
            error!("kid key not found");
            StatusCode::UNAUTHORIZED.into_response()
        })?;

        let decoding_key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e).map_err(|_| {
            error!("failed to create decoding key");
            StatusCode::UNAUTHORIZED.into_response()
        })?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[&self.state.oauth_config.audience]);
        validation.set_issuer(&[&self.state.oauth_config.issuer_url]);

        let token_data =
            jsonwebtoken::decode::<C>(token, &decoding_key, &validation).map_err(|e| {
                error!("Token validation error: {e}");
                StatusCode::UNAUTHORIZED.into_response()
            })?;

        let authed_user = token_data
            .claims
            .into_authed_user::<R>(&self.state.oauth_config.roles_claims_path);

        tracing::Span::current().record("user_id", authed_user.id.to_string());
        match &authed_user.email {
            Some(email) => {
                debug!(
                    "token validated for '{email}'. roles: {}",
                    authed_user.roles
                );
            }
            None => {
                debug!("token validated. user's roles: {}", authed_user.roles)
            }
        }

        Ok(authed_user)
    }
}
