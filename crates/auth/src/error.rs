use axum::{
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Missing or malformed Authorization header")]
    MissingToken,

    #[error("Token is invalid: {0}")]
    InvalidToken(String),

    #[error("Token has expired")]
    ExpiredToken,

    #[error("Token issuer does not match the configured Cognito pool")]
    InvalidIssuer,

    #[error("token_use claim must be 'access'")]
    InvalidTokenUse,

    #[error("No JWKS key matches the token kid")]
    UnknownKid,

    #[error("Failed to fetch JWKS from Cognito: {0}")]
    JwksFetchError(String),

    #[error("Internal auth error: {0}")]
    Internal(String),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            AuthError::MissingToken
            | AuthError::InvalidToken(_)
            | AuthError::ExpiredToken
            | AuthError::InvalidIssuer
            | AuthError::InvalidTokenUse
            | AuthError::UnknownKid => (StatusCode::UNAUTHORIZED, self.to_string()),

            AuthError::JwksFetchError(_)
            | AuthError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        let body = serde_json::json!({ "error": msg });
        (status, Json(body)).into_response()
    }
}
