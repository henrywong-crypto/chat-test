use leptos::prelude::*;
use super::{assistant_message::AssistantMessage, user_message::UserMessage};

// ── Domain type ───────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
}

#[derive(Clone, Debug)]
pub struct UiMessage {
    pub id:      String,
    pub role:    MessageRole,
    pub content: String,
}

// ── Component ─────────────────────────────────────────────────────────────────

#[component]
pub fn MessageList(
    messages:  ReadSignal<Vec<UiMessage>>,
    streaming: ReadSignal<String>,
) -> impl IntoView {
    view! {
        <div class="message-list">
            {move || {
                messages.get().into_iter().map(|msg| {
                    match msg.role {
                        MessageRole::User =>
                            view! { <UserMessage content=msg.content/> }.into_any(),
                        MessageRole::Assistant =>
                            view! { <AssistantMessage content=msg.content/> }.into_any(),
                    }
                }).collect_view()
            }}

            // Live streaming bubble
            {move || {
                let text = streaming.get();
                if text.is_empty() {
                    None
                } else {
                    Some(view! { <AssistantMessage content=text/> })
                }
            }}
        </div>
    }
}
