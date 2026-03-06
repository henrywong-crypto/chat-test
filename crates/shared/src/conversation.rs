use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

// ── Role ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
}

// ── Content blocks ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContent {
    pub body: String,
}

/// Base-64 encoded image bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageContent {
    /// MIME type, e.g. `"image/jpeg"`.
    pub media_type: String,
    /// Base-64 encoded image data.
    pub data: String,
}

/// A model requesting a tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUseContent {
    pub tool_use_id: String,
    pub name: String,
    /// Arbitrary JSON input chosen by the model.
    pub input: JsonValue,
}

/// Application returning the result of a tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultContent {
    pub tool_use_id: String,
    pub content: Vec<ContentBlock>,
    pub is_error: bool,
}

/// A reasoning / extended-thinking block produced by Claude models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningContent {
    /// Thinking text (may be hidden from the UI depending on config).
    pub thinking: String,
    /// Opaque signature returned by Bedrock for round-trip validation.
    pub signature: Option<String>,
}

/// Discriminated union of all content types a message may contain.
/// The `type` field is used for serde tagging so the JSON shape matches
/// the Bedrock Converse API conventions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text(TextContent),
    Image(ImageContent),
    ToolUse(ToolUseContent),
    ToolResult(ToolResultContent),
    Reasoning(ReasoningContent),
}

impl ContentBlock {
    /// Convenience constructor for plain text.
    pub fn text(body: impl Into<String>) -> Self {
        Self::Text(TextContent { body: body.into() })
    }

    /// Return the text body if this is a `Text` block.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(t) => Some(&t.body),
            _             => None,
        }
    }
}

// ── Feedback ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackType {
    Good,
    Bad,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feedback {
    pub thumbs: FeedbackType,
    pub detail: Option<String>,
}

// ── Token usage ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    /// Tokens served from the prompt cache (billed at reduced rate).
    pub cache_read_tokens: u32,
    /// Tokens written into the prompt cache (billed at higher write rate).
    pub cache_write_tokens: u32,
}

impl TokenUsage {
    pub fn total(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }
}

// ── RAG citation ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsedChunk {
    /// Excerpt of the retrieved document.
    pub content: String,
    /// Human-readable source name (filename, URL, etc.).
    pub source: String,
    /// Position in the ranked retrieval list (0-based).
    pub rank: u32,
    /// Optional deep-link to the original document.
    pub source_link: Option<String>,
    /// Bedrock Knowledge Base data source ID.
    pub data_source_id: String,
}

// ── Message ───────────────────────────────────────────────────────────────────

/// A single message in a conversation.
///
/// Messages form a tree (not a list) to support branching / editing.
/// Traverse from `Conversation::last_message_id` following
/// `parent_message_id` to reconstruct the active thread.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub role: Role,
    pub content: Vec<ContentBlock>,
    /// ID of the parent message (None for the root / "system" sentinel).
    pub parent_message_id: Option<String>,
    /// IDs of child messages (multiple = branching).
    pub children_message_ids: Vec<String>,
    /// Unix timestamp in seconds (fractional).
    pub create_time: f64,
    pub feedback: Option<Feedback>,
    /// RAG citations attached to an assistant message.
    pub used_chunks: Vec<UsedChunk>,
    /// Model that produced this message (None for user messages).
    pub model: Option<String>,
    pub token_usage: Option<TokenUsage>,
}

impl Message {
    /// Collect all plain-text parts joined by newlines.
    pub fn text_content(&self) -> String {
        self.content
            .iter()
            .filter_map(|b| b.as_text())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

// ── Conversation ──────────────────────────────────────────────────────────────

/// Lightweight metadata returned in list endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMeta {
    pub id: String,
    pub title: String,
    /// Unix timestamp when the conversation was created.
    pub create_time: f64,
    /// Accumulated cost in USD across all messages.
    pub total_price: f64,
    /// Bot used for this conversation, if any.
    pub bot_id: Option<String>,
    pub user_id: String,
}

/// Full conversation including every message.
///
/// `message_map` keys are message IDs.  Follow the tree from
/// `last_message_id` → `parent_message_id` to get the active thread.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    #[serde(flatten)]
    pub meta: ConversationMeta,
    /// The newest message ID in the active thread.
    pub last_message_id: String,
    pub message_map: HashMap<String, Message>,
}

impl Conversation {
    /// Walk `parent_message_id` from `last_message_id`, returning
    /// the ordered message chain oldest-first.
    pub fn active_thread(&self) -> Vec<&Message> {
        let mut chain = Vec::new();
        let mut cur_id = Some(self.last_message_id.as_str());

        while let Some(id) = cur_id {
            match self.message_map.get(id) {
                Some(msg) => {
                    chain.push(msg);
                    cur_id = msg.parent_message_id.as_deref();
                }
                None => break,
            }
        }
        chain.reverse();
        chain
    }
}
