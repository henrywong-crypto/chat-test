/// JWT verification against Cognito JWKS.

use std::sync::Arc;

use jsonwebtoken::{decode, decode_header, Algorithm, Validation};
use serde::Deserialize;
use tokio::sync::RwLock;
use tracing::debug;

use shared::{User, UserGroup};

use crate::{config::CognitoConfig, error::AuthError, jwks::JwksCache};

// ── Cognito JWT claims ────────────────────────────────────────────────────────

/// Claims present in a Cognito *access* token.
///
/// Cognito access tokens do not contain `email` directly — that lives in ID
/// tokens.  We carry `username` (which Cognito sets to the email address for
/// email-based user pools) and fall back to `sub` for the display identity.
#[derive(Debug, Deserialize)]
struct CognitoClaims {
    /// Stable user identifier (UUID).
    sub: String,
    /// In email-based pools Cognito sets `username` to the email address.
    username: Option<String>,
    /// Explicitly present in ID tokens and sometimes access tokens.
    email: Option<String>,
    /// Cognito groups the user belongs to.
    #[serde(rename = "cognito:groups", default)]
    cognito_groups: Vec<String>,
    /// Must be `"access"` for access tokens.
    token_use: String,
    /// Token issuer — validated by `jsonwebtoken::Validation`; kept for debug output.
    #[allow(dead_code)]
    iss: String,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Verify a raw JWT bearer token and return the authenticated `User`.
///
/// Steps performed:
/// 1. Decode the header to extract `kid` (no signature check yet).
/// 2. Look up the matching RSA public key from the JWKS cache.
/// 3. Decode & verify the signature, expiry, issuer, and `token_use`.
/// 4. Map Cognito groups to `UserGroup` values.
pub async fn verify_token(
    token: &str,
    cache: &Arc<RwLock<JwksCache>>,
    config: &CognitoConfig,
) -> Result<User, AuthError> {
    // ── 1. Extract kid from the header ────────────────────────────────────────
    let header = decode_header(token)
        .map_err(|e| AuthError::InvalidToken(format!("bad header: {e}")))?;

    let kid = header
        .kid
        .ok_or_else(|| AuthError::InvalidToken("missing kid".into()))?;

    debug!(kid, "verifying JWT");

    // ── 2. Fetch the right RSA public key ─────────────────────────────────────
    let decoding_key = JwksCache::get_key(cache, &kid, config).await?;

    // ── 3. Decode + validate ──────────────────────────────────────────────────
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&[config.issuer_url()]);
    // `aud` is absent from Cognito access tokens; skip audience validation.
    validation.validate_aud = false;

    let token_data = decode::<CognitoClaims>(token, &decoding_key, &validation)
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::ExpiredToken,
            jsonwebtoken::errors::ErrorKind::InvalidIssuer    => AuthError::InvalidIssuer,
            _                                                  => AuthError::InvalidToken(e.to_string()),
        })?;

    let claims = token_data.claims;

    // ── 4. Extra claim checks ─────────────────────────────────────────────────
    if claims.token_use != "access" {
        return Err(AuthError::InvalidTokenUse);
    }

    // ── 5. Map to domain User ─────────────────────────────────────────────────
    let email = claims.email
        .or(claims.username)
        .unwrap_or_else(|| claims.sub.clone());

    let groups = claims
        .cognito_groups
        .iter()
        .map(|g| UserGroup::from_cognito_name(g))
        .collect();

    Ok(User { id: claims.sub, email, groups })
}
