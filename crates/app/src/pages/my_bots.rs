/// My Bots page — `/bots`.  Lists the current user's bots.

use leptos::prelude::*;
use templates::{Breadcrumb, NavLink, Page};

use crate::context::auth::use_auth;

#[component]
pub fn MyBotsPage() -> impl IntoView {
    let auth    = use_auth();
    let version = RwSignal::new(0u32);

    let bots = LocalResource::new(move || {
        let v     = version.get();
        let token = auth.get().map(|u| u.token).unwrap_or_default();
        async move {
            let _ = v;
            if token.is_empty() { return vec![]; }
            #[cfg(feature = "hydrate")]
            { crate::api::fetch_my_bots(&token).await.unwrap_or_default() }
            #[cfg(not(feature = "hydrate"))]
            { vec![] }
        }
    });

    let bot_list = view! {
        {move || {
            bots.get().map(|wrap| {
                let list = (*wrap).clone();
                if list.is_empty() {
                    view! { <p>"No bots yet. Create your first one above."</p> }.into_any()
                } else {
                    let user_id = auth.get().map(|u| u.id).unwrap_or_default();
                    let rows = list.into_iter().map(|bot: shared::Bot| {
                        let is_mine = bot.owner_user_id == user_id;
                        let edit_href = format!("/bots/{}/edit", bot.id);
                        let bot_id = bot.id.clone();
                        #[cfg(feature = "hydrate")]
                        let token = auth.get().map(|u| u.token).unwrap_or_default();
                        view! {
                            <tr>
                                <td>{bot.title}</td>
                                <td>{bot.description}</td>
                                <td>{format!("{:?}", bot.visibility)}</td>
                                <td>
                                    {if is_mine {
                                        view! {
                                            <a href={edit_href}>"Edit"</a>
                                            " "
                                            <a href="#" on:click=move |ev| {
                                                ev.prevent_default();
                                                #[cfg(feature = "hydrate")]
                                                {
                                                    let t = token.clone();
                                                    let id = bot_id.clone();
                                                    wasm_bindgen_futures::spawn_local(async move {
                                                        let _ = crate::api::delete_bot(&id, &t).await;
                                                        version.update(|v| *v += 1);
                                                    });
                                                }
                                                #[cfg(not(feature = "hydrate"))]
                                                let _ = &bot_id;
                                            }>"Delete"</a>
                                        }.into_any()
                                    } else {
                                        view! { <span>"-"</span> }.into_any()
                                    }}
                                </td>
                            </tr>
                        }
                    }).collect_view();
                    view! {
                        <table>
                            <tr><th>"Title"</th><th>"Description"</th><th>"Visibility"</th><th>"Actions"</th></tr>
                            {rows}
                        </table>
                    }.into_any()
                }
            })
        }}
    };

    Page {
        title: "My Bots".to_string(),
        breadcrumbs: vec![Breadcrumb::current("My Bots")],
        nav_links: vec![NavLink::new("Create Bot", "/bots/new")],
        info_rows: vec![],
        content: bot_list,
        subpages: vec![],
    }
    .into_view()
}
