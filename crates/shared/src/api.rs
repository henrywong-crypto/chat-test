/// API request and response types shared between the Axum server handlers
/// and the Leptos server functions.
///
/// All types derive `Serialize + Deserialize` so they can be used in both
/// JSON HTTP bodies and Leptos server function codegen.

use serde::{Deserialize, Serialize};

use crate::{
    Bot, BotVisibility, ContentBlock, ConversationMeta, GenerationParams,
    InferenceProfile, TokenUsage, ToolUseContent, UserGroup,
};

// ── Chat ──────────────────────────────────────────────────────────────────────

/// POST /api/chat  (or Leptos server fn `send_message`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    /// One or more content blocks from the user (text, image, …).
    pub content: Vec<ContentBlock>,
    /// Target bot.  `None` = plain model chat with no custom instruction.
    pub bot_id: Option<String>,
    /// Continue an existing conversation.  `None` = start a new one.
    pub conversation_id: Option<String>,
    /// Override the user's default model for this turn.
    pub model_id: Option<String>,
}

/// Individual events streamed back over SSE during generation.
///
/// The client accumulates `Text` deltas into the displayed message,
/// collects any `ToolUse` blocks, then acts on `Done` or `Error`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    /// Incremental text token(s) from the model.
    Text { delta: String },
    /// Model is requesting a tool invocation.
    ToolUse(ToolUseContent),
    /// Generation complete — includes final token usage.
    Done {
        usage: TokenUsage,
        stop_reason: String,
        /// ID assigned to the persisted assistant message.
        message_id: String,
        /// ID of the conversation (useful when one was auto-created).
        conversation_id: String,
    },
    /// A recoverable or fatal error during generation.
    Error { message: String },
}

/// POST /api/upload  (multipart form; field name "file")
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadResponse {
    pub key:          String,
    pub content_type: String,
    pub name:         String,
}

// ── Conversations ─────────────────────────────────────────────────────────────

/// GET /api/conversations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationListResponse {
    pub conversations: Vec<ConversationMeta>,
    /// Pass this in the next request for the following page.
    pub next_token: Option<String>,
}

/// PATCH /api/conversations/:id/title
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTitleRequest {
    pub title: String,
}

// ── Bots ──────────────────────────────────────────────────────────────────────

/// POST /api/bots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBotRequest {
    pub title: String,
    pub description: String,
    pub instruction: String,
    pub model_id: Option<String>,
    pub generation_params: Option<GenerationParams>,
    /// Attach a Bedrock Knowledge Base by ID.
    pub knowledge_base_id: Option<String>,
    pub visibility: BotVisibility,
}

/// PUT /api/bots/:id  — all fields optional; only provided fields are updated.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateBotRequest {
    pub title:             Option<String>,
    pub description:       Option<String>,
    pub instruction:       Option<String>,
    pub model_id:          Option<String>,
    pub generation_params: Option<GenerationParams>,
    pub knowledge_base_id: Option<String>,
    pub visibility:        Option<BotVisibility>,
}

/// GET /api/bots  and  GET /api/bots/store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotListResponse {
    pub bots:       Vec<Bot>,
    pub next_token: Option<String>,
}

// ── Admin ─────────────────────────────────────────────────────────────────────

/// Lightweight user representation for admin list views.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminUserRecord {
    pub id:         String,
    pub email:      String,
    pub groups:     Vec<UserGroup>,
    pub created_at: Option<String>,
    pub enabled:    bool,
}

/// PATCH /api/admin/users/:id/groups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserGroupsRequest {
    pub add_groups:    Vec<UserGroup>,
    pub remove_groups: Vec<UserGroup>,
}

/// GET /api/admin/users
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminUserListResponse {
    pub users:      Vec<AdminUserRecord>,
    pub next_token: Option<String>,
}

/// Single-model usage row in the analytics endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsageRecord {
    pub model_id:      String,
    pub input_tokens:  u64,
    pub output_tokens: u64,
    pub total_cost:    f64,
}

/// Per-user cost row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserUsageRecord {
    pub user_id:     String,
    pub email:       String,
    pub total_cost:  f64,
    pub total_tokens: u64,
}

/// GET /api/admin/analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageAnalyticsResponse {
    pub total_conversations: u64,
    pub total_input_tokens:  u64,
    pub total_output_tokens: u64,
    pub estimated_cost_usd:  f64,
    pub by_model:            Vec<ModelUsageRecord>,
    pub top_users:           Vec<UserUsageRecord>,
}

// ── Models ────────────────────────────────────────────────────────────────────

/// A single model entry returned by `GET /api/models`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id:           String,
    pub display_name: String,
    pub provider:     String,
    pub vision:       bool,
    pub tool_use:     bool,
    pub reasoning:    bool,
}

/// `GET /api/models` response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelListResponse {
    pub models: Vec<ModelInfo>,
}

// ── Inference Profiles ────────────────────────────────────────────────────────

/// POST /api/inference-profiles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInferenceProfileRequest {
    pub model_id: String,
}

/// GET /api/inference-profiles  and  GET /api/admin/inference-profiles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceProfileListResponse {
    pub profiles: Vec<InferenceProfile>,
}
