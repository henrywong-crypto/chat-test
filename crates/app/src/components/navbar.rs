/// Top navigation bar — shows the current conversation title and action buttons.

use leptos::prelude::*;

use crate::context::settings::use_settings_context;

#[component]
pub fn Navbar() -> impl IntoView {
    let settings = use_settings_context();

    view! {
        <header class="navbar">
            <span class="navbar-title">"New conversation"</span>
            <div class="navbar-actions">
                <button
                    class="btn-icon"
                    title="Settings"
                    on:click=move |_| settings.open.set(true)
                >
                    "⚙"
                </button>
            </div>
        </header>
    }
}
