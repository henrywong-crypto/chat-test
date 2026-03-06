/// POST /api/chat — streaming chat via Server-Sent Events.
///
/// Accepts a `SendMessageRequest` JSON body and streams back `StreamEvent`
/// items as SSE data lines until the model finishes or errors.
///
/// # Flow
/// 1. Authenticate → extract conversation (or create new one)
/// 2. Load bot config if `bot_id` is present
/// 3. Resolve the Bedrock invoke target via the profile manager
/// 4. Build the `ConverseRequest` from conversation history
/// 5. Start a Bedrock stream in a background task
/// 6. Forward events over SSE channel; on Done persist the assistant message

use std::collections::HashMap;
use std::convert::Infallible;
use std::time::Duration;

use axum::{
    extract::State,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    Json,
};
use chrono::Utc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::warn;
use uuid::Uuid;

use auth::extractor::CurrentUser;
use bedrock::{
    converse::{
        build_inference_config, build_system_blocks, convert_messages, ConverseRequest,
    },
    cost::calculate_message_cost,
    stream::{stream_response, BedrockStreamEvent},
};
use shared::{
    api::{SendMessageRequest, StreamEvent},
    ContentBlock, Conversation, ConversationMeta, Message, Role, TextContent, TokenUsage,
    ToolUseContent,
};

use crate::server::state::AppState;

// ── Handler ───────────────────────────────────────────────────────────────────

pub async fn chat_stream(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(req): Json<SendMessageRequest>,
) -> impl IntoResponse {
    let (tx, rx) = mpsc::channel::<Result<Event, Infallible>>(64);

    tokio::spawn(async move {
        if let Err(e) = run_chat(state, user, req, tx.clone()).await {
            let ev = error_event(&e.to_string());
            let _ = tx.send(Ok(ev)).await;
        }
    });

    Sse::new(ReceiverStream::new(rx))
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}

// ── Core logic ────────────────────────────────────────────────────────────────

async fn run_chat(
    state: AppState,
    user:  shared::User,
    req:   SendMessageRequest,
    tx:    mpsc::Sender<Result<Event, Infallible>>,
) -> anyhow::Result<()> {
    // ── 1. Resolve conversation ───────────────────────────────────────────────
    let (mut conv, is_new) = match &req.conversation_id {
        Some(id) => {
            let c = state.conversations.get(&user.id, id).await
                .map_err(|e| anyhow::anyhow!("load conversation: {e}"))?;
            (c, false)
        }
        None => (new_conversation(&user.id, req.bot_id.as_deref()), true),
    };

    // ── 2. Load bot config ────────────────────────────────────────────────────
    let (bot_instruction, bot_model, bot_gen_params) = if let Some(ref bot_id) = req.bot_id {
        match state.bots.get(&user.id, bot_id).await {
            Ok(bot) => (
                Some(bot.instruction.clone()),
                bot.model_id.clone(),
                Some(bot.generation_params.clone()),
            ),
            Err(_) => (None, None, None),
        }
    } else {
        (None, None, None)
    };

    // ── 3. Resolve model and invoke target ────────────────────────────────────
    let model_id = req.model_id
        .or(bot_model)
        .unwrap_or_else(|| state.default_model_id.clone());

    let model_arn = state.profile_manager
        .get_invoke_target(&user.id, &model_id)
        .await
        .map_err(|e| anyhow::anyhow!("resolve model: {e}"))?;

    // ── 4. Build user message and append to conversation ──────────────────────
    let user_msg_id = Uuid::new_v4().to_string();
    let user_msg = Message {
        id:                    user_msg_id.clone(),
        role:                  Role::User,
        content:               req.content.clone(),
        parent_message_id:     Some(conv.last_message_id.clone()),
        children_message_ids:  vec![],
        create_time:           unix_now(),
        feedback:              None,
        used_chunks:           vec![],
        model:                 None,
        token_usage:           None,
    };

    // Update the old tip's children list.
    if let Some(parent) = conv.message_map.get_mut(&conv.last_message_id) {
        parent.children_message_ids.push(user_msg_id.clone());
    }
    conv.message_map.insert(user_msg_id.clone(), user_msg);
    conv.last_message_id = user_msg_id.clone();

    // If this is a new conversation set the title from the user text.
    if is_new {
        let text = req.content.iter()
            .find_map(|b| if let ContentBlock::Text(t) = b { Some(t.body.as_str()) } else { None })
            .unwrap_or("New conversation");
        conv.meta.title = text.chars().take(60).collect();
    }

    // ── 5. Build Bedrock ConverseRequest ──────────────────────────────────────
    let gen_params = bot_gen_params.unwrap_or_default();
    let thread = conv.active_thread();
    let aws_messages = convert_messages(
        &thread.iter().map(|m| (*m).clone()).collect::<Vec<_>>(),
    ).map_err(|e| anyhow::anyhow!("convert messages: {e}"))?;

    let converse_req = ConverseRequest {
        model_arn,
        messages:   aws_messages,
        system:     build_system_blocks(bot_instruction.as_deref()),
        inf_config: build_inference_config(&gen_params),
    };

    // ── 6. Start stream ───────────────────────────────────────────────────────
    let mut stream = stream_response(&state.bedrock_runtime, converse_req)
        .await
        .map_err(|e| anyhow::anyhow!("start stream: {e}"))?;

    // Accumulate for DB save.
    let mut text_buf    = String::new();
    let mut tool_blocks: Vec<ToolUseContent> = vec![];
    #[allow(unused_assignments)]
    let mut usage       = TokenUsage::default();

    // ── 7. Forward events ─────────────────────────────────────────────────────
    use tokio_stream::StreamExt as _;
    while let Some(result) = stream.next().await {
        match result {
            Err(e) => {
                let _ = tx.send(Ok(error_event(&e.to_string()))).await;
                return Err(anyhow::anyhow!("stream error: {e}"));
            }
            Ok(ev) => match ev {
                BedrockStreamEvent::Text { delta } => {
                    text_buf.push_str(&delta);
                    let sse = stream_event_to_sse(&StreamEvent::Text { delta });
                    if tx.send(Ok(sse)).await.is_err() { return Ok(()); }
                }
                BedrockStreamEvent::ToolUse(tu) => {
                    let sse = stream_event_to_sse(&StreamEvent::ToolUse(tu.clone()));
                    tool_blocks.push(tu);
                    if tx.send(Ok(sse)).await.is_err() { return Ok(()); }
                }
                BedrockStreamEvent::Done { usage: u, stop_reason } => {
                    usage = u;

                    // ── 8. Persist assistant message ──────────────────────────
                    let asst_msg_id = Uuid::new_v4().to_string();
                    let mut content: Vec<ContentBlock> = vec![];
                    if !text_buf.is_empty() {
                        content.push(ContentBlock::Text(TextContent { body: text_buf.clone() }));
                    }
                    for tu in &tool_blocks {
                        content.push(ContentBlock::ToolUse(tu.clone()));
                    }

                    let asst_msg = Message {
                        id:                    asst_msg_id.clone(),
                        role:                  Role::Assistant,
                        content,
                        parent_message_id:     Some(user_msg_id.clone()),
                        children_message_ids:  vec![],
                        create_time:           unix_now(),
                        feedback:              None,
                        used_chunks:           vec![],
                        model:                 Some(model_id.clone()),
                        token_usage:           Some(usage.clone()),
                    };

                    // Update user message children.
                    if let Some(um) = conv.message_map.get_mut(&user_msg_id) {
                        um.children_message_ids.push(asst_msg_id.clone());
                    }
                    conv.message_map.insert(asst_msg_id.clone(), asst_msg);
                    conv.last_message_id = asst_msg_id.clone();

                    // Accumulate cost.
                    let cost = calculate_message_cost(&model_id, &usage, &state.aws_region);
                    conv.meta.total_price += cost;

                    // Save conversation (non-fatal if it fails).
                    if let Err(e) = state.conversations.put(&conv).await {
                        warn!(error = %e, "failed to persist conversation");
                    }

                    // Send Done event.
                    let done = StreamEvent::Done {
                        usage,
                        stop_reason,
                        message_id:      asst_msg_id,
                        conversation_id: conv.meta.id.clone(),
                    };
                    let _ = tx.send(Ok(stream_event_to_sse(&done))).await;
                    return Ok(());
                }
                BedrockStreamEvent::Error { message } => {
                    let _ = tx.send(Ok(error_event(&message))).await;
                    return Err(anyhow::anyhow!("bedrock error: {message}"));
                }
            },
        }
    }

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn new_conversation(user_id: &str, bot_id: Option<&str>) -> Conversation {
    let conv_id = Uuid::new_v4().to_string();
    let now = unix_now();

    // Sentinel "system" root node that all messages branch from.
    let root_id = "system".to_string();
    let root = Message {
        id:                    root_id.clone(),
        role:                  Role::System,
        content:               vec![],
        parent_message_id:     None,
        children_message_ids:  vec![],
        create_time:           now,
        feedback:              None,
        used_chunks:           vec![],
        model:                 None,
        token_usage:           None,
    };

    let mut message_map = HashMap::new();
    message_map.insert(root_id.clone(), root);

    Conversation {
        meta: ConversationMeta {
            id:          conv_id,
            title:       "New conversation".into(),
            create_time: now,
            total_price: 0.0,
            bot_id:      bot_id.map(|s| s.to_string()),
            user_id:     user_id.to_string(),
        },
        last_message_id: root_id,
        message_map,
    }
}

fn stream_event_to_sse(ev: &StreamEvent) -> Event {
    Event::default().data(serde_json::to_string(ev).unwrap_or_default())
}

fn error_event(msg: &str) -> Event {
    let ev = StreamEvent::Error { message: msg.to_string() };
    stream_event_to_sse(&ev)
}

fn unix_now() -> f64 {
    Utc::now().timestamp_millis() as f64 / 1000.0
}
