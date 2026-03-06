/// Full chat page — handles both new conversations (`/`) and existing ones (`/c/:id`).

use std::sync::atomic::{AtomicU32, Ordering};

use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map};
#[cfg(feature = "hydrate")]
use shared::ContentBlock;

use crate::components::chat::{
    message_input::MessageInput,
    message_list::{MessageList, MessageRole, UiMessage},
    model_selector::ModelSelector,
};
use crate::context::auth::use_auth;
use crate::context::conversations::use_conversation_context;

static MSG_COUNTER: AtomicU32 = AtomicU32::new(1);

fn tmp_id() -> String {
    format!("tmp-{}", MSG_COUNTER.fetch_add(1, Ordering::Relaxed))
}

#[component]
pub fn ChatPage() -> impl IntoView {
    let auth     = use_auth();
    let conv_ctx = use_conversation_context();
    let params   = use_params_map();
    let navigate = use_navigate();

    let selected_model: RwSignal<Option<String>> = RwSignal::new(None);
    let (messages,     set_messages)     = signal(Vec::<UiMessage>::new());
    let (streaming,    set_streaming)    = signal(String::new());
    let (is_streaming, set_is_streaming) = signal(false);

    let conv_id = move || params.with(|p| p.get("id").map(|s| s.to_string()));

    // These are only referenced in #[cfg(feature = "hydrate")] blocks; suppress
    // the unused-variable lint that fires in the SSR build.
    #[cfg(not(feature = "hydrate"))]
    let _ = (&conv_ctx, &navigate, &conv_id);
    // ── Models ────────────────────────────────────────────────────────────────
    let models_res = LocalResource::new(move || async move {
        #[cfg(feature = "hydrate")]
        {
            crate::api::fetch_models().await.unwrap_or_default()
        }
        #[cfg(not(feature = "hydrate"))]
        { vec![] }
    });

    let models_signal = Signal::derive(move || {
        models_res.get().map(|w| (*w).clone()).unwrap_or_default()
    });

    // ── Load conversation ─────────────────────────────────────────────────────
    let conv_res = LocalResource::new(move || {
        let token = auth.get().map(|u| u.token).unwrap_or_default();
        #[cfg(feature = "hydrate")]
        let id = conv_id();
        async move {
            #[cfg(feature = "hydrate")]
            {
                let Some(id) = id else {
                    return None::<shared::Conversation>;
                };
                if token.is_empty() { return None; }
                crate::api::fetch_conversation(&id, &token).await.ok()
            }
            #[cfg(not(feature = "hydrate"))]
            {
                let _ = token;
                None::<shared::Conversation>
            }
        }
    });

    Effect::new(move |_| {
        if let Some(wrap) = conv_res.get() {
            if let Some(conv) = (*wrap).clone() {
                let thread = conv.active_thread();
                let ui: Vec<UiMessage> = thread
                    .into_iter()
                    .filter_map(|msg: &shared::Message| {
                        let content = msg.text_content();
                        let role = match msg.role {
                            shared::Role::User      => MessageRole::User,
                            shared::Role::Assistant => MessageRole::Assistant,
                            shared::Role::System    => return None,
                        };
                        Some(UiMessage { id: msg.id.clone(), role, content })
                    })
                    .collect();
                set_messages.set(ui);
            }
        }
    });

    // ── Send ──────────────────────────────────────────────────────────────────
    let on_send = move |text: String| {
        if is_streaming.get_untracked() { return; }

        // Optimistic user bubble — shown in both SSR stub and hydrate.
        set_messages.update(|v| {
            v.push(UiMessage {
                id:      tmp_id(),
                role:    MessageRole::User,
                content: text.clone(),
            });
        });
        set_is_streaming.set(true);
        set_streaming.set(String::new());

        // Everything below is browser-only.
        #[cfg(feature = "hydrate")]
        {
            use shared::{SendMessageRequest, StreamEvent};

            let token           = auth.get_untracked().map(|u| u.token).unwrap_or_default();
            let conversation_id = conv_id();
            let model_id        = selected_model.get_untracked();
            let current_conv_id = conversation_id.clone();
            let nav             = navigate.clone();

            let request = SendMessageRequest {
                content:         vec![ContentBlock::text(text)],
                bot_id:          None,
                conversation_id,
                model_id,
            };

            wasm_bindgen_futures::spawn_local(async move {
                let result = crate::api::stream_chat(
                    request,
                    token,
                    move |event: StreamEvent| match event {
                        StreamEvent::Text { delta } => {
                            set_streaming.update(|t| t.push_str(&delta));
                        }
                        StreamEvent::Done {
                            message_id,
                            conversation_id: new_conv_id,
                            ..
                        } => {
                            let final_text = streaming.get_untracked();
                            set_messages.update(|v| {
                                v.push(UiMessage {
                                    id:      message_id,
                                    role:    MessageRole::Assistant,
                                    content: final_text,
                                });
                            });
                            set_streaming.set(String::new());
                            set_is_streaming.set(false);
                            if current_conv_id.is_none() {
                                nav(&format!("/c/{new_conv_id}"), Default::default());
                            }
                            conv_ctx.version.update(|v| *v += 1);
                        }
                        StreamEvent::Error { message } => {
                            leptos::logging::error!("stream error: {message}");
                            set_streaming.set(String::new());
                            set_is_streaming.set(false);
                        }
                        _ => {}
                    },
                )
                .await;

                if result.is_err() {
                    set_streaming.set(String::new());
                    set_is_streaming.set(false);
                }
            });
        }
    };

    // ── View ──────────────────────────────────────────────────────────────────
    view! {
        <div class="chat-page">
            <div class="chat-toolbar">
                <ModelSelector selected=selected_model models=models_signal/>
            </div>

            {move || {
                if messages.get().is_empty() && streaming.get().is_empty() {
                    view! {
                        <div class="page-center">
                            <div class="empty-state">
                                <h2>"What can I help you with?"</h2>
                                <p>"Start typing below to begin a new conversation."</p>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <MessageList messages=messages streaming=streaming/>
                    }.into_any()
                }
            }}

            <MessageInput
                on_send=on_send
                disabled=Signal::derive(move || is_streaming.get())
            />
        </div>
    }
}
