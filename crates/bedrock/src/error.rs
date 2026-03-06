use thiserror::Error;

#[derive(Debug, Error)]
pub enum BedrockError {
    #[error("Bedrock SDK error: {0}")]
    Sdk(String),

    #[error("Request throttled — too many requests")]
    Throttling,

    #[error("Model not found in registry: {0}")]
    ModelNotFound(String),

    #[error("Application inference profile error: {0}")]
    ProfileError(String),

    #[error("Message conversion error: {0}")]
    Conversion(String),

    #[error("Profile cache error: {0}")]
    CacheError(String),

    #[error("Stream error: {0}")]
    StreamError(String),
}

impl BedrockError {
    /// Returns true when the error is a transient throttling condition.
    pub fn is_throttling(&self) -> bool {
        matches!(self, Self::Throttling)
    }
}
