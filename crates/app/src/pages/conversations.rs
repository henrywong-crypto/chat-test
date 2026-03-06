/// Conversations list page — `/conversations`.

use leptos::prelude::*;
use templates::{Breadcrumb, NavLink, Page};

use crate::context::auth::use_auth;

const PER_PAGE: usize = 20;

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

    let page = RwSignal::new(0usize);

    let content = view! {
        {move || {
            convs.get().map(|wrap| {
                let list = (*wrap).clone();
                if list.is_empty() {
                    view! {
                        <p>"No conversations yet. "<a href="/c/new">"Start a new one."</a></p>
                    }.into_any()
                } else {
                    let total       = list.len();
                    let page_count  = total.div_ceil(PER_PAGE);
                    let cur_page    = page.get().min(page_count.saturating_sub(1));
                    let start       = cur_page * PER_PAGE;
                    let end         = (start + PER_PAGE).min(total);
                    let slice       = list[start..end].to_vec();

                    let rows = slice.into_iter().map(|c: shared::ConversationMeta| {
                        let href = format!("/c/{}", c.id);
                        let id   = c.id.clone();
                        view! {
                            <tr>
                                <td><a href={href}>{id}</a></td>
                                <td>{c.title}</td>
                                <td>{format_ts(c.last_msg_time)}</td>
                                <td>{format_ts(c.last_reply_time)}</td>
                            </tr>
                        }
                    }).collect::<Vec<_>>();

                    view! {
                        <p>{format!("Total: {} | Page {} / {}", total, cur_page + 1, page_count)}</p>
                        <table>
                            <thead>
                                <tr>
                                    <th>"ID"</th>
                                    <th>"Topic"</th>
                                    <th>"Last Message"</th>
                                    <th>"Last Reply"</th>
                                </tr>
                            </thead>
                            <tbody>{rows}</tbody>
                        </table>
                        {(page_count > 1).then(|| view! {
                            <p>
                                {(cur_page > 0).then(|| view! {
                                    <a href="#" on:click=move |ev| {
                                        ev.prevent_default();
                                        page.update(|p| *p = p.saturating_sub(1));
                                    }>"Previous"</a>
                                })}
                                " "
                                {(cur_page + 1 < page_count).then(|| view! {
                                    <a href="#" on:click=move |ev| {
                                        ev.prevent_default();
                                        page.update(|p| *p += 1);
                                    }>"Next"</a>
                                })}
                            </p>
                        })}
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

/// Format a Unix timestamp (seconds) as a human-readable string.
/// Returns an empty string for zero/missing timestamps (old records).
fn format_ts(ts: f64) -> String {
    if ts <= 0.0 { return String::new(); }
    #[cfg(feature = "hydrate")]
    {
        let ms  = ts * 1000.0;
        let d   = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(ms));
        d.to_locale_string("en-US", &wasm_bindgen::JsValue::UNDEFINED)
            .as_string()
            .unwrap_or_default()
    }
    #[cfg(not(feature = "hydrate"))]
    { format!("{ts:.0}") }
}
