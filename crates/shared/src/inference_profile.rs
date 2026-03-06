use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── ProfileStatus ─────────────────────────────────────────────────────────────

/// Lifecycle state of an AWS Application Inference Profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileStatus {
    /// AWS CreateInferenceProfile call has been made; waiting for it to become active.
    Creating,
    /// Profile is ready to accept inference requests.
    Active,
    /// AWS returned an error during creation.
    Failed,
    /// DeleteInferenceProfile has been called; waiting for removal.
    Deleting,
}

impl ProfileStatus {
    pub fn is_usable(&self) -> bool {
        *self == Self::Active
    }
}

// ── InferenceProfile ──────────────────────────────────────────────────────────

/// A per-user, per-model AWS Bedrock Application Inference Profile.
///
/// These profiles enable tag-based cost attribution in AWS Cost Explorer.
/// Each unique (user_id, model_id) pair gets one profile.
///
/// ARN format:
///   `arn:aws:bedrock:<region>:<account>:application-inference-profile/<id>`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceProfile {
    /// Internal DB identifier (same as the ARN suffix or a UUID).
    pub id: String,
    /// Cognito sub of the owning user.
    pub user_id: String,
    /// Internal model identifier (e.g. `"claude-v3.5-sonnet"`).
    pub model_id: String,
    /// Full ARN to pass to `BedrockRuntime::converse`.
    pub profile_arn: String,
    /// AWS region where the profile was created.
    pub region: String,
    pub created_at: DateTime<Utc>,
    pub status: ProfileStatus,
}

impl InferenceProfile {
    /// Construct the tag map used when creating the AWS resource.
    /// These tags power Cost Explorer `user_id` / `model_id` filters.
    pub fn build_aws_tags(user_id: &str, model_id: &str) -> Vec<(String, String)> {
        vec![
            ("user_id".to_string(),       user_id.to_string()),
            ("model_id".to_string(),      model_id.to_string()),
            ("managed_by".to_string(),    "bedrock-rs".to_string()),
        ]
    }

    /// Generate a deterministic, AWS-safe profile name (max 64 chars,
    /// alphanumeric + hyphens only).
    pub fn profile_name(user_id: &str, model_id: &str) -> String {
        let sanitize = |s: &str| -> String {
            s.chars()
                .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
                .collect()
        };
        // Truncate each part to 30 chars so the combined name ≤ 64.
        let u = sanitize(user_id)
            .chars().take(30).collect::<String>();
        let m = sanitize(model_id)
            .chars().take(30).collect::<String>();
        format!("{u}-{m}")
    }
}

// ── Requests ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProfileRequest {
    pub user_id: String,
    pub model_id: String,
    pub region: String,
}
