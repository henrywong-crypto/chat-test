/// Left-hand navigation sidebar.

use leptos::prelude::*;
use leptos_router::{components::A, hooks::use_location};
use shared::ConversationMeta;

use crate::context::auth::use_auth;
use crate::context::conversations::use_conversation_context;

#[component]
pub fn Sidebar() -> impl IntoView {
    let auth     = use_auth();
    let conv_ctx = use_conversation_context();

    let conversations = LocalResource::new(move || {
        let version = conv_ctx.version.get();
        let token   = auth.get().map(|u| u.token).unwrap_or_default();
        async move {
            let _ = version;
            if token.is_empty() {
                return Vec::<ConversationMeta>::new();
            }
            #[cfg(feature = "hydrate")]
            {
                crate::api::fetch_conversations(&token).await.unwrap_or_default()
            }
            #[cfg(not(feature = "hydrate"))]
            {
                Vec::<ConversationMeta>::new()
            }
        }
    });

    let location = use_location();

    view! {
        <nav class="sidebar">
            <div class="sidebar-header">
                <A href="/" attr:class="sidebar-logo">"Bedrock RS"</A>
                <A href="/" attr:class="btn-new-chat">"+  New"</A>
            </div>

            <div class="sidebar-conversations">
                {move || {
                    let path = location.pathname.get();
                    conversations.get().map(|wrap| {
                        let list = (*wrap).clone();
                        list.into_iter().map(move |conv: ConversationMeta| {
                            let href   = format!("/c/{}", conv.id);
                            let active = path == href;
                            let class  = if active {
                                "conv-item conv-item-active"
                            } else {
                                "conv-item"
                            };
                            view! {
                                <A href=href attr:class=class>
                                    <span class="conv-title">{conv.title}</span>
                                </A>
                            }
                        }).collect_view()
                    })
                }}
            </div>

            <div class="sidebar-nav">
                {move || {
                    let path = location.pathname.get();
                    let bots_active       = path.starts_with("/bots") && !path.starts_with("/bots/store");
                    let bots_store_active = path.starts_with("/bots/store");
                    let admin_users_active    = path == "/admin/users";
                    let admin_analytics_active = path == "/admin/analytics";
                    let is_admin = auth.get().map(|u| u.is_admin).unwrap_or(false);

                    view! {
                        <A
                            href="/bots"
                            attr:class=if bots_active { "nav-item nav-item-active" } else { "nav-item" }
                        >
                            <span class="nav-icon">"🤖"</span>
                            "My Bots"
                        </A>
                        <A
                            href="/bots/store"
                            attr:class=if bots_store_active { "nav-item nav-item-active" } else { "nav-item" }
                        >
                            <span class="nav-icon">"🏪"</span>
                            "Bot Store"
                        </A>

                        // Admin section — only shown to admins
                        {is_admin.then(|| view! {
                            <div class="sidebar-section-label">"Admin"</div>
                            <A
                                href="/admin/users"
                                attr:class=if admin_users_active { "nav-item nav-item-active" } else { "nav-item" }
                            >
                                <span class="nav-icon">"👤"</span>
                                "Users"
                            </A>
                            <A
                                href="/admin/analytics"
                                attr:class=if admin_analytics_active { "nav-item nav-item-active" } else { "nav-item" }
                            >
                                <span class="nav-icon">"📊"</span>
                                "Analytics"
                            </A>
                        })}
                    }
                }}
            </div>

            <div class="sidebar-footer">
                {move || {
                    let user = auth.get();
                    let initial = user.as_ref()
                        .and_then(|u| u.email.chars().next())
                        .map(|c| c.to_uppercase().to_string())
                        .unwrap_or_else(|| "?".into());
                    let label = user
                        .map(|u| u.email)
                        .unwrap_or_else(|| "Not signed in".into());
                    view! {
                        <div class="user-row">
                            <div class="user-avatar">{initial}</div>
                            <span class="user-name">{label}</span>
                        </div>
                    }
                }}
            </div>
        </nav>
    }
}
