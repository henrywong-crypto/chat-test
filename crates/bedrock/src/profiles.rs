/// Application Inference Profile management.
///
/// # Architecture
/// - `BedrockProfileClient`  — thin wrapper around the AWS Bedrock management SDK.
///   It knows nothing about caching.
/// - `ProfileCache` trait     — DB-backed lookup/store; implemented in `crates/db`.
/// - `InferenceProfileManager` — coordinates the two: check cache → create if missing.

use std::sync::Arc;

use async_trait::async_trait;
use aws_sdk_bedrock::{
    types::{InferenceProfileType, Tag},
    Client as BedrockMgmtClient,
};
use tracing::{debug, info, warn};

use crate::{
    error::BedrockError,
    models::{foundation_model_arn, get_model, InferenceMode, resolve_invoke_target},
};
use shared::InferenceProfile as DomainProfile;

// ── ProfileCache trait ────────────────────────────────────────────────────────

/// Minimal DB interface needed by `InferenceProfileManager`.
/// `crates/db` implements this via `InferenceProfileRepository`.
#[async_trait]
pub trait ProfileCache: Send + Sync {
    /// Look up a cached ARN for `(user_id, model_id)`.
    async fn get(&self, user_id: &str, model_id: &str) -> anyhow::Result<Option<String>>;

    /// Persist a newly created ARN.
    async fn put(
        &self,
        user_id: &str,
        model_id: &str,
        arn: &str,
        region: &str,
    ) -> anyhow::Result<()>;

    /// Return all cached profiles for a user (for the UI profile list).
    async fn list_for_user(&self, user_id: &str) -> anyhow::Result<Vec<DomainProfile>>;

    /// Remove a cached entry (called after successful AWS delete).
    async fn remove(&self, user_id: &str, model_id: &str) -> anyhow::Result<()>;
}

// ── Wire types from AWS SDK response ─────────────────────────────────────────

/// Lightweight profile info from a ListInferenceProfiles call.
#[derive(Debug, Clone)]
pub struct ListedProfile {
    pub profile_arn:  String,
    pub name:         String,
    pub model_id:     String,
    pub status:       String,
}

// ── BedrockProfileClient ──────────────────────────────────────────────────────

/// Thin async wrapper around the Bedrock management SDK for profile CRUD.
#[derive(Clone)]
pub struct BedrockProfileClient {
    client:     BedrockMgmtClient,
    aws_region: String,
}

impl BedrockProfileClient {
    pub fn new(client: BedrockMgmtClient, aws_region: impl Into<String>) -> Self {
        Self { client, aws_region: aws_region.into() }
    }

    /// Create an Application Inference Profile for a (user, model) pair.
    /// Returns the new profile ARN.
    pub async fn create_profile(
        &self,
        user_id:         &str,
        model_id:        &str,
        bedrock_model_id: &str,
    ) -> Result<String, BedrockError> {
        let name      = shared::InferenceProfile::profile_name(user_id, model_id);
        let model_arn = foundation_model_arn(&self.aws_region, bedrock_model_id);

        let tags = shared::InferenceProfile::build_aws_tags(user_id, model_id)
            .into_iter()
            .filter_map(|(k, v)| {
                Tag::builder().key(k).value(v).build().ok()
            })
            .collect::<Vec<_>>();

        info!(name, model_arn, "creating Application Inference Profile");

        let resp = self
            .client
            .create_inference_profile()
            .inference_profile_name(&name)
            .model_source(
                aws_sdk_bedrock::types::InferenceProfileModelSource::CopyFrom(model_arn),
            )
            .set_tags(if tags.is_empty() { None } else { Some(tags) })
            .send()
            .await
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("ThrottlingException") || msg.contains("TooManyRequests") {
                    BedrockError::Throttling
                } else {
                    BedrockError::ProfileError(msg)
                }
            })?;

        let arn = resp.inference_profile_arn;

        info!(arn, "Application Inference Profile created");
        Ok(arn)
    }

    /// List all Application Inference Profiles visible to this AWS account.
    pub async fn list_profiles(&self) -> Result<Vec<ListedProfile>, BedrockError> {
        let mut profiles = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self
                .client
                .list_inference_profiles()
                .r#type_equals(InferenceProfileType::Application);

            if let Some(tok) = next_token.take() {
                req = req.next_token(tok);
            }

            let resp = req
                .send()
                .await
                .map_err(|e| BedrockError::ProfileError(e.to_string()))?;

            for p in resp.inference_profile_summaries.unwrap_or_default() {
                profiles.push(ListedProfile {
                    profile_arn: p.inference_profile_arn,
                    name:        p.inference_profile_name,
                    model_id:    p.models
                        .into_iter()
                        .next()
                        .and_then(|m| m.model_arn)
                        .unwrap_or_default(),
                    status: p.status.as_str().to_string(),
                });
            }

            next_token = resp.next_token;
            if next_token.is_none() { break; }
        }

        Ok(profiles)
    }

    /// Delete an Application Inference Profile by ARN.
    pub async fn delete_profile(&self, arn: &str) -> Result<(), BedrockError> {
        info!(arn, "deleting Application Inference Profile");
        self.client
            .delete_inference_profile()
            .inference_profile_identifier(arn)
            .send()
            .await
            .map_err(|e| BedrockError::ProfileError(e.to_string()))?;
        Ok(())
    }

    /// Fetch details of a single profile by ARN or name.
    pub async fn get_profile(&self, identifier: &str) -> Result<ListedProfile, BedrockError> {
        let resp = self
            .client
            .get_inference_profile()
            .inference_profile_identifier(identifier)
            .send()
            .await
            .map_err(|e| BedrockError::ProfileError(e.to_string()))?;

        Ok(ListedProfile {
            profile_arn: resp.inference_profile_arn,
            name:        resp.inference_profile_name,
            model_id:    resp
                .models
                .into_iter()
                .next()
                .and_then(|m| m.model_arn)
                .unwrap_or_default(),
            status: resp.status.as_str().to_string(),
        })
    }
}

// ── InferenceProfileManager ───────────────────────────────────────────────────

/// Coordinates AWS profile CRUD with the DB cache.
///
/// Inject into `AppState` and call `get_invoke_target()` in every chat handler.
#[derive(Clone)]
pub struct InferenceProfileManager {
    pub aws_client: BedrockProfileClient,
    pub cache:      Arc<dyn ProfileCache>,
    pub mode:       InferenceMode,
}

impl InferenceProfileManager {
    pub fn new(
        aws_client: BedrockProfileClient,
        cache:      Arc<dyn ProfileCache>,
        mode:       InferenceMode,
    ) -> Self {
        Self { aws_client, cache, mode }
    }

    /// Return the string to pass as `model_id` to Bedrock for this user/model pair.
    ///
    /// - `ApplicationProfile` mode: create or retrieve a per-user profile ARN.
    /// - `CrossRegion` / `Direct`: derive the target from the static model registry.
    pub async fn get_invoke_target(
        &self,
        user_id:  &str,
        model_id: &str,
    ) -> Result<String, BedrockError> {
        let model = get_model(model_id)
            .ok_or_else(|| BedrockError::ModelNotFound(model_id.to_string()))?;

        match &self.mode {
            InferenceMode::ApplicationProfile => {
                let arn = self.get_or_create_profile(user_id, model_id).await?;
                Ok(arn)
            }
            other => Ok(resolve_invoke_target(model, None, other)),
        }
    }

    /// Check the DB cache; create an AWS profile if not found.
    pub async fn get_or_create_profile(
        &self,
        user_id:  &str,
        model_id: &str,
    ) -> Result<String, BedrockError> {
        // ── 1. DB cache hit ───────────────────────────────────────────────────
        if let Ok(Some(arn)) = self.cache.get(user_id, model_id).await {
            debug!(user_id, model_id, arn, "inference profile cache hit");
            return Ok(arn);
        }

        // ── 2. Create via AWS API ─────────────────────────────────────────────
        let model = get_model(model_id)
            .ok_or_else(|| BedrockError::ModelNotFound(model_id.to_string()))?;

        let arn = self
            .aws_client
            .create_profile(user_id, model_id, model.bedrock_model_id)
            .await?;

        // ── 3. Persist to cache ───────────────────────────────────────────────
        if let Err(e) = self.cache.put(user_id, model_id, &arn, &self.aws_client.aws_region).await {
            // Non-fatal: next call will re-create.
            warn!(error = %e, "failed to persist inference profile to cache");
        }

        Ok(arn)
    }

    /// Delete a profile from both AWS and the DB cache.
    pub async fn delete_profile(
        &self,
        user_id:  &str,
        model_id: &str,
        arn:      &str,
    ) -> Result<(), BedrockError> {
        self.aws_client.delete_profile(arn).await?;

        if let Err(e) = self.cache.remove(user_id, model_id).await {
            warn!(error = %e, "failed to remove inference profile from cache");
        }
        Ok(())
    }

    /// Reconcile DB cache with the live AWS state for a user.
    /// Useful after manual changes in the AWS console.
    pub async fn sync_profiles(&self, user_id: &str) -> Result<Vec<ListedProfile>, BedrockError> {
        let all = self.aws_client.list_profiles().await?;
        // Filter by user tag embedded in the profile name convention.
        let user_prefix = format!("{}-", shared::InferenceProfile::profile_name(user_id, ""));
        let user_profiles: Vec<_> = all
            .into_iter()
            .filter(|p| p.name.starts_with(&user_prefix.trim_end_matches('-').to_string()))
            .collect();
        info!(user_id, count = user_profiles.len(), "synced inference profiles");
        Ok(user_profiles)
    }
}
