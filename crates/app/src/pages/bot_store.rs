/// Bot Store page — `/bots/store`.  Shows all public bots.

use leptos::prelude::*;
use templates::{Breadcrumb, NavLink, Page};

use crate::context::auth::use_auth;

#[component]
pub fn BotStorePage() -> impl IntoView {
    let auth = use_auth();

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
                    let rows = list.into_iter().map(|bot: shared::Bot| {
                        view! {
                            <tr>
                                <td>{bot.title}</td>
                                <td>{bot.description}</td>
                                <td><a href="/c/new">"Use"</a></td>
                            </tr>
                        }
                    }).collect::<Vec<_>>();
                    view! {
                        <table>
                            <thead><tr><th>"Title"</th><th>"Description"</th><th></th></tr></thead>
                            <tbody>{rows}</tbody>
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
