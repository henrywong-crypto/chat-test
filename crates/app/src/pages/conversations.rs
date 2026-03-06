/// Conversations list page — `/conversations`.

use leptos::prelude::*;
use templates::{Breadcrumb, NavLink, Page};

use crate::context::auth::use_auth;

#[component]
pub fn ConversationsPage() -> impl IntoView {
    let auth = use_auth();

    let convs = LocalResource::new(move || {
        let token = auth.get().map(|u| u.token).unwrap_or_default();
        async move {
            if token.is_empty() { return vec![]; }
            #[cfg(feature = "hydrate")]
            { crate::api::fetch_conversations(&token).await.unwrap_or_default() }
            #[cfg(not(feature = "hydrate"))]
            { vec![] }
        }
    });

    let content = view! {
        {move || {
            convs.get().map(|wrap| {
                let list = (*wrap).clone();
                if list.is_empty() {
                    view! {
                        <p>"No conversations yet. "<a href="/c/new">"Start a new one."</a></p>
                    }.into_any()
                } else {
                    let rows = list.into_iter().map(|c: shared::ConversationMeta| {
                        let href = format!("/c/{}", c.id);
                        view! {
                            <tr>
                                <td><a href={href}>{c.title}</a></td>
                                <td>{c.id}</td>
                            </tr>
                        }
                    }).collect_view();
                    view! {
                        <table>
                            <tr><th>"Title"</th><th>"ID"</th></tr>
                            {rows}
                        </table>
                    }.into_any()
                }
            })
        }}
    };

    Page {
        title:       "Conversations".to_string(),
        breadcrumbs: vec![
            Breadcrumb::link("Bedrock RS", "/"),
            Breadcrumb::current("Conversations"),
        ],
        nav_links:   vec![NavLink::new("New Conversation", "/c/new")],
        info_rows:   vec![],
        content,
        subpages:    vec![],
    }
    .into_view()
}
