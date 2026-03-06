/// Streaming response handler for Bedrock Converse Stream API.
///
/// Returns a `tokio_stream::wrappers::ReceiverStream` that emits
/// `BedrockStreamEvent` items as the model generates tokens.  A background
/// task reads from the AWS EventStream and forwards converted events through
/// an mpsc channel.

use aws_sdk_bedrockruntime::{
    types::{
        ContentBlockDelta, ContentBlockStart, ConverseStreamOutput as AwsStreamEvent,
    },
    Client as BedrockRuntimeClient,
};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use shared::{TokenUsage, ToolUseContent};

use crate::{
    converse::ConverseRequest,
    error::BedrockError,
};

// ── BedrockStreamEvent ────────────────────────────────────────────────────────

/// Events emitted by `stream_response`.
///
/// The chat handler in `crates/app` converts these to `shared::StreamEvent`
/// (which adds `message_id` / `conversation_id` after the DB write).
#[derive(Debug, Clone)]
pub enum BedrockStreamEvent {
    /// One or more token characters appended to the current text block.
    Text { delta: String },
    /// The model is requesting a tool invocation (complete, after accumulation).
    ToolUse(ToolUseContent),
    /// Streaming is complete.
    Done { usage: TokenUsage, stop_reason: String },
    /// An error occurred during streaming.
    Error { message: String },
}

// ── Accumulator for in-progress tool use ─────────────────────────────────────

#[derive(Default)]
struct ToolUseAccumulator {
    #[allow(dead_code)]
    index:       i32,
    tool_use_id: String,
    name:        String,
    /// Raw JSON string being assembled from deltas.
    input_json:  String,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Start a streaming Bedrock Converse call.
///
/// The stream terminates when the model emits a `Done` or `Error` event.
/// Dropping the returned stream cancels the background task on the next
/// channel send (the task detects the closed receiver and exits).
pub async fn stream_response(
    client: &BedrockRuntimeClient,
    req:    ConverseRequest,
) -> Result<ReceiverStream<Result<BedrockStreamEvent, BedrockError>>, BedrockError> {
    let resp = client
        .converse_stream()
        .model_id(&req.model_arn)
        .set_messages(Some(req.messages))
        .set_system(if req.system.is_empty() { None } else { Some(req.system) })
        .inference_config(req.inf_config)
        .send()
        .await
        .map_err(|e| {
            let msg = format!("{e:?}");
            if msg.contains("ThrottlingException") || msg.contains("TooManyRequests") {
                BedrockError::Throttling
            } else {
                BedrockError::Sdk(msg)
            }
        })?;

    let (tx, rx) = mpsc::channel::<Result<BedrockStreamEvent, BedrockError>>(64);

    tokio::spawn(async move {
        let mut event_stream = resp.stream;
        let mut tool_accum: Option<ToolUseAccumulator> = None;
        let mut usage = TokenUsage::default();
        let mut stop_reason = String::from("end_turn");

        loop {
            match event_stream.recv().await {
                Ok(Some(event)) => {
                    let events = process_event(event, &mut tool_accum, &mut usage, &mut stop_reason);
                    for ev in events {
                        if tx.send(Ok(ev)).await.is_err() {
                            // Receiver dropped — client disconnected.
                            return;
                        }
                    }
                }
                Ok(None) => {
                    // Stream ended cleanly — emit Done.
                    let _ = tx
                        .send(Ok(BedrockStreamEvent::Done {
                            usage:       usage.clone(),
                            stop_reason: stop_reason.clone(),
                        }))
                        .await;
                    return;
                }
                Err(e) => {
                    let _ = tx
                        .send(Err(BedrockError::StreamError(e.to_string())))
                        .await;
                    return;
                }
            }
        }
    });

    Ok(ReceiverStream::new(rx))
}

// ── Event processing ──────────────────────────────────────────────────────────

fn process_event(
    event:       AwsStreamEvent,
    tool_accum:  &mut Option<ToolUseAccumulator>,
    usage:       &mut TokenUsage,
    stop_reason: &mut String,
) -> Vec<BedrockStreamEvent> {
    match event {
        // ── Text / reasoning deltas ───────────────────────────────────────────
        AwsStreamEvent::ContentBlockDelta(e) => {
            match e.delta {
                Some(ContentBlockDelta::Text(text)) => {
                    vec![BedrockStreamEvent::Text { delta: text }]
                }
                Some(ContentBlockDelta::ReasoningContent(rc)) => {
                    // Emit reasoning text deltas as Text so the UI can display them.
                    if let Ok(text) = rc.as_text() {
                        vec![BedrockStreamEvent::Text { delta: text.clone() }]
                    } else {
                        vec![]
                    }
                }
                Some(ContentBlockDelta::ToolUse(tu)) => {
                    // Accumulate the JSON input string.
                    if let Some(acc) = tool_accum {
                        acc.input_json.push_str(&tu.input);
                    }
                    vec![]
                }
                _ => vec![],
            }
        }

        // ── Block start — begin a new tool use accumulator ────────────────────
        AwsStreamEvent::ContentBlockStart(e) => {
            if let Some(ContentBlockStart::ToolUse(tu)) = e.start {
                *tool_accum = Some(ToolUseAccumulator {
                    index:       e.content_block_index,
                    tool_use_id: tu.tool_use_id,
                    name:        tu.name,
                    input_json:  String::new(),
                });
            }
            vec![]
        }

        // ── Block stop — finalise a complete tool use ─────────────────────────
        AwsStreamEvent::ContentBlockStop(_) => {
            if let Some(acc) = tool_accum.take() {
                let input = serde_json::from_str(&acc.input_json)
                    .unwrap_or(serde_json::Value::Object(Default::default()));
                vec![BedrockStreamEvent::ToolUse(ToolUseContent {
                    tool_use_id: acc.tool_use_id,
                    name:        acc.name,
                    input,
                })]
            } else {
                vec![]
            }
        }

        // ── Message stop — record stop reason ────────────────────────────────
        AwsStreamEvent::MessageStop(e) => {
            *stop_reason = e.stop_reason.as_str().to_string();
            vec![]
        }

        // ── Usage metadata ────────────────────────────────────────────────────
        AwsStreamEvent::Metadata(e) => {
            if let Some(u) = e.usage {
                usage.input_tokens       = u.input_tokens  as u32;
                usage.output_tokens      = u.output_tokens as u32;
                usage.cache_read_tokens  = u.cache_read_input_tokens.unwrap_or(0) as u32;
                usage.cache_write_tokens = u.cache_write_input_tokens.unwrap_or(0) as u32;
            }
            vec![]
        }

        // Unknown future variants — ignore.
        _ => vec![],
    }
}
