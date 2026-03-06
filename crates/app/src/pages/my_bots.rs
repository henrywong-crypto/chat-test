/// My Bots page — `/bots`.  Lists the current user's bots.

use leptos::prelude::*;
use leptos_router::components::A;

use crate::components::bots::bot_card::BotCard;
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

    view! {
        <div class="page-content">
            <div class="page-header">
                <h1 class="page-title">"My Bots"</h1>
                <A href="/bots/new" attr:class="btn btn-primary">"+ Create Bot"</A>
            </div>

            <div class="bot-grid">
                {move || {
                    bots.get().map(|wrap| {
                        let list = (*wrap).clone();
                        if list.is_empty() {
                            view! {
                                <div class="empty-state">
                                    <h2>"No bots yet"</h2>
                                    <p>"Create your first bot to give it a custom instruction and model."</p>
                                </div>
                            }.into_any()
                        } else {
                            let user_id = auth.get().map(|u| u.id).unwrap_or_default();
                            list.into_iter().map(|bot: shared::Bot| {
                                let is_mine = bot.owner_user_id == user_id;
                                #[cfg(feature = "hydrate")]
                                let token = auth.get().map(|u| u.token).unwrap_or_default();
                                view! {
                                    <BotCard
                                        bot=bot
                                        editable=is_mine
                                        on_delete=move |id| {
                                            #[cfg(feature = "hydrate")]
                                            {
                                                let t = token.clone();
                                                wasm_bindgen_futures::spawn_local(async move {
                                                    let _ = crate::api::delete_bot(&id, &t).await;
                                                    version.update(|v| *v += 1);
                                                });
                                            }
                                            #[cfg(not(feature = "hydrate"))]
                                            let _ = &id;
                                        }
                                        on_use=move |_| {}
                                    />
                                }
                            }).collect_view().into_any()
                        }
                    })
                }}
            </div>
        </div>
    }
}
