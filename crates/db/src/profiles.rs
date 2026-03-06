/// Inference profile repository — implements `bedrock::ProfileCache`.
///
/// # Table design
/// - Table: `INFERENCE_PROFILES_TABLE_NAME`
/// - PK: `userId` (S)  SK: `modelId` (S)
/// - Attributes: `profileArn`, `region`, `createTime`

use std::collections::HashMap;

use async_trait::async_trait;
use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::Client as DynamoClient;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::debug;

use bedrock::profiles::ProfileCache;
use shared::InferenceProfile as DomainProfile;

use crate::marshaling::{from_item, to_item};

// ── DynamoDB record ───────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct ProfileRecord {
    #[serde(rename = "userId")]
    user_id: String,
    #[serde(rename = "modelId")]
    model_id: String,
    #[serde(rename = "profileArn")]
    profile_arn: String,
    region: String,
    /// ISO-8601 timestamp stored as a DynamoDB String attribute.
    #[serde(rename = "createdAt")]
    created_at: String,
}

// ── Repository ────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct InferenceProfileRepository {
    dynamo: DynamoClient,
    table:  String,
}

impl InferenceProfileRepository {
    pub fn new(dynamo: DynamoClient, table: impl Into<String>) -> Self {
        Self { dynamo, table: table.into() }
    }
}

#[async_trait]
impl ProfileCache for InferenceProfileRepository {
    /// Look up a cached ARN for `(user_id, model_id)`.
    async fn get(&self, user_id: &str, model_id: &str) -> anyhow::Result<Option<String>> {
        let resp = self.dynamo
            .get_item()
            .table_name(&self.table)
            .key("userId",  AttributeValue::S(user_id.to_string()))
            .key("modelId", AttributeValue::S(model_id.to_string()))
            .projection_expression("profileArn")
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("DynamoDB error: {e}"))?;

        if let Some(item) = resp.item {
            if let Some(AttributeValue::S(arn)) = item.get("profileArn") {
                debug!(user_id, model_id, arn, "profile cache hit");
                return Ok(Some(arn.clone()));
            }
        }
        Ok(None)
    }

    /// Persist a newly created ARN.
    async fn put(
        &self,
        user_id:  &str,
        model_id: &str,
        arn:      &str,
        region:   &str,
    ) -> anyhow::Result<()> {
        let rec = ProfileRecord {
            user_id:     user_id.to_string(),
            model_id:    model_id.to_string(),
            profile_arn: arn.to_string(),
            region:      region.to_string(),
            created_at:  Utc::now().to_rfc3339(),
        };
        let item = to_item(&rec)
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        self.dynamo
            .put_item()
            .table_name(&self.table)
            .set_item(Some(item))
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("DynamoDB error: {e}"))?;

        debug!(user_id, model_id, arn, "profile cached");
        Ok(())
    }

    /// Return all cached profiles for a user.
    async fn list_for_user(&self, user_id: &str) -> anyhow::Result<Vec<DomainProfile>> {
        let mut profiles = Vec::new();
        let mut last_key: Option<HashMap<String, AttributeValue>> = None;

        loop {
            let mut req = self.dynamo
                .query()
                .table_name(&self.table)
                .key_condition_expression("userId = :uid")
                .expression_attribute_values(":uid", AttributeValue::S(user_id.to_string()));

            if let Some(ek) = last_key.take() {
                req = req.set_exclusive_start_key(Some(ek));
            }

            let resp = req.send().await
                .map_err(|e| anyhow::anyhow!("DynamoDB error: {e}"))?;

            for item in resp.items.unwrap_or_default() {
                let rec: ProfileRecord = from_item(item)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                let created_at = rec.created_at.parse::<DateTime<Utc>>()
                    .unwrap_or_else(|_| Utc::now());
                profiles.push(DomainProfile {
                    id:          format!("{}/{}", rec.user_id, rec.model_id),
                    user_id:     rec.user_id,
                    model_id:    rec.model_id,
                    profile_arn: rec.profile_arn,
                    region:      rec.region,
                    status:      shared::ProfileStatus::Active,
                    created_at,
                });
            }

            last_key = resp.last_evaluated_key;
            if last_key.is_none() { break; }
        }
        Ok(profiles)
    }

    /// Remove a cached entry.
    async fn remove(&self, user_id: &str, model_id: &str) -> anyhow::Result<()> {
        self.dynamo
            .delete_item()
            .table_name(&self.table)
            .key("userId",  AttributeValue::S(user_id.to_string()))
            .key("modelId", AttributeValue::S(model_id.to_string()))
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("DynamoDB error: {e}"))?;
        Ok(())
    }
}
