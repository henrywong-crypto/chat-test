/// Bot repository — DynamoDB CRUD for custom bot definitions.
///
/// # Table design
/// - Table: `BOTS_TABLE_NAME`
/// - PK: `userId` (S, = owner_user_id)  SK: `botId` (S)
/// - GSI `visibility-index`: PK `visibility` (S), SK `createTime` (N)
///   Used to list all public bots sorted by creation time.

use std::collections::HashMap;

use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::Client as DynamoClient;
use serde::{Deserialize, Serialize};
use tracing::debug;

use shared::{Bot, BotVisibility, GenerationParams, KnowledgeConfig};

use crate::{
    error::DbError,
    marshaling::{from_item, to_item},
};

// ── DynamoDB record type ──────────────────────────────────────────────────────

/// Flattened DynamoDB record for a Bot.
/// Mirrors `shared::Bot` but uses DynamoDB-friendly key names.
#[derive(Serialize, Deserialize)]
struct BotRecord {
    /// DynamoDB partition key (= ownerUserId).
    #[serde(rename = "userId")]
    user_id: String,
    #[serde(rename = "botId")]
    bot_id: String,
    title: String,
    description: String,
    instruction: String,
    #[serde(rename = "modelId")]
    model_id: Option<String>,
    #[serde(rename = "generationParams")]
    generation_params: GenerationParams,
    knowledge: Option<KnowledgeConfig>,
    visibility: BotVisibility,
    #[serde(rename = "isStarred")]
    is_starred: bool,
    #[serde(rename = "createTime")]
    create_time: f64,
    #[serde(rename = "lastUsedTime")]
    last_used_time: Option<f64>,
}

impl From<BotRecord> for Bot {
    fn from(r: BotRecord) -> Self {
        Bot {
            id:                r.bot_id,
            owner_user_id:     r.user_id,
            title:             r.title,
            description:       r.description,
            instruction:       r.instruction,
            model_id:          r.model_id,
            generation_params: r.generation_params,
            knowledge:         r.knowledge,
            visibility:        r.visibility,
            is_starred:        r.is_starred,
            create_time:       r.create_time,
            last_used_time:    r.last_used_time,
        }
    }
}

impl From<&Bot> for BotRecord {
    fn from(b: &Bot) -> Self {
        BotRecord {
            user_id:           b.owner_user_id.clone(),
            bot_id:            b.id.clone(),
            title:             b.title.clone(),
            description:       b.description.clone(),
            instruction:       b.instruction.clone(),
            model_id:          b.model_id.clone(),
            generation_params: b.generation_params.clone(),
            knowledge:         b.knowledge.clone(),
            visibility:        b.visibility.clone(),
            is_starred:        b.is_starred,
            create_time:       b.create_time,
            last_used_time:    b.last_used_time,
        }
    }
}

// ── Repository ────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct BotRepository {
    dynamo: DynamoClient,
    table:  String,
}

impl BotRepository {
    pub fn new(dynamo: DynamoClient, table: impl Into<String>) -> Self {
        Self { dynamo, table: table.into() }
    }

    // ── Read ──────────────────────────────────────────────────────────────────

    /// Fetch a single bot by owner + id.
    pub async fn get(&self, owner_id: &str, bot_id: &str) -> Result<Bot, DbError> {
        let resp = self.dynamo
            .get_item()
            .table_name(&self.table)
            .key("userId", AttributeValue::S(owner_id.to_string()))
            .key("botId",  AttributeValue::S(bot_id.to_string()))
            .send()
            .await
            .map_err(|e| DbError::Dynamo(e.to_string()))?;

        let item = resp.item.ok_or_else(|| {
            DbError::NotFound(format!("bot {}/{}", owner_id, bot_id))
        })?;

        Ok(Bot::from(from_item::<BotRecord>(item)?))
    }

    /// List all bots owned by `owner_id`.
    pub async fn list_mine(&self, owner_id: &str) -> Result<Vec<Bot>, DbError> {
        let mut bots = Vec::new();
        let mut last_key: Option<HashMap<String, AttributeValue>> = None;

        loop {
            let mut req = self.dynamo
                .query()
                .table_name(&self.table)
                .key_condition_expression("userId = :uid")
                .expression_attribute_values(":uid", AttributeValue::S(owner_id.to_string()));

            if let Some(ek) = last_key.take() {
                req = req.set_exclusive_start_key(Some(ek));
            }

            let resp = req.send().await.map_err(|e| DbError::Dynamo(e.to_string()))?;

            for item in resp.items.unwrap_or_default() {
                bots.push(Bot::from(from_item::<BotRecord>(item)?));
            }

            last_key = resp.last_evaluated_key;
            if last_key.is_none() { break; }
        }
        Ok(bots)
    }

    /// List all publicly visible bots via the `visibility-index` GSI.
    pub async fn list_public(&self) -> Result<Vec<Bot>, DbError> {
        let mut bots = Vec::new();
        let mut last_key: Option<HashMap<String, AttributeValue>> = None;

        loop {
            let mut req = self.dynamo
                .query()
                .table_name(&self.table)
                .index_name("visibility-index")
                .key_condition_expression("visibility = :v")
                .expression_attribute_values(
                    ":v",
                    AttributeValue::S("public".to_string()),
                )
                .scan_index_forward(false); // newest first

            if let Some(ek) = last_key.take() {
                req = req.set_exclusive_start_key(Some(ek));
            }

            let resp = req.send().await.map_err(|e| DbError::Dynamo(e.to_string()))?;

            for item in resp.items.unwrap_or_default() {
                bots.push(Bot::from(from_item::<BotRecord>(item)?));
            }

            last_key = resp.last_evaluated_key;
            if last_key.is_none() { break; }
        }
        Ok(bots)
    }

    // ── Write ─────────────────────────────────────────────────────────────────

    /// Upsert a bot.
    pub async fn put(&self, bot: &Bot) -> Result<(), DbError> {
        let rec = BotRecord::from(bot);
        let item = to_item(&rec)?;
        self.dynamo
            .put_item()
            .table_name(&self.table)
            .set_item(Some(item))
            .send()
            .await
            .map_err(|e| DbError::Dynamo(e.to_string()))?;
        debug!(owner = bot.owner_user_id, bot_id = bot.id, "bot saved");
        Ok(())
    }

    /// Delete a bot.
    pub async fn delete(&self, owner_id: &str, bot_id: &str) -> Result<(), DbError> {
        self.dynamo
            .delete_item()
            .table_name(&self.table)
            .key("userId", AttributeValue::S(owner_id.to_string()))
            .key("botId",  AttributeValue::S(bot_id.to_string()))
            .send()
            .await
            .map_err(|e| DbError::Dynamo(e.to_string()))?;
        Ok(())
    }
}
