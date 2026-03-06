/// Full chat page — `/c/new` and `/c/:id`.

use std::sync::atomic::{AtomicU32, Ordering};
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map};
#[cfg(feature = "hydrate")]
use shared::{ContentBlock, S3FileContent};
use shared::{Conversation, Message, ModelInfo, Role};

use crate::context::auth::use_auth;
use crate::context::conversations::use_conversation_context;

#[allow(dead_code)]
static MSG_COUNTER: AtomicU32 = AtomicU32::new(1);

#[cfg_attr(not(feature = "hydrate"), allow(dead_code))]
fn generate_tmp_id() -> String {
    format!("tmp-{}", MSG_COUNTER.fetch_add(1, Ordering::Relaxed))
}

#[derive(Clone)]
struct UiMessage {
    #[allow(dead_code)]
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
    let (uploading,    set_uploading)    = signal(false);
    let conv_title                       = RwSignal::new(String::new());
    let file_ref: NodeRef<leptos::html::Input> = NodeRef::new();

    let conv_id = move || params.with(|p| p.get("id").map(|s| s.to_string()));

    #[cfg(not(feature = "hydrate"))]
    let _ = (&conv_ctx, &navigate, &conv_id, &set_uploading, &file_ref);

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
                let Some(id) = id else { return None::<Conversation>; };
                if token.is_empty() { return None; }
                crate::api::fetch_conversation(&id, &token).await.ok()
            }
            #[cfg(not(feature = "hydrate"))]
            { let _ = token; None::<Conversation> }
        }
    });

    Effect::new(move |_| {
        if let Some(wrap) = conv_res.get() {
            if let Some(conv) = (*wrap).clone() {
                conv_title.set(conv.meta.title.clone());
                let thread = conv.active_thread();
                let ui: Vec<UiMessage> = thread
                    .into_iter()
                    .filter_map(|msg: &Message| {
                        let role = match msg.role {
                            Role::User      => "user".to_string(),
                            Role::Assistant => "assistant".to_string(),
                            Role::System    => return None,
                        };
                        let mut parts: Vec<String> = vec![];
                        for block in &msg.content {
                            match block {
                                shared::ContentBlock::Text(t)   => parts.push(t.body.clone()),
                                shared::ContentBlock::S3File(f) => {
                                    parts.push(format!("[file: {}]", f.name))
                                }
                                _ => {}
                            }
                        }
                        Some(UiMessage { id: msg.id.clone(), role, content: parts.join("\n") })
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
        if text.is_empty() || is_streaming.get_untracked() || uploading.get_untracked() {
            return;
        }

        set_input_text.set(String::new());
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

            wasm_bindgen_futures::spawn_local(async move {
                // ── Optional file upload ──────────────────────────────────────
                let s3_block: Option<ContentBlock> = if let Some(input_el) = file_ref.get_untracked() {
                    if let Some(file) = input_el.files().and_then(|fl| fl.get(0)) {
                        set_uploading.set(true);
                        let form_data = web_sys::FormData::new().unwrap();
                        form_data.append_with_blob("file", &file).unwrap();
                        let result = crate::api::upload_file(form_data, &token).await;
                        set_uploading.set(false);
                        match result {
                            Ok(resp) => Some(ContentBlock::S3File(S3FileContent {
                                key:        resp.key,
                                media_type: resp.content_type,
                                name:       resp.name,
                            })),
                            Err(e) => {
                                leptos::logging::error!("upload error: {e}");
                                None
                            }
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                // ── Build display content ─────────────────────────────────────
                let display = if let Some(ref sf) = s3_block {
                    if let ContentBlock::S3File(f) = sf {
                        format!("{text}\n[file: {}]", f.name)
                    } else { text.clone() }
                } else {
                    text.clone()
                };

                set_messages.update(|v| {
                    v.push(UiMessage {
                        id:      generate_tmp_id(),
                        role:    "user".to_string(),
                        content: display,
                    });
                });

                // ── Build request content ─────────────────────────────────────
                let mut content = vec![ContentBlock::text(text)];
                if let Some(block) = s3_block { content.push(block); }

                let request = SendMessageRequest {
                    content,
                    bot_id:          None,
                    conversation_id,
                    model_id,
                };

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
            <table><tbody>
                {move || conv_id().map(|_| view! {
                    <tr><td><a href="/c/new">"New Conversation"</a></td></tr>
                })}
                <tr><td><a href="/conversations">"Back"</a></td></tr>
            </tbody></table>

            {move || (!messages.get().is_empty() || !streaming.get().is_empty()).then(|| view! {
                <h2>"Messages"</h2>
                <p>{move || format!("Total: {}", messages.get().len())}</p>
                <table>
                    <thead><tr><th>"#"</th><th>"Role"</th><th>"Content"</th></tr></thead>
                    <tbody>
                    {move || messages.get().into_iter().enumerate().map(|(i, m)| view! {
                        <tr>
                            <td>{i + 1}</td>
                            <td>{m.role}</td>
                            <td><pre>{m.content}</pre></td>
                        </tr>
                    }).collect::<Vec<_>>()}
                    {move || {
                        let s = streaming.get();
                        if s.is_empty() { None } else {
                            Some(view! {
                                <tr>
                                    <td>"…"</td>
                                    <td>"assistant"</td>
                                    <td><pre>{s}</pre></td>
                                </tr>
                            })
                        }
                    }}
                    </tbody>
                </table>
            })}

            <form on:submit=on_submit>
                <table><tbody>
                    <tr>
                        <td><label>"Model"</label></td>
                        <td>
                            <select on:change=move |ev| {
                                let v = event_target_value(&ev);
                                selected_model.set(if v.is_empty() { None } else { Some(v) });
                            }>
                                <option value="">"Default"</option>
                                {move || models_res.get().map(|wrap| {
                                    (*wrap).clone().into_iter().map(|m: ModelInfo| view! {
                                        <option value=m.id.clone()>{m.display_name}</option>
                                    }).collect::<Vec<_>>()
                                })}
                            </select>
                        </td>
                    </tr>
                    <tr>
                        <td><label>"File"</label></td>
                        <td><input type="file" node_ref=file_ref accept="image/*"/></td>
                    </tr>
                    <tr>
                        <td><label>"Message"</label></td>
                        <td>
                            <textarea rows="4"
                                prop:value=input_text
                                on:input=move |ev| set_input_text.set(event_target_value(&ev))
                            />
                        </td>
                    </tr>
                    <tr>
                        <td></td>
                        <td>
                            <input type="submit"
                                prop:value=move || {
                                    if uploading.get()    { "Uploading…" }
                                    else if is_streaming.get() { "Sending…" }
                                    else { "Send" }
                                }
                                prop:disabled=move || is_streaming.get() || uploading.get()
                            />
                        </td>
                    </tr>
                </tbody></table>
            </form>
        </div>
    }
}
