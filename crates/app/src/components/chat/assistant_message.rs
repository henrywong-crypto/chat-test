use leptos::prelude::*;
use super::markdown::MarkdownRenderer;

#[component]
pub fn AssistantMessage(#[prop(into)] content: String) -> impl IntoView {
    view! {
        <div class="message assistant-message">
            <MarkdownRenderer content=content/>
        </div>
    }
}
