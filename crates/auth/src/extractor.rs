/// Axum extractors and route-guard extractors for authentication.
///
/// # How to wire into AppState
///
/// ```rust
/// // In crates/app/src/server/state.rs:
/// use auth::{AuthState, CognitoConfig, JwksCache};
/// use std::sync::Arc;
/// use tokio::sync::RwLock;
///
/// #[derive(Clone)]
/// pub struct AppState {
///     pub cognito: CognitoConfig,
///     pub jwks:    Arc<RwLock<JwksCache>>,
///     // … other fields
/// }
///
/// impl AuthState for AppState {
///     fn jwks_cache(&self)     -> Arc<RwLock<JwksCache>> { self.jwks.clone() }
///     fn cognito_config(&self) -> &CognitoConfig          { &self.cognito }
/// }
/// ```
///
/// # Using in handlers
///
/// ```rust
/// async fn my_handler(CurrentUser(user): CurrentUser) -> impl IntoResponse {
///     format!("Hello {}", user.email)
/// }
///
/// async fn admin_handler(RequireAdmin(user): RequireAdmin) -> impl IntoResponse {
///     format!("Admin panel for {}", user.email)
/// }
/// ```

use std::sync::Arc;

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Json},
};
use tokio::sync::RwLock;
use tracing::debug;

use shared::User;

use crate::{config::CognitoConfig, error::AuthError, jwks::JwksCache, verify::verify_token};

// ── AuthState trait ───────────────────────────────────────────────────────────

/// Implemented by `AppState` to make `CurrentUser` / `RequireAdmin` work as
/// Axum extractors against any concrete state type.
pub trait AuthState: Clone + Send + Sync + 'static {
    fn jwks_cache(&self) -> Arc<RwLock<JwksCache>>;
    fn cognito_config(&self) -> &CognitoConfig;
}

// ── Token helper ──────────────────────────────────────────────────────────────

fn extract_bearer(parts: &Parts) -> Result<String, AuthError> {
    let header = parts
        .headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(AuthError::MissingToken)?;

    header
        .strip_prefix("Bearer ")
        .map(|t| t.to_string())
        .ok_or(AuthError::MissingToken)
}

// ── CurrentUser ───────────────────────────────────────────────────────────────

/// Extractor that requires a valid Cognito JWT.
///
/// Returns `401 Unauthorized` if the token is missing or invalid.
pub struct CurrentUser(pub User);

#[async_trait]
impl<S> FromRequestParts<S> for CurrentUser
where
    S: AuthState,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let token = extract_bearer(parts)?;
        debug!("authenticating request");
        let user = verify_token(&token, &state.jwks_cache(), state.cognito_config()).await?;
        Ok(CurrentUser(user))
    }
}

// ── RequireAdmin ──────────────────────────────────────────────────────────────

/// Extractor that requires a valid JWT **and** membership in the `Admin` group.
///
/// Returns `401` for auth failures, `403 Forbidden` for insufficient permissions.
pub struct RequireAdmin(pub User);

#[async_trait]
impl<S> FromRequestParts<S> for RequireAdmin
where
    S: AuthState,
{
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let CurrentUser(user) = CurrentUser::from_request_parts(parts, state)
            .await
            .map_err(|e| e.into_response())
            .map_err(|_| {
                (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"})))
            })?;

        if !user.is_admin() {
            return Err((
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({"error": "Admin access required"})),
            ));
        }
        Ok(RequireAdmin(user))
    }
}

// ── RequireBotCreation ────────────────────────────────────────────────────────

/// Extractor that requires `CreatingBotAllowed` group (or `Admin`).
pub struct RequireBotCreation(pub User);

#[async_trait]
impl<S> FromRequestParts<S> for RequireBotCreation
where
    S: AuthState,
{
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let CurrentUser(user) = CurrentUser::from_request_parts(parts, state)
            .await
            .map_err(|_| (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Unauthorized"})),
            ))?;

        if !user.can_create_bot() {
            return Err((
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({"error": "Bot creation not permitted for your account"})),
            ));
        }
        Ok(RequireBotCreation(user))
    }
}
