/// Low-level helpers for DynamoDB attribute marshaling.
///
/// Uses `serde_dynamo` to convert Rust types ↔ DynamoDB `AttributeValue` maps.

use std::collections::HashMap;

use aws_sdk_dynamodb::types::AttributeValue;
use serde::{de::DeserializeOwned, Serialize};

use crate::DbError;

pub type Item = HashMap<String, AttributeValue>;

/// Serialize a Rust value into a DynamoDB item.
pub fn to_item<T: Serialize>(value: &T) -> Result<Item, DbError> {
    serde_dynamo::to_item(value).map_err(|e| DbError::Serde(e.to_string()))
}

/// Deserialize a DynamoDB item into a Rust value.
pub fn from_item<T: DeserializeOwned>(item: Item) -> Result<T, DbError> {
    serde_dynamo::from_item(item).map_err(|e| DbError::Serde(e.to_string()))
}

/// Return the current Unix timestamp as fractional seconds.
pub fn unix_now() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}
