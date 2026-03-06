/// Conversation repository — DynamoDB CRUD with S3 large-object offload.
///
/// # Table design
/// - Table: `CONVERSATIONS_TABLE_NAME`
/// - PK: `userId` (S)  SK: `conversationId` (S)
/// - Attributes: title, createTime (N), totalPrice (N), botId (S?),
///   lastMessageId (S), messageMapJson (S) or s3Key (S) for large maps.
/// - Large message maps (> 300 KB) are stored in S3 and `s3Key` is set.

use std::collections::HashMap;
use std::sync::Arc;

use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::Client as DynamoClient;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use shared::{Conversation, ConversationMeta, Message};

use crate::{
    error::DbError,
    marshaling::{from_item, to_item},
    s3::S3Store,
};

// 300 KB threshold for S3 offload.
const S3_THRESHOLD: usize = 300 * 1024;

// ── DynamoDB record type ──────────────────────────────────────────────────────

/// Internal DynamoDB record for a conversation.
#[derive(Serialize, Deserialize)]
struct ConvRecord {
    #[serde(rename = "userId")]
    user_id: String,
    #[serde(rename = "conversationId")]
    conversation_id: String,
    title: String,
    #[serde(rename = "createTime")]
    create_time: f64,
    #[serde(rename = "totalPrice")]
    total_price: f64,
    #[serde(rename = "botId")]
    bot_id: Option<String>,
    #[serde(rename = "lastMessageId")]
    last_message_id: String,
    /// JSON-encoded message map (absent when offloaded to S3).
    #[serde(rename = "messageMapJson", skip_serializing_if = "Option::is_none")]
    message_map_json: Option<String>,
    /// S3 key for the message map (set when `message_map_json` is absent).
    #[serde(rename = "s3Key", skip_serializing_if = "Option::is_none")]
    s3_key: Option<String>,
}

// ── Repository ────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ConversationRepository {
    dynamo: DynamoClient,
    s3:     Arc<S3Store>,
    table:  String,
}

impl ConversationRepository {
    pub fn new(dynamo: DynamoClient, s3: Arc<S3Store>, table: impl Into<String>) -> Self {
        Self { dynamo, s3, table: table.into() }
    }

    // ── Read operations ───────────────────────────────────────────────────────

    /// Return metadata for all conversations owned by `user_id`,
    /// ordered newest-first.
    pub async fn list_for_user(&self, user_id: &str) -> Result<Vec<ConversationMeta>, DbError> {
        let mut metas = Vec::new();
        let mut last_key: Option<HashMap<String, AttributeValue>> = None;

        loop {
            let mut req = self.dynamo
                .query()
                .table_name(&self.table)
                .key_condition_expression("userId = :uid")
                .expression_attribute_values(":uid", AttributeValue::S(user_id.to_string()))
                // Fetch only the metadata columns (skip the large message map).
                .projection_expression(
                    "userId, conversationId, title, createTime, totalPrice, botId, lastMessageId"
                )
                .scan_index_forward(false); // newest first

            if let Some(ek) = last_key.take() {
                req = req.set_exclusive_start_key(Some(ek));
            }

            let resp = req.send().await.map_err(|e| DbError::Dynamo(e.to_string()))?;

            for item in resp.items.unwrap_or_default() {
                let rec: ConvRecord = from_item(item)?;
                metas.push(conv_record_to_meta(rec));
            }

            last_key = resp.last_evaluated_key;
            if last_key.is_none() { break; }
        }
        Ok(metas)
    }

    /// Fetch a single conversation including its full message map.
    pub async fn get(&self, user_id: &str, conv_id: &str) -> Result<Conversation, DbError> {
        let resp = self.dynamo
            .get_item()
            .table_name(&self.table)
            .key("userId",         AttributeValue::S(user_id.to_string()))
            .key("conversationId", AttributeValue::S(conv_id.to_string()))
            .send()
            .await
            .map_err(|e| DbError::Dynamo(e.to_string()))?;

        let item = resp.item.ok_or_else(|| {
            DbError::NotFound(format!("{}/{}", user_id, conv_id))
        })?;

        let rec: ConvRecord = from_item(item)?;
        self.record_to_conversation(rec).await
    }

    // ── Write operations ──────────────────────────────────────────────────────

    /// Upsert a conversation (create or replace).
    ///
    /// Message maps larger than 300 KB are offloaded to S3.
    pub async fn put(&self, conv: &Conversation) -> Result<(), DbError> {
        let message_map_json = serde_json::to_string(&conv.message_map)?;

        let (message_map_json_field, s3_key_field) =
            if message_map_json.len() > S3_THRESHOLD {
                let key = S3Store::message_map_key(&conv.meta.user_id, &conv.meta.id);
                self.s3.put_bytes(&key, message_map_json.into_bytes()).await?;
                (None, Some(key))
            } else {
                (Some(message_map_json), None)
            };

        let rec = ConvRecord {
            user_id:          conv.meta.user_id.clone(),
            conversation_id:  conv.meta.id.clone(),
            title:            conv.meta.title.clone(),
            create_time:      conv.meta.create_time,
            total_price:      conv.meta.total_price,
            bot_id:           conv.meta.bot_id.clone(),
            last_message_id:  conv.last_message_id.clone(),
            message_map_json: message_map_json_field,
            s3_key:           s3_key_field,
        };

        let item = to_item(&rec)?;
        self.dynamo
            .put_item()
            .table_name(&self.table)
            .set_item(Some(item))
            .send()
            .await
            .map_err(|e| DbError::Dynamo(e.to_string()))?;

        debug!(user_id = conv.meta.user_id, conv_id = conv.meta.id, "conversation saved");
        Ok(())
    }

    /// Update only the title of an existing conversation.
    pub async fn update_title(
        &self,
        user_id: &str,
        conv_id: &str,
        title:   &str,
    ) -> Result<(), DbError> {
        self.dynamo
            .update_item()
            .table_name(&self.table)
            .key("userId",         AttributeValue::S(user_id.to_string()))
            .key("conversationId", AttributeValue::S(conv_id.to_string()))
            .update_expression("SET title = :t")
            .expression_attribute_values(":t", AttributeValue::S(title.to_string()))
            .send()
            .await
            .map_err(|e| DbError::Dynamo(e.to_string()))?;
        Ok(())
    }

    /// Delete a conversation and its S3 message map (if any).
    pub async fn delete(&self, user_id: &str, conv_id: &str) -> Result<(), DbError> {
        // Best-effort S3 cleanup first.
        let s3_key = S3Store::message_map_key(user_id, conv_id);
        self.s3.delete(&s3_key).await;

        self.dynamo
            .delete_item()
            .table_name(&self.table)
            .key("userId",         AttributeValue::S(user_id.to_string()))
            .key("conversationId", AttributeValue::S(conv_id.to_string()))
            .send()
            .await
            .map_err(|e| DbError::Dynamo(e.to_string()))?;
        Ok(())
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    async fn record_to_conversation(&self, rec: ConvRecord) -> Result<Conversation, DbError> {
        let message_map_json = match (rec.message_map_json.as_deref(), rec.s3_key.as_deref()) {
            (Some(json), _) => json.to_string(),
            (None, Some(key)) => {
                let bytes = self.s3.get_bytes(key).await?;
                String::from_utf8(bytes)
                    .map_err(|e| DbError::Serde(e.to_string()))?
            }
            (None, None) => {
                warn!(conv_id = rec.conversation_id, "conversation has no message data");
                "{}".to_string()
            }
        };

        let message_map: HashMap<String, Message> = serde_json::from_str(&message_map_json)?;
        let last_message_id = rec.last_message_id.clone();
        let meta = conv_record_to_meta(rec);

        Ok(Conversation { meta, last_message_id, message_map })
    }
}

fn conv_record_to_meta(rec: ConvRecord) -> ConversationMeta {
    ConversationMeta {
        id:           rec.conversation_id,
        title:        rec.title,
        create_time:  rec.create_time,
        total_price:  rec.total_price,
        bot_id:       rec.bot_id,
        user_id:      rec.user_id,
    }
}
