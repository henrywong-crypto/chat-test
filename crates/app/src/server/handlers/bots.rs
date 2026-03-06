/// GET  /api/bots          — list my bots
/// GET  /api/bots/store    — list public bots
/// POST /api/bots          — create bot
/// GET  /api/bots/:id      — get a bot
/// PUT  /api/bots/:id      — update bot
/// DELETE /api/bots/:id    — delete bot

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use chrono::Utc;
use uuid::Uuid;

use auth::extractor::{CurrentUser, RequireBotCreation};
use shared::{
    api::{BotListResponse, CreateBotRequest, UpdateBotRequest},
    Bot, KnowledgeConfig, RetrievalParams,
};

use crate::server::state::AppState;

pub async fn list_my_bots(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> impl IntoResponse {
    match state.bots.list_mine(&user.id).await {
        Ok(bots) => Json(BotListResponse { bots, next_token: None }).into_response(),
        Err(e)   => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn list_public_bots(
    State(state): State<AppState>,
    CurrentUser(_user): CurrentUser,
) -> impl IntoResponse {
    match state.bots.list_public().await {
        Ok(bots) => Json(BotListResponse { bots, next_token: None }).into_response(),
        Err(e)   => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn get_bot(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(bot_id): Path<String>,
) -> impl IntoResponse {
    match state.bots.get(&user.id, &bot_id).await {
        Ok(bot) => Json(bot).into_response(),
        Err(db::DbError::NotFound(_)) => StatusCode::NOT_FOUND.into_response(),
        Err(e)  => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn create_bot(
    State(state): State<AppState>,
    RequireBotCreation(user): RequireBotCreation,
    Json(body): Json<CreateBotRequest>,
) -> impl IntoResponse {
    let knowledge = body.knowledge_base_id.map(|kb_id| KnowledgeConfig {
        knowledge_base_id: kb_id,
        retrieval_params: RetrievalParams::default(),
    });

    let bot = Bot {
        id:                Uuid::new_v4().to_string(),
        owner_user_id:     user.id.clone(),
        title:             body.title,
        description:       body.description,
        instruction:       body.instruction,
        model_id:          body.model_id,
        generation_params: body.generation_params.unwrap_or_default(),
        knowledge,
        visibility:        body.visibility,
        is_starred:        false,
        create_time:       Utc::now().timestamp_millis() as f64 / 1000.0,
        last_used_time:    None,
    };

    match state.bots.put(&bot).await {
        Ok(())  => (StatusCode::CREATED, Json(bot)).into_response(),
        Err(e)  => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn update_bot(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(bot_id): Path<String>,
    Json(body): Json<UpdateBotRequest>,
) -> impl IntoResponse {
    let mut bot = match state.bots.get(&user.id, &bot_id).await {
        Ok(b)                         => b,
        Err(db::DbError::NotFound(_)) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    if !bot.is_owned_by(&user.id) && !user.is_admin() {
        return StatusCode::FORBIDDEN.into_response();
    }

    if let Some(t) = body.title             { bot.title             = t; }
    if let Some(d) = body.description       { bot.description       = d; }
    if let Some(i) = body.instruction       { bot.instruction       = i; }
    if let Some(m) = body.model_id          { bot.model_id          = Some(m); }
    if let Some(p) = body.generation_params { bot.generation_params = p; }
    if let Some(v) = body.visibility        { bot.visibility        = v; }
    if let Some(kb_id) = body.knowledge_base_id {
        bot.knowledge = Some(KnowledgeConfig {
            knowledge_base_id: kb_id,
            retrieval_params: RetrievalParams::default(),
        });
    }

    match state.bots.put(&bot).await {
        Ok(())  => Json(bot).into_response(),
        Err(e)  => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn delete_bot(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(bot_id): Path<String>,
) -> impl IntoResponse {
    // Verify ownership before deleting.
    match state.bots.get(&user.id, &bot_id).await {
        Err(db::DbError::NotFound(_)) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        Ok(bot) if !bot.is_owned_by(&user.id) && !user.is_admin() => {
            return StatusCode::FORBIDDEN.into_response();
        }
        Ok(_) => {}
    }

    match state.bots.delete(&user.id, &bot_id).await {
        Ok(())  => StatusCode::NO_CONTENT.into_response(),
        Err(e)  => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
