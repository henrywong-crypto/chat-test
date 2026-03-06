/// Convert domain types to AWS Bedrock Converse API types, and handle
/// both streaming and non-streaming invocations.

use std::time::Duration;

use aws_sdk_bedrockruntime::{
    operation::converse::ConverseOutput,
    types::{
        ContentBlock as AwsContentBlock, ConversationRole, ImageBlock, ImageFormat, ImageSource,
        InferenceConfiguration, Message as AwsMessage, SystemContentBlock,
        ToolResultBlock, ToolResultContentBlock, ToolResultStatus, ToolUseBlock,
    },
    Client as BedrockRuntimeClient,
};
use aws_smithy_types::Document;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use tracing::{debug, warn};

use shared::{ContentBlock, GenerationParams, Message, Role, TokenUsage, ToolUseContent};

use crate::error::BedrockError;

// ── serde_json::Value → aws_smithy_types::Document ───────────────────────────

pub fn json_to_document(v: serde_json::Value) -> Document {
    match v {
        serde_json::Value::Null          => Document::Null,
        serde_json::Value::Bool(b)       => Document::Bool(b),
        serde_json::Value::String(s)     => Document::String(s),
        serde_json::Value::Number(n) => {
            if let Some(u) = n.as_u64() {
                Document::Number(aws_smithy_types::Number::PosInt(u))
            } else if let Some(i) = n.as_i64() {
                Document::Number(aws_smithy_types::Number::NegInt(i))
            } else {
                Document::Number(aws_smithy_types::Number::Float(n.as_f64().unwrap_or(0.0)))
            }
        }
        serde_json::Value::Array(arr) => {
            Document::Array(arr.into_iter().map(json_to_document).collect())
        }
        serde_json::Value::Object(obj) => {
            Document::Object(obj.into_iter().map(|(k, v)| (k, json_to_document(v))).collect())
        }
    }
}

// ── ContentBlock conversion ───────────────────────────────────────────────────

fn convert_content_block(block: &ContentBlock) -> Result<AwsContentBlock, BedrockError> {
    match block {
        ContentBlock::Text(t) => Ok(AwsContentBlock::Text(t.body.clone())),

        ContentBlock::Image(img) => {
            let bytes = BASE64
                .decode(&img.data)
                .map_err(|e| BedrockError::Conversion(format!("base64 decode: {e}")))?;

            let format = match img.media_type.as_str() {
                "image/jpeg" | "image/jpg" => ImageFormat::Jpeg,
                "image/png"                => ImageFormat::Png,
                "image/gif"                => ImageFormat::Gif,
                "image/webp"               => ImageFormat::Webp,
                other => return Err(BedrockError::Conversion(format!("unsupported image type: {other}"))),
            };

            let source = ImageSource::Bytes(
                aws_sdk_bedrockruntime::primitives::Blob::new(bytes),
            );

            let image_block = ImageBlock::builder()
                .format(format)
                .source(source)
                .build()
                .map_err(|e| BedrockError::Conversion(e.to_string()))?;

            Ok(AwsContentBlock::Image(image_block))
        }

        ContentBlock::ToolUse(t) => {
            let tool_block = ToolUseBlock::builder()
                .tool_use_id(&t.tool_use_id)
                .name(&t.name)
                .input(json_to_document(t.input.clone()))
                .build()
                .map_err(|e| BedrockError::Conversion(e.to_string()))?;

            Ok(AwsContentBlock::ToolUse(tool_block))
        }

        ContentBlock::ToolResult(tr) => {
            let content: Vec<ToolResultContentBlock> = tr
                .content
                .iter()
                .filter_map(|b| match b {
                    ContentBlock::Text(t) => {
                        Some(ToolResultContentBlock::Text(t.body.clone()))
                    }
                    _ => {
                        warn!("ignoring non-text content in ToolResult");
                        None
                    }
                })
                .collect();

            let status = if tr.is_error {
                ToolResultStatus::Error
            } else {
                ToolResultStatus::Success
            };

            let result_block = ToolResultBlock::builder()
                .tool_use_id(&tr.tool_use_id)
                .set_content(Some(content))
                .status(status)
                .build()
                .map_err(|e| BedrockError::Conversion(e.to_string()))?;

            Ok(AwsContentBlock::ToolResult(result_block))
        }

        ContentBlock::Reasoning(_) => {
            // Reasoning blocks are model-generated; we don't send them back.
            Err(BedrockError::Conversion("cannot send Reasoning block to model".into()))
        }
    }
}

fn domain_role(role: &Role) -> ConversationRole {
    match role {
        Role::User      => ConversationRole::User,
        Role::Assistant => ConversationRole::Assistant,
        Role::System    => ConversationRole::User, // System handled separately
    }
}

/// Convert a slice of domain messages into AWS SDK messages.
/// System-role messages are skipped (handled via `system` parameter).
pub fn convert_messages(messages: &[Message]) -> Result<Vec<AwsMessage>, BedrockError> {
    let mut out = Vec::with_capacity(messages.len());
    for msg in messages {
        if msg.role == Role::System { continue; }

        let content: Vec<AwsContentBlock> = msg
            .content
            .iter()
            .filter_map(|b| {
                convert_content_block(b)
                    .map_err(|e| warn!("skipping unconvertible block: {e}"))
                    .ok()
            })
            .collect();

        if content.is_empty() { continue; }

        let aws_msg = AwsMessage::builder()
            .role(domain_role(&msg.role))
            .set_content(Some(content))
            .build()
            .map_err(|e| BedrockError::Conversion(e.to_string()))?;

        out.push(aws_msg);
    }
    Ok(out)
}

/// Build the system content blocks from a bot instruction string.
pub fn build_system_blocks(instruction: Option<&str>) -> Vec<SystemContentBlock> {
    match instruction {
        Some(s) if !s.is_empty() => vec![SystemContentBlock::Text(s.to_string())],
        _ => vec![],
    }
}

/// Convert domain `GenerationParams` to `InferenceConfiguration`.
pub fn build_inference_config(params: &GenerationParams) -> InferenceConfiguration {
    let mut b = InferenceConfiguration::builder()
        .max_tokens(params.max_tokens as i32)
        .temperature(params.temperature);

    if !params.stop_sequences.is_empty() {
        b = b.set_stop_sequences(Some(params.stop_sequences.clone()));
    }

    b.build()
}

// ── Non-streaming invocation ──────────────────────────────────────────────────

pub struct ConverseRequest {
    pub model_arn:   String,
    pub messages:    Vec<AwsMessage>,
    pub system:      Vec<SystemContentBlock>,
    pub inf_config:  InferenceConfiguration,
}

pub struct ConverseResult {
    pub content: Vec<ContentBlock>,
    pub usage:   TokenUsage,
    pub stop_reason: String,
}

/// Invoke the model and wait for the full response.
///
/// Retries up to 5 times with exponential backoff on throttling.
pub async fn invoke_model(
    client: &BedrockRuntimeClient,
    req:    ConverseRequest,
) -> Result<ConverseResult, BedrockError> {
    let max_attempts = 5u32;
    let mut attempt  = 0u32;

    loop {
        let result = client
            .converse()
            .model_id(&req.model_arn)
            .set_messages(Some(req.messages.clone()))
            .set_system(if req.system.is_empty() { None } else { Some(req.system.clone()) })
            .inference_config(req.inf_config.clone())
            .send()
            .await;

        match result {
            Ok(resp) => return parse_converse_output(resp),
            Err(e) => {
                let msg = e.to_string();
                let is_throttle =
                    msg.contains("ThrottlingException") || msg.contains("TooManyRequests");

                if is_throttle && attempt < max_attempts {
                    attempt += 1;
                    let delay = Duration::from_millis(500 * (1u64 << attempt));
                    debug!(attempt, delay_ms = delay.as_millis(), "throttled — backing off");
                    tokio::time::sleep(delay).await;
                } else if is_throttle {
                    return Err(BedrockError::Throttling);
                } else {
                    return Err(BedrockError::Sdk(msg));
                }
            }
        }
    }
}

fn parse_converse_output(resp: ConverseOutput) -> Result<ConverseResult, BedrockError> {
    use aws_sdk_bedrockruntime::types::ConverseOutput as OutputEnum;

    // Extract assistant message content.
    let content = match resp.output {
        Some(OutputEnum::Message(msg)) => {
            msg.content
                .into_iter()
                .filter_map(aws_to_domain_block)
                .collect()
        }
        _ => vec![],
    };

    let usage = resp
        .usage
        .map(|u| TokenUsage {
            input_tokens:       u.input_tokens  as u32,
            output_tokens:      u.output_tokens as u32,
            cache_read_tokens:  u.cache_read_input_tokens.unwrap_or(0) as u32,
            cache_write_tokens: u.cache_write_input_tokens.unwrap_or(0) as u32,
        })
        .unwrap_or_default();

    let stop_reason = resp.stop_reason.as_str().to_string();

    Ok(ConverseResult { content, usage, stop_reason })
}

// ── AWS → domain content blocks ───────────────────────────────────────────────

pub fn aws_to_domain_block(block: AwsContentBlock) -> Option<ContentBlock> {
    match block {
        AwsContentBlock::Text(t) => Some(ContentBlock::text(t)),

        AwsContentBlock::ToolUse(tu) => {
            let input_val = document_to_json(tu.input);
            Some(ContentBlock::ToolUse(ToolUseContent {
                tool_use_id: tu.tool_use_id,
                name:        tu.name,
                input:       input_val,
            }))
        }

        AwsContentBlock::ReasoningContent(rc) => {
            use shared::ReasoningContent;
            let thinking = rc
                .as_reasoning_text()
                .map(|rt| rt.text.clone())
                .unwrap_or_default();
            let signature = rc
                .as_reasoning_text()
                .ok()
                .and_then(|rt| rt.signature.clone());
            Some(ContentBlock::Reasoning(ReasoningContent { thinking, signature }))
        }

        // Image responses are rare; skip for now.
        _ => None,
    }
}

fn document_to_json(doc: Document) -> serde_json::Value {
    match doc {
        Document::Null      => serde_json::Value::Null,
        Document::Bool(b)   => serde_json::Value::Bool(b),
        Document::String(s) => serde_json::Value::String(s),
        Document::Number(n) => match n {
            aws_smithy_types::Number::PosInt(u) => serde_json::json!(u),
            aws_smithy_types::Number::NegInt(i) => serde_json::json!(i),
            aws_smithy_types::Number::Float(f)  => serde_json::json!(f),
        },
        Document::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(document_to_json).collect())
        }
        Document::Object(obj) => {
            serde_json::Value::Object(
                obj.into_iter()
                    .map(|(k, v)| (k, document_to_json(v)))
                    .collect(),
            )
        }
    }
}
