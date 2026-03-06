/// Full chat page — `/c/new` and `/c/:id`.

use std::sync::atomic::{AtomicU32, Ordering};
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map};
#[cfg(feature = "hydrate")]
use shared::ContentBlock;

use crate::context::auth::use_auth;
use crate::context::conversations::use_conversation_context;

static MSG_COUNTER: AtomicU32 = AtomicU32::new(1);

fn generate_tmp_id() -> String {
    format!("tmp-{}", MSG_COUNTER.fetch_add(1, Ordering::Relaxed))
}

#[derive(Clone)]
struct UiMessage {
    id:      String,
    role:    String,
    content: String,
}

#[component]
pub fn ChatPage() -> impl IntoView {
    let auth     = use_auth();
    let conv_ctx = use_conversation_context();
    let params   = use_params_map();
    let navigate = use_navigate();

    let (input_text,   set_input_text)   = signal(String::new());
    let selected_model: RwSignal<Option<String>> = RwSignal::new(None);
    let (messages,     set_messages)     = signal(Vec::<UiMessage>::new());
    let (streaming,    set_streaming)    = signal(String::new());
    let (is_streaming, set_is_streaming) = signal(false);
    let conv_title                       = RwSignal::new(String::new());

    let conv_id = move || params.with(|p| p.get("id").map(|s| s.to_string()));

    #[cfg(not(feature = "hydrate"))]
    let _ = (&conv_ctx, &navigate, &conv_id);

    // ── Models ────────────────────────────────────────────────────────────────
    let models_res = LocalResource::new(move || async move {
        #[cfg(feature = "hydrate")]
        { crate::api::fetch_models().await.unwrap_or_default() }
        #[cfg(not(feature = "hydrate"))]
        { vec![] }
    });

    // ── Load conversation ─────────────────────────────────────────────────────
    let conv_res = LocalResource::new(move || {
        let token = auth.get().map(|u| u.token).unwrap_or_default();
        #[cfg(feature = "hydrate")]
        let id = conv_id();
        async move {
            #[cfg(feature = "hydrate")]
            {
                let Some(id) = id else { return None::<shared::Conversation>; };
                if token.is_empty() { return None; }
                crate::api::fetch_conversation(&id, &token).await.ok()
            }
            #[cfg(not(feature = "hydrate"))]
            { let _ = token; None::<shared::Conversation> }
        }
    });

    Effect::new(move |_| {
        if let Some(wrap) = conv_res.get() {
            if let Some(conv) = (*wrap).clone() {
                conv_title.set(conv.meta.title.clone());
                let thread = conv.active_thread();
                let ui: Vec<UiMessage> = thread
                    .into_iter()
                    .filter_map(|msg: &shared::Message| {
                        let content = msg.text_content();
                        let role = match msg.role {
                            shared::Role::User      => "user".to_string(),
                            shared::Role::Assistant => "assistant".to_string(),
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
    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let text = input_text.get_untracked().trim().to_string();
        if text.is_empty() || is_streaming.get_untracked() { return; }

        set_input_text.set(String::new());
        set_messages.update(|v| {
            v.push(UiMessage {
                id:      generate_tmp_id(),
                role:    "user".to_string(),
                content: text.clone(),
            });
        });
        set_is_streaming.set(true);
        set_streaming.set(String::new());

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
                                    role:    "assistant".to_string(),
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
        <div class="hub-page">
            <h1>
                <a href="/">"Bedrock RS"</a>
                " / "
                <a href="/conversations">"Conversations"</a>
                " / "
                {move || {
                    if conv_id().is_some() {
                        let t = conv_title.get();
                        if t.is_empty() { "Loading…".to_string() } else { t }
                    } else {
                        "New Conversation".to_string()
                    }
                }}
            </h1>

            <h2>"Navigation"</h2>
            <table>
                <tr><td><a href="/c/new">"New Conversation"</a></td></tr>
                <tr><td><a href="/conversations">"All Conversations"</a></td></tr>
            </table>

            <h2>"Messages"</h2>
            {move || messages.get().into_iter().map(|m| view! {
                <p><strong>{format!("{}:", m.role)}</strong></p>
                <pre style="margin-top:0;white-space:pre-wrap">{m.content}</pre>
            }).collect_view()}

            {move || {
                let s = streaming.get();
                if s.is_empty() {
                    None
                } else {
                    Some(view! {
                        <p><strong>"assistant:"</strong></p>
                        <pre style="margin-top:0;white-space:pre-wrap">{s}</pre>
                    })
                }
            }}

            <form on:submit=on_submit>
                <p>
                    <label>"Model: "</label>
                    <select on:change=move |ev| {
                        let v = event_target_value(&ev);
                        selected_model.set(if v.is_empty() { None } else { Some(v) });
                    }>
                        <option value="">"Default"</option>
                        {move || models_res.get().map(|wrap| {
                            (*wrap).clone().into_iter().map(|m: shared::ModelInfo| view! {
                                <option value=m.id.clone()>{m.display_name}</option>
                            }).collect_view()
                        })}
                    </select>
                </p>
                <p>
                    <textarea rows="4" style="width:100%;font-family:monospace"
                        prop:value=input_text
                        on:input=move |ev| set_input_text.set(event_target_value(&ev))
                    />
                </p>
                <p>
                    <button type="submit" prop:disabled=is_streaming>
                        {move || if is_streaming.get() { "Sending…" } else { "Send" }}
                    </button>
                </p>
            </form>
        </div>
    }
}
