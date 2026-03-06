/// Bot Store page — `/bots/store`.  Shows all public bots.

use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use templates::{Breadcrumb, NavLink, Page};

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

    let bot_list = view! {
        {move || {
            bots.get().map(|wrap| {
                let list = (*wrap).clone();
                if list.is_empty() {
                    view! { <p>"No bots have been published to the store yet."</p> }.into_any()
                } else {
                    let nav = navigate.clone();
                    let rows = list.into_iter().map(move |bot: shared::Bot| {
                        let nav2 = nav.clone();
                        view! {
                            <tr>
                                <td>{bot.title}</td>
                                <td>{bot.description}</td>
                                <td>
                                    <a href="#" on:click=move |ev| {
                                        ev.prevent_default();
                                        nav2("/", Default::default());
                                    }>"Use"</a>
                                </td>
                            </tr>
                        }
                    }).collect_view();
                    view! {
                        <table>
                            <tr><th>"Title"</th><th>"Description"</th><th></th></tr>
                            {rows}
                        </table>
                    }.into_any()
                }
            })
        }}
    };

    Page {
        title: "Bot Store".to_string(),
        breadcrumbs: vec![Breadcrumb::current("Bot Store")],
        nav_links: vec![NavLink::new("My Bots", "/bots")],
        info_rows: vec![],
        content: bot_list,
        subpages: vec![],
    }
    .into_view()
}
