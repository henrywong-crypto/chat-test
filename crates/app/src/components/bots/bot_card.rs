use leptos::prelude::*;
use shared::{Bot, BotVisibility};

#[component]
pub fn BotCard(
    bot:       Bot,
    /// Show Edit / Delete buttons (owner view).
    #[prop(default = false)]
    editable:  bool,
    on_delete: impl Fn(String) + 'static,
    on_use:    impl Fn(String) + 'static,
) -> impl IntoView {
    let id          = bot.id.clone();
    let id2         = bot.id.clone();
    let title       = bot.title.clone();
    let description = bot.description.clone();
    let vis_label   = match bot.visibility {
        BotVisibility::Public   => "Public",
        BotVisibility::Unlisted => "Unlisted",
        BotVisibility::Private  => "Private",
    };
    let vis_class = match bot.visibility {
        BotVisibility::Public   => "badge badge-green",
        BotVisibility::Unlisted => "badge badge-yellow",
        BotVisibility::Private  => "badge badge-gray",
    };
    let edit_href = format!("/bots/{}/edit", bot.id);

    view! {
        <div class="bot-card">
            <div class="bot-card-header">
                <span class="bot-card-title">{title}</span>
                <span class=vis_class>{vis_label}</span>
            </div>
            <p class="bot-card-desc">{description}</p>
            <div class="bot-card-actions">
                {if editable {
                    view! {
                        <a href=edit_href class="btn btn-sm btn-secondary">"Edit"</a>
                        <button
                            class="btn btn-sm btn-danger"
                            on:click=move |_| on_delete(id.clone())
                        >
                            "Delete"
                        </button>
                    }.into_any()
                } else {
                    view! {
                        <button
                            class="btn btn-sm btn-primary"
                            on:click=move |_| on_use(id2.clone())
                        >
                            "Use Bot"
                        </button>
                    }.into_any()
                }}
            </div>
        </div>
    }
}
