use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("DynamoDB error: {0}")]
    Dynamo(String),

    #[error("S3 error: {0}")]
    S3(String),

    #[error("Serialization error: {0}")]
    Serde(String),

    #[error("Item not found: {0}")]
    NotFound(String),
}

impl From<serde_json::Error> for DbError {
    fn from(e: serde_json::Error) -> Self {
        DbError::Serde(e.to_string())
    }
}
