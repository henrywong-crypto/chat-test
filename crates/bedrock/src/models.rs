/// Model registry: static definitions, capability flags, and pricing.
///
/// Internal model IDs (e.g. `"claude-3-5-sonnet"`) are stable aliases used
/// throughout the app.  `bedrock_model_id` is what the Bedrock API actually
/// wants.

// ── Supporting types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelProvider {
    Anthropic,
    Amazon,
    Meta,
    Mistral,
    DeepSeek,
}

impl ModelProvider {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Anthropic => "Anthropic",
            Self::Amazon    => "Amazon",
            Self::Meta      => "Meta",
            Self::Mistral   => "Mistral",
            Self::DeepSeek  => "DeepSeek",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModelCapabilities {
    pub tool_use:  bool,
    pub vision:    bool,
    pub reasoning: bool,
    pub streaming: bool,
}

#[derive(Debug, Clone)]
pub struct ModelPricing {
    /// USD per 1 000 input tokens.
    pub input_per_1k:        f64,
    /// USD per 1 000 output tokens.
    pub output_per_1k:       f64,
    /// USD per 1 000 tokens written to the prompt cache.
    pub cache_write_per_1k:  f64,
    /// USD per 1 000 tokens read from the prompt cache.
    pub cache_read_per_1k:   f64,
}

#[derive(Debug, Clone)]
pub struct ModelInfo {
    /// Stable internal ID used everywhere in the application.
    pub id: &'static str,
    /// The ID passed directly to the Bedrock Converse API.
    pub bedrock_model_id: &'static str,
    pub display_name: &'static str,
    pub provider: ModelProvider,
    pub capabilities: ModelCapabilities,
    pub pricing: ModelPricing,
    /// Regions where a cross-region inference prefix is available.
    /// Key = prefix (e.g. `"us"`, `"eu"`, `"ap"`), value = list of home regions.
    pub cross_region_prefixes: &'static [(&'static str, &'static [&'static str])],
    /// Whether a Bedrock-managed global inference profile exists for this model.
    pub supports_global_inference: bool,
}

// ── Inference mode ────────────────────────────────────────────────────────────

/// How to resolve the model identifier for a Bedrock Converse call.
#[derive(Debug, Clone)]
pub enum InferenceMode {
    /// Use an Application Inference Profile ARN (per-user tagging).
    ApplicationProfile,
    /// Prepend a regional routing prefix to the base model ID.
    CrossRegion { prefix: String },
    /// Use the raw Bedrock model ID (no cross-region routing).
    Direct,
}

/// Build the string to pass as `model_id` / `model_arn` to Bedrock.
pub fn resolve_invoke_target(
    model: &ModelInfo,
    profile_arn: Option<&str>,
    mode: &InferenceMode,
) -> String {
    match mode {
        InferenceMode::ApplicationProfile => {
            profile_arn
                .unwrap_or(model.bedrock_model_id)
                .to_string()
        }
        InferenceMode::CrossRegion { prefix } => {
            format!("{}.{}", prefix, model.bedrock_model_id)
        }
        InferenceMode::Direct => model.bedrock_model_id.to_string(),
    }
}

// ── Static registry ───────────────────────────────────────────────────────────

macro_rules! caps {
    (tools=$t:expr, vision=$v:expr, reasoning=$r:expr) => {
        ModelCapabilities { tool_use: $t, vision: $v, reasoning: $r, streaming: true }
    };
}

macro_rules! price {
    (in=$i:expr, out=$o:expr) => {
        ModelPricing { input_per_1k: $i, output_per_1k: $o, cache_write_per_1k: 0.0, cache_read_per_1k: 0.0 }
    };
    (in=$i:expr, out=$o:expr, cw=$cw:expr, cr=$cr:expr) => {
        ModelPricing { input_per_1k: $i, output_per_1k: $o, cache_write_per_1k: $cw, cache_read_per_1k: $cr }
    };
}

static CROSS_REGION_US: &[(&str, &[&str])] = &[
    ("us", &["us-east-1", "us-east-2", "us-west-2"]),
];
#[allow(dead_code)]
static CROSS_REGION_EU: &[(&str, &[&str])] = &[
    ("eu", &["eu-west-1", "eu-west-3", "eu-central-1"]),
];
#[allow(dead_code)]
static CROSS_REGION_AP: &[(&str, &[&str])] = &[
    ("ap", &["ap-northeast-1", "ap-southeast-1", "ap-south-1"]),
];
static CROSS_REGION_ALL: &[(&str, &[&str])] = &[
    ("us", &["us-east-1", "us-east-2", "us-west-2"]),
    ("eu", &["eu-west-1", "eu-west-3", "eu-central-1"]),
    ("ap", &["ap-northeast-1", "ap-southeast-1", "ap-south-1"]),
];

static MODELS: &[ModelInfo] = &[
    // ── Anthropic Claude ─────────────────────────────────────────────────────
    ModelInfo {
        id:              "claude-3-5-sonnet-v2",
        bedrock_model_id: "anthropic.claude-3-5-sonnet-20241022-v2:0",
        display_name:    "Claude 3.5 Sonnet",
        provider:        ModelProvider::Anthropic,
        capabilities:    caps!(tools=true,  vision=true,  reasoning=false),
        pricing:         price!(in=0.003, out=0.015, cw=0.00375, cr=0.0003),
        cross_region_prefixes:      CROSS_REGION_ALL,
        supports_global_inference:  true,
    },
    ModelInfo {
        id:              "claude-3-5-haiku",
        bedrock_model_id: "anthropic.claude-3-5-haiku-20241022-v1:0",
        display_name:    "Claude 3.5 Haiku",
        provider:        ModelProvider::Anthropic,
        capabilities:    caps!(tools=true,  vision=true,  reasoning=false),
        pricing:         price!(in=0.0008, out=0.004, cw=0.001, cr=0.00008),
        cross_region_prefixes:      CROSS_REGION_US,
        supports_global_inference:  false,
    },
    ModelInfo {
        id:              "claude-3-opus",
        bedrock_model_id: "anthropic.claude-3-opus-20240229-v1:0",
        display_name:    "Claude 3 Opus",
        provider:        ModelProvider::Anthropic,
        capabilities:    caps!(tools=true,  vision=true,  reasoning=false),
        pricing:         price!(in=0.015, out=0.075, cw=0.01875, cr=0.0015),
        cross_region_prefixes:      CROSS_REGION_US,
        supports_global_inference:  false,
    },
    ModelInfo {
        id:              "claude-3-haiku",
        bedrock_model_id: "anthropic.claude-3-haiku-20240307-v1:0",
        display_name:    "Claude 3 Haiku",
        provider:        ModelProvider::Anthropic,
        capabilities:    caps!(tools=true,  vision=true,  reasoning=false),
        pricing:         price!(in=0.00025, out=0.00125),
        cross_region_prefixes:      CROSS_REGION_ALL,
        supports_global_inference:  false,
    },
    ModelInfo {
        id:              "claude-4-5-sonnet",
        bedrock_model_id: "anthropic.claude-sonnet-4-5-20250929-v1:0",
        display_name:    "Claude Sonnet 4.5",
        provider:        ModelProvider::Anthropic,
        capabilities:    caps!(tools=true,  vision=true,  reasoning=true),
        pricing:         price!(in=0.003, out=0.015, cw=0.00375, cr=0.0003),
        cross_region_prefixes:      CROSS_REGION_ALL,
        supports_global_inference:  true,
    },
    ModelInfo {
        id:              "claude-4-5-opus",
        bedrock_model_id: "anthropic.claude-opus-4-5-20251101-v1:0",
        display_name:    "Claude Opus 4.5",
        provider:        ModelProvider::Anthropic,
        capabilities:    caps!(tools=true,  vision=true,  reasoning=true),
        pricing:         price!(in=0.015, out=0.075, cw=0.01875, cr=0.0015),
        cross_region_prefixes:      CROSS_REGION_ALL,
        supports_global_inference:  true,
    },

    // ── Amazon Nova ──────────────────────────────────────────────────────────
    ModelInfo {
        id:              "nova-pro",
        bedrock_model_id: "amazon.nova-pro-v1:0",
        display_name:    "Amazon Nova Pro",
        provider:        ModelProvider::Amazon,
        capabilities:    caps!(tools=true,  vision=true,  reasoning=false),
        pricing:         price!(in=0.0008, out=0.0032),
        cross_region_prefixes:      CROSS_REGION_US,
        supports_global_inference:  false,
    },
    ModelInfo {
        id:              "nova-lite",
        bedrock_model_id: "amazon.nova-lite-v1:0",
        display_name:    "Amazon Nova Lite",
        provider:        ModelProvider::Amazon,
        capabilities:    caps!(tools=true,  vision=true,  reasoning=false),
        pricing:         price!(in=0.00006, out=0.00024),
        cross_region_prefixes:      CROSS_REGION_US,
        supports_global_inference:  false,
    },
    ModelInfo {
        id:              "nova-micro",
        bedrock_model_id: "amazon.nova-micro-v1:0",
        display_name:    "Amazon Nova Micro",
        provider:        ModelProvider::Amazon,
        capabilities:    caps!(tools=false, vision=false, reasoning=false),
        pricing:         price!(in=0.000035, out=0.00014),
        cross_region_prefixes:      CROSS_REGION_US,
        supports_global_inference:  false,
    },

    // ── Mistral ──────────────────────────────────────────────────────────────
    ModelInfo {
        id:              "mistral-large-2",
        bedrock_model_id: "mistral.mistral-large-2402-v1:0",
        display_name:    "Mistral Large 2",
        provider:        ModelProvider::Mistral,
        capabilities:    caps!(tools=true,  vision=false, reasoning=false),
        pricing:         price!(in=0.003, out=0.009),
        cross_region_prefixes:      &[],
        supports_global_inference:  false,
    },
    ModelInfo {
        id:              "mistral-small",
        bedrock_model_id: "mistral.mistral-small-2402-v1:0",
        display_name:    "Mistral Small",
        provider:        ModelProvider::Mistral,
        capabilities:    caps!(tools=false, vision=false, reasoning=false),
        pricing:         price!(in=0.001, out=0.003),
        cross_region_prefixes:      &[],
        supports_global_inference:  false,
    },

    // ── Meta Llama ───────────────────────────────────────────────────────────
    ModelInfo {
        id:              "llama-3-1-8b",
        bedrock_model_id: "meta.llama3-1-8b-instruct-v1:0",
        display_name:    "Llama 3.1 8B",
        provider:        ModelProvider::Meta,
        capabilities:    caps!(tools=false, vision=false, reasoning=false),
        pricing:         price!(in=0.00022, out=0.00022),
        cross_region_prefixes:      &[],
        supports_global_inference:  false,
    },
    ModelInfo {
        id:              "llama-3-1-70b",
        bedrock_model_id: "meta.llama3-1-70b-instruct-v1:0",
        display_name:    "Llama 3.1 70B",
        provider:        ModelProvider::Meta,
        capabilities:    caps!(tools=false, vision=false, reasoning=false),
        pricing:         price!(in=0.00099, out=0.00099),
        cross_region_prefixes:      &[],
        supports_global_inference:  false,
    },
    ModelInfo {
        id:              "llama-3-3-70b",
        bedrock_model_id: "meta.llama3-3-70b-instruct-v1:0",
        display_name:    "Llama 3.3 70B",
        provider:        ModelProvider::Meta,
        capabilities:    caps!(tools=false, vision=false, reasoning=false),
        pricing:         price!(in=0.00099, out=0.00099),
        cross_region_prefixes:      &[],
        supports_global_inference:  false,
    },
];

// ── Public API ────────────────────────────────────────────────────────────────

pub fn get_model(id: &str) -> Option<&'static ModelInfo> {
    MODELS.iter().find(|m| m.id == id)
}

pub fn list_models() -> &'static [ModelInfo] {
    MODELS
}

/// Build the foundation model ARN used when creating an Application Inference Profile.
/// Format: `arn:aws:bedrock:{region}::foundation-model/{bedrock_model_id}`
pub fn foundation_model_arn(region: &str, bedrock_model_id: &str) -> String {
    format!("arn:aws:bedrock:{}::foundation-model/{}", region, bedrock_model_id)
}
