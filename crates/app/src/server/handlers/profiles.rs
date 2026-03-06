/// GET    /api/inference-profiles          — list my profiles
/// POST   /api/inference-profiles          — create profile for a model
/// DELETE /api/inference-profiles/:model_id — delete profile

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};

use auth::extractor::CurrentUser;
use shared::api::{CreateInferenceProfileRequest, InferenceProfileListResponse};

use crate::server::state::AppState;

pub async fn list_profiles(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> impl IntoResponse {
    // The profile manager's cache holds all profiles we've created for this user.
    // We query the DB directly for the full list.
    match state.profile_manager.cache.list_for_user(&user.id).await {
        Ok(profiles) => Json(InferenceProfileListResponse { profiles }).into_response(),
        Err(e)       => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn create_profile(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<CreateInferenceProfileRequest>,
) -> impl IntoResponse {
    match state.profile_manager.get_or_create_profile(&user.id, &body.model_id).await {
        Ok(arn)  => Json(serde_json::json!({"arn": arn})).into_response(),
        Err(bedrock::error::BedrockError::ModelNotFound(m)) => {
            (StatusCode::BAD_REQUEST, format!("Unknown model: {m}")).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn delete_profile(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(model_id): Path<String>,
) -> impl IntoResponse {
    // Look up the ARN from cache first.
    let arn = match state.profile_manager.cache.get(&user.id, &model_id).await {
        Ok(Some(arn)) => arn,
        Ok(None)      => return StatusCode::NOT_FOUND.into_response(),
        Err(e)        => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    match state.profile_manager.delete_profile(&user.id, &model_id, &arn).await {
        Ok(())  => StatusCode::NO_CONTENT.into_response(),
        Err(e)  => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
