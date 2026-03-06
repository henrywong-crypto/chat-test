/// AppState — holds all shared server resources injected via Axum `State`.
///
/// Constructed once at startup from environment variables.
/// All inner types are cheap to clone (Arc-backed or Copy).

use std::sync::Arc;

use aws_sdk_cognitoidentityprovider::Client as CognitoClient;
use aws_sdk_bedrockruntime::Client as BedrockRuntimeClient;
use serde::Deserialize;
use tokio::sync::RwLock;

use auth::{AuthState, CognitoConfig, JwksCache};
use bedrock::{
    models::InferenceMode,
    profiles::{BedrockProfileClient, InferenceProfileManager, ProfileCache},
};
use db::{BotRepository, ConversationRepository, InferenceProfileRepository, S3Store};

// ── Config ────────────────────────────────────────────────────────────────────

/// All configuration loaded from environment variables.
#[derive(Debug, Deserialize)]
pub struct Config {
    /// AWS region for all services.
    #[serde(default = "default_region")]
    pub aws_region: String,

    /// Cognito User Pool ID (optional when DEV_AUTH_BYPASS=true).
    #[serde(default)]
    pub cognito_user_pool_id: String,

    /// DynamoDB table names.
    pub conversations_table_name: String,
    pub bots_table_name: String,
    pub inference_profiles_table_name: String,

    /// S3 bucket for large message maps.
    pub large_message_bucket: String,

    /// How to invoke Bedrock models.
    /// `"direct"` | `"cross_region:us"` | `"application_profile"`.
    #[serde(default)]
    pub inference_mode: String,

    /// Default model ID when none is specified in a request.
    #[serde(default = "default_model")]
    pub default_model_id: String,

    /// Override DynamoDB endpoint (for local development).
    pub dynamodb_endpoint: Option<String>,
}

fn default_region() -> String { "us-east-1".into() }
fn default_model()  -> String { "claude-3-5-sonnet-v2".into() }

fn parse_inference_mode(s: &str) -> InferenceMode {
    if let Some(prefix) = s.strip_prefix("cross_region:") {
        return InferenceMode::CrossRegion { prefix: prefix.to_string() };
    }
    match s {
        "application_profile" => InferenceMode::ApplicationProfile,
        _                     => InferenceMode::Direct,
    }
}

// ── AppState ──────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    // ── Auth ──────────────────────────────────────────────────────────────────
    pub cognito: CognitoConfig,
    pub jwks:    Arc<RwLock<JwksCache>>,

    // ── DB ────────────────────────────────────────────────────────────────────
    pub conversations: ConversationRepository,
    pub bots:          BotRepository,
    pub s3:            Arc<S3Store>,

    // ── Bedrock ───────────────────────────────────────────────────────────────
    pub bedrock_runtime:  BedrockRuntimeClient,
    pub profile_manager:  InferenceProfileManager,

    // ── Cognito admin ─────────────────────────────────────────────────────────
    pub cognito_client: CognitoClient,
    pub user_pool_id:   String,

    // ── Runtime config ────────────────────────────────────────────────────────
    pub default_model_id: String,
    pub aws_region:       String,
}

impl AppState {
    pub async fn from_env() -> anyhow::Result<Self> {
        let cfg: Config = envy::from_env()?;

        // ── AWS SDK config ────────────────────────────────────────────────────
        let aws_conf = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(cfg.aws_region.clone()))
            .load()
            .await;

        // DynamoDB (local override for dev).
        let dynamo = {
            let builder = aws_sdk_dynamodb::config::Builder::from(&aws_conf);
            let builder = if let Some(ep) = &cfg.dynamodb_endpoint {
                builder.endpoint_url(ep)
            } else {
                builder
            };
            aws_sdk_dynamodb::Client::from_conf(builder.build())
        };

        let s3              = Arc::new(S3Store::new(aws_sdk_s3::Client::new(&aws_conf), &cfg.large_message_bucket));
        let bedrock_runtime = BedrockRuntimeClient::new(&aws_conf);
        let bedrock_mgmt    = aws_sdk_bedrock::Client::new(&aws_conf);
        let cognito_client  = CognitoClient::new(&aws_conf);

        // ── Repositories ──────────────────────────────────────────────────────
        let conversations = ConversationRepository::new(dynamo.clone(), s3.clone(), &cfg.conversations_table_name);
        let bots          = BotRepository::new(dynamo.clone(), &cfg.bots_table_name);

        let profile_cache = Arc::new(InferenceProfileRepository::new(
            dynamo, &cfg.inference_profiles_table_name,
        )) as Arc<dyn ProfileCache>;

        // ── Bedrock profile manager ───────────────────────────────────────────
        let mode            = parse_inference_mode(&cfg.inference_mode);
        let profile_client  = BedrockProfileClient::new(bedrock_mgmt, &cfg.aws_region);
        let profile_manager = InferenceProfileManager::new(profile_client, profile_cache, mode);

        // ── Auth ──────────────────────────────────────────────────────────────
        let cognito = CognitoConfig::new(&cfg.cognito_user_pool_id, &cfg.aws_region);
        let jwks    = Arc::new(RwLock::new(JwksCache::new()));

        Ok(AppState {
            cognito,
            jwks,
            conversations,
            bots,
            s3,
            bedrock_runtime,
            profile_manager,
            cognito_client,
            user_pool_id:    cfg.cognito_user_pool_id,
            default_model_id: cfg.default_model_id,
            aws_region:       cfg.aws_region,
        })
    }
}

// ── AuthState impl ────────────────────────────────────────────────────────────

impl AuthState for AppState {
    fn jwks_cache(&self)     -> Arc<RwLock<JwksCache>> { self.jwks.clone() }
    fn cognito_config(&self) -> &CognitoConfig          { &self.cognito }
}
