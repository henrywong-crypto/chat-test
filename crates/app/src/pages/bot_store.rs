/// Bot Store page — `/bots/store`.  Shows all public bots.

use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use templates::{Breadcrumb, Page};

use crate::components::bots::bot_card::BotCard;
use crate::context::auth::use_auth;

#[component]
pub fn BotStorePage() -> impl IntoView {
    let auth     = use_auth();
    let navigate = use_navigate();

    let bots = LocalResource::new(move || {
        let token = auth.get().map(|u| u.token).unwrap_or_default();
        async move {
            if token.is_empty() { return vec![]; }
            #[cfg(feature = "hydrate")]
            { crate::api::fetch_bot_store(&token).await.unwrap_or_default() }
            #[cfg(not(feature = "hydrate"))]
            { vec![] }
        }
    });

    let bot_grid = view! {
        <div class="bot-grid">
            {move || {
                bots.get().map(|wrap| {
                    let list = (*wrap).clone();
                    if list.is_empty() {
                        view! {
                            <div class="empty-state">
                                <h2>"No public bots"</h2>
                                <p>"No bots have been published to the store yet."</p>
                            </div>
                        }.into_any()
                    } else {
                        let nav = navigate.clone();
                        list.into_iter().map(move |bot| {
                            let nav2 = nav.clone();
                            view! {
                                <BotCard
                                    bot=bot
                                    editable=false
                                    on_delete=|_| {}
                                    on_use=move |_id| {
                                        nav2("/", Default::default());
                                    }
                                />
                            }
                        }).collect_view().into_any()
                    }
                })
            }}
        </div>
    };

    Page {
        title: "Bot Store".to_string(),
        breadcrumbs: vec![Breadcrumb::current("Bot Store")],
        nav_links: vec![],
        info_rows: vec![],
        content: bot_grid,
        subpages: vec![],
    }
    .into_view()
}
