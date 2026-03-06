/// JWKS fetching and in-process caching.
///
/// Cognito exposes public RSA keys at a well-known JWKS endpoint.
/// We fetch them once and cache for `TTL` (1 hour), refreshing
/// automatically when a token arrives with an unknown `kid`.

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use jsonwebtoken::DecodingKey;
use reqwest::Client;
use serde::Deserialize;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::{config::CognitoConfig, error::AuthError};

/// How long cached keys are considered fresh.
const TTL: Duration = Duration::from_secs(3600);

// ── Wire types (Cognito JWKS JSON) ────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct JwksResponse {
    keys: Vec<JwkKey>,
}

#[derive(Debug, Deserialize)]
struct JwkKey {
    /// Key type — we only care about `"RSA"`.
    kty: String,
    /// Intended use — we only care about `"sig"`.
    #[serde(rename = "use")]
    key_use: String,
    /// Key ID — matches the `kid` header in the JWT.
    kid: String,
    /// Base64url-encoded RSA modulus.
    n: String,
    /// Base64url-encoded RSA public exponent.
    e: String,
}

// ── Cache ──────────────────────────────────────────────────────────────────────

/// Thread-safe, lazily-refreshing JWKS key cache.
///
/// Wrap in `Arc<RwLock<JwksCache>>` and keep one instance in `AppState`.
pub struct JwksCache {
    /// `kid` → decoded RSA public key.
    keys: HashMap<String, DecodingKey>,
    /// When keys were last successfully fetched.
    last_fetched: Option<Instant>,
    /// HTTP client reused across refreshes.
    http: Client,
}

impl JwksCache {
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            last_fetched: None,
            http: Client::new(),
        }
    }

    /// Returns a shared reference to the current key map, refreshing if stale.
    ///
    /// Callers must hold a **write** lock while calling this because a refresh
    /// may mutate `self`.  The double-check inside avoids redundant fetches
    /// when multiple tasks race to refresh simultaneously.
    pub async fn keys_refreshed(
        &mut self,
        config: &CognitoConfig,
    ) -> Result<&HashMap<String, DecodingKey>, AuthError> {
        let stale = self
            .last_fetched
            .map(|t| t.elapsed() >= TTL)
            .unwrap_or(true);

        if stale {
            info!("JWKS cache stale — refreshing from Cognito");
            self.keys = fetch_jwks(&self.http, config).await?;
            self.last_fetched = Some(Instant::now());
        }

        Ok(&self.keys)
    }

    /// Lookup a key by kid, refreshing if not found (handles key rotation).
    pub async fn get_key(
        cache: &Arc<RwLock<Self>>,
        kid: &str,
        config: &CognitoConfig,
    ) -> Result<DecodingKey, AuthError> {
        // Fast path: read lock, key present and cache is fresh.
        {
            let guard = cache.read().await;
            if guard.last_fetched.map(|t| t.elapsed() < TTL).unwrap_or(false) {
                if let Some(key) = guard.keys.get(kid) {
                    debug!(kid, "JWKS cache hit");
                    return Ok(key.clone());
                }
            }
        }

        // Slow path: write lock, refresh, then look up.
        let mut guard = cache.write().await;
        // Double-check: another task may have refreshed while we waited.
        if guard.last_fetched.map(|t| t.elapsed() < TTL).unwrap_or(false) {
            if let Some(key) = guard.keys.get(kid) {
                return Ok(key.clone());
            }
        }

        let keys = guard.keys_refreshed(config).await?;
        keys.get(kid)
            .cloned()
            .ok_or_else(|| {
                warn!(kid, "kid not found in JWKS after refresh");
                AuthError::UnknownKid
            })
    }
}

impl Default for JwksCache {
    fn default() -> Self { Self::new() }
}

// ── Fetch helper ──────────────────────────────────────────────────────────────

async fn fetch_jwks(
    http: &Client,
    config: &CognitoConfig,
) -> Result<HashMap<String, DecodingKey>, AuthError> {
    let url = config.jwks_url();
    debug!(url, "fetching JWKS");

    let resp: JwksResponse = http
        .get(&url)
        .send()
        .await
        .map_err(|e| AuthError::JwksFetchError(e.to_string()))?
        .error_for_status()
        .map_err(|e| AuthError::JwksFetchError(e.to_string()))?
        .json()
        .await
        .map_err(|e| AuthError::JwksFetchError(format!("parse error: {e}")))?;

    let mut keys = HashMap::new();
    for jwk in resp.keys {
        if jwk.kty != "RSA" || jwk.key_use != "sig" {
            continue;
        }
        match DecodingKey::from_rsa_components(&jwk.n, &jwk.e) {
            Ok(dk) => {
                debug!(kid = %jwk.kid, "loaded RSA public key");
                keys.insert(jwk.kid, dk);
            }
            Err(e) => {
                warn!(kid = %jwk.kid, error = %e, "failed to build DecodingKey from JWKS entry");
            }
        }
    }

    info!(count = keys.len(), "JWKS refreshed");
    Ok(keys)
}
