use std::fmt::Debug;

use anyhow::{Context, Result};
use tracing::{error, info, instrument};

use crate::{
    ArwLock,
    oauth::{Jwk, Jwks, JwksState, OAuthConfig},
};

#[derive(Debug, Clone)]
pub struct OAuth2State {
    pub jwks: JwksState,
    pub oauth_config: OAuthConfig,
}

impl OAuth2State {
    pub async fn create() -> Result<Self> {
        let oauth_config = OAuthConfig::from_env()?;
        Self::create_with(oauth_config).await
    }

    pub async fn create_with(oauth_config: OAuthConfig) -> Result<Self> {
        let jwks = refresh_jwks_from_url(&oauth_config.jwks_url).await?;
        Ok(Self {
            jwks: JwksState {
                keys: ArwLock::new(jwks),
            },
            oauth_config,
        })
    }

    #[instrument]
    pub async fn refresh_jwks(&mut self) -> Result<()> {
        let jwks = refresh_jwks_from_url(&self.oauth_config.jwks_url).await?;
        let mut keys = self.jwks.keys.write().await;
        *keys = jwks;
        Ok(())
    }
}

#[instrument]
async fn refresh_jwks_from_url(jwks_uri: &str) -> Result<Vec<Jwk>> {
    info!("fetching JWKS");

    let jwks: Jwks = reqwest::get(jwks_uri)
        .await
        .context("failed to fetch jwks uri")?
        .json()
        .await
        .context("failed to parse jwks uri response as JSON")?;

    if jwks.keys.is_empty() {
        error!("no jwks were found");
    } else {
        info!("found {} jwks", jwks.keys.len());
    }
    Ok(jwks.keys)
}
