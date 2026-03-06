/// GET /api/conversations
/// GET /api/conversations/:id
/// DELETE /api/conversations/:id
/// PATCH /api/conversations/:id/title

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};

use auth::extractor::CurrentUser;
use shared::api::{ConversationListResponse, UpdateTitleRequest};

use crate::server::state::AppState;

pub async fn list_conversations(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> impl IntoResponse {
    match state.conversations.list_for_user(&user.id).await {
        Ok(convs) => Json(ConversationListResponse {
            conversations: convs,
            next_token: None,
        }).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn get_conversation(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(conv_id): Path<String>,
) -> impl IntoResponse {
    match state.conversations.get(&user.id, &conv_id).await {
        Ok(conv) => Json(conv).into_response(),
        Err(db::DbError::NotFound(_)) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn delete_conversation(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(conv_id): Path<String>,
) -> impl IntoResponse {
    match state.conversations.delete(&user.id, &conv_id).await {
        Ok(())                        => StatusCode::NO_CONTENT.into_response(),
        Err(db::DbError::NotFound(_)) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn update_title(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(conv_id): Path<String>,
    Json(body): Json<UpdateTitleRequest>,
) -> impl IntoResponse {
    match state.conversations.update_title(&user.id, &conv_id, &body.title).await {
        Ok(())                        => StatusCode::NO_CONTENT.into_response(),
        Err(db::DbError::NotFound(_)) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
