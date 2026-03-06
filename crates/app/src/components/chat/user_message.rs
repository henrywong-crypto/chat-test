use leptos::prelude::*;

#[component]
pub fn UserMessage(#[prop(into)] content: String) -> impl IntoView {
    view! {
        <div class="message user-message">
            <div class="message-bubble">{content}</div>
        </div>
    }
}
