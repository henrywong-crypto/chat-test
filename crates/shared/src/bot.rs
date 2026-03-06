use serde::{Deserialize, Serialize};

// ── GenerationParams ──────────────────────────────────────────────────────────

/// Inference parameters forwarded to the Bedrock Converse API.
/// All fields are optional-ish; sensible defaults are provided.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationParams {
    /// Hard cap on generated tokens.
    pub max_tokens: u32,
    /// Controls randomness.  Range: 0.0–1.0.
    pub temperature: f32,
    /// Nucleus sampling threshold.  Range: 0.0–1.0.
    pub top_p: f32,
    /// Top-k sampling (not supported by all models).
    pub top_k: Option<u32>,
    /// Sequences that halt generation when emitted.
    pub stop_sequences: Vec<String>,
    /// Budget (tokens) for Claude extended-thinking / reasoning.
    /// `None` disables reasoning mode.
    pub reasoning_budget_tokens: Option<u32>,
}

impl Default for GenerationParams {
    fn default() -> Self {
        Self {
            max_tokens:               4096,
            temperature:              0.7,
            top_p:                    0.9,
            top_k:                    None,
            stop_sequences:           vec![],
            reasoning_budget_tokens:  None,
        }
    }
}

impl GenerationParams {
    /// "Creative" preset — high temperature.
    pub fn creative() -> Self {
        Self { temperature: 0.9, top_p: 0.95, ..Self::default() }
    }

    /// "Precise" preset — low temperature.
    pub fn precise() -> Self {
        Self { temperature: 0.1, top_p: 0.8, ..Self::default() }
    }
}

// ── Bot visibility ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BotVisibility {
    /// Only the owner can see and use this bot.
    Private,
    /// Anyone with the link can use it; not listed in the store.
    Unlisted,
    /// Listed in the public bot store.
    Public,
}

// ── Knowledge base ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchType {
    Semantic,
    Hybrid,
    Keyword,
}

impl Default for SearchType {
    fn default() -> Self { Self::Hybrid }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalParams {
    pub max_results: u32,
    pub search_type: SearchType,
}

impl Default for RetrievalParams {
    fn default() -> Self {
        Self { max_results: 5, search_type: SearchType::default() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeConfig {
    pub knowledge_base_id: String,
    pub retrieval_params: RetrievalParams,
}

// ── Bot ───────────────────────────────────────────────────────────────────────

/// A custom assistant bot definition.
///
/// Bots are owned by a user but may be shared/published.  When a user
/// starts a conversation with a bot the bot's `instruction` is prepended
/// as a system message and the `generation_params` override the defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bot {
    pub id: String,
    pub owner_user_id: String,
    pub title: String,
    pub description: String,
    /// System prompt / instruction shown to the model before user messages.
    pub instruction: String,
    /// Override model for this bot; `None` = user's default model.
    pub model_id: Option<String>,
    pub generation_params: GenerationParams,
    /// Attach a Bedrock Knowledge Base for RAG.
    pub knowledge: Option<KnowledgeConfig>,
    pub visibility: BotVisibility,
    /// Whether the current user has starred/pinned this bot.
    pub is_starred: bool,
    pub create_time: f64,
    pub last_used_time: Option<f64>,
}

impl Bot {
    pub fn is_public(&self) -> bool {
        self.visibility == BotVisibility::Public
    }

    pub fn is_owned_by(&self, user_id: &str) -> bool {
        self.owner_user_id == user_id
    }
}
