/// S3 store for large message maps (> 300 KB) offloaded from DynamoDB.

use aws_sdk_s3::Client as S3Client;
use bytes::Bytes;
use tracing::{debug, warn};

use crate::DbError;

#[derive(Clone)]
pub struct S3Store {
    client: S3Client,
    bucket: String,
}

impl S3Store {
    pub fn new(client: S3Client, bucket: impl Into<String>) -> Self {
        Self { client, bucket: bucket.into() }
    }

    /// Upload raw bytes to `s3://{bucket}/{key}`.
    pub async fn put_bytes(&self, key: &str, data: Vec<u8>) -> Result<(), DbError> {
        debug!(key, bytes = data.len(), "S3 put");
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(Bytes::from(data).into())
            .content_type("application/json")
            .send()
            .await
            .map_err(|e| DbError::S3(e.to_string()))?;
        Ok(())
    }

    /// Download and return the raw bytes from `s3://{bucket}/{key}`.
    pub async fn get_bytes(&self, key: &str) -> Result<Vec<u8>, DbError> {
        debug!(key, "S3 get");
        let resp = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| DbError::S3(e.to_string()))?;

        let bytes = resp
            .body
            .collect()
            .await
            .map_err(|e| DbError::S3(e.to_string()))?
            .into_bytes();

        Ok(bytes.to_vec())
    }

    /// Delete `s3://{bucket}/{key}` (best-effort — logs but does not fail).
    pub async fn delete(&self, key: &str) {
        debug!(key, "S3 delete");
        if let Err(e) = self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            warn!(key, error = %e, "S3 delete failed");
        }
    }

    /// Build the S3 key for a conversation's message map.
    pub fn message_map_key(user_id: &str, conv_id: &str) -> String {
        format!("{user_id}/{conv_id}/messages.json")
    }
}
