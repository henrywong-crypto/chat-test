/// Admin users page — `/admin/users`.
/// Lists all Cognito users and allows toggling their group membership inline.

use leptos::prelude::*;
use shared::{AdminUserRecord, UserGroup};
#[cfg(feature = "hydrate")]
use shared::UpdateUserGroupsRequest;
use templates::{Breadcrumb, Page};

use crate::context::auth::use_auth;

// ── AdminUsersPage ─────────────────────────────────────────────────────────────

#[component]
pub fn AdminUsersPage() -> impl IntoView {
    let auth    = use_auth();
    let version = RwSignal::new(0u32);

    let users = LocalResource::new(move || {
        let v     = version.get();
        let token = auth.get().map(|u| u.token).unwrap_or_default();
        async move {
            let _ = v;
            if token.is_empty() { return vec![]; }
            #[cfg(feature = "hydrate")]
            { crate::api::fetch_admin_users(&token).await.map(|r| r.users).unwrap_or_default() }
            #[cfg(not(feature = "hydrate"))]
            { vec![] }
        }
    });

    let is_admin = move || auth.get().map(|u| u.is_admin).unwrap_or(false);

    let users_table = view! {
        <Show
            when=is_admin
            fallback=|| view! { <p class="admin-denied">"Access denied — admin only."</p> }
        >
            {move || {
                users.get().map(|wrap| {
                    let list = (*wrap).clone();
                    if list.is_empty() {
                        view! {
                            <div class="empty-state">
                                <h2>"No users"</h2>
                                <p>"No users found in the Cognito pool."</p>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div class="admin-table-wrap">
                                <table class="admin-table">
                                    <thead>
                                        <tr>
                                            <th>"Email"</th>
                                            <th>"ID"</th>
                                            <th class="center">"Admin"</th>
                                            <th class="center">"Create Bots"</th>
                                            <th class="center">"Publish"</th>
                                            <th class="center">"Status"</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {list.into_iter().map(|user: AdminUserRecord| {
                                            view! { <UserRow user=user version=version /> }
                                        }).collect_view()}
                                    </tbody>
                                </table>
                            </div>
                        }.into_any()
                    }
                })
            }}
        </Show>
    };

    Page {
        title: "Users".to_string(),
        breadcrumbs: vec![Breadcrumb::current("Users")],
        nav_links: vec![],
        info_rows: vec![],
        content: users_table,
        subpages: vec![],
    }
    .into_view()
}

// ── UserRow ───────────────────────────────────────────────────────────────────

#[component]
fn UserRow(user: AdminUserRecord, version: RwSignal<u32>) -> impl IntoView {
    let auth = use_auth();
    #[cfg(not(feature = "hydrate"))]
    let _ = version;

    let has_admin    = user.groups.contains(&UserGroup::Admin);
    let has_bots     = user.groups.contains(&UserGroup::CreatingBotAllowed);
    let has_publish  = user.groups.contains(&UserGroup::PublishAllowed);

    let admin_sig   = RwSignal::new(has_admin);
    let bots_sig    = RwSignal::new(has_bots);
    let publish_sig = RwSignal::new(has_publish);

    let user_id = user.id.clone();
    let email   = user.email.clone();
    let enabled = user.enabled;

    let toggle = move |group: UserGroup, is_checked: bool| {
        let uid   = user_id.clone();
        let token = auth.get_untracked().map(|u| u.token).unwrap_or_default();
        #[cfg(feature = "hydrate")]
        {
            let req = if is_checked {
                UpdateUserGroupsRequest { add_groups: vec![group], remove_groups: vec![] }
            } else {
                UpdateUserGroupsRequest { add_groups: vec![], remove_groups: vec![group] }
            };
            wasm_bindgen_futures::spawn_local(async move {
                let _ = crate::api::update_user_groups(&uid, &req, &token).await;
                version.update(|v| *v += 1);
            });
        }
        #[cfg(not(feature = "hydrate"))]
        let _ = (&uid, &token, &group, &is_checked);
    };

    let toggle_admin   = {
        let t = toggle.clone();
        move |ev: leptos::ev::Event| {
            let checked = event_target_checked(&ev);
            admin_sig.set(checked);
            t(UserGroup::Admin, checked);
        }
    };
    let toggle_bots    = {
        let t = toggle.clone();
        move |ev: leptos::ev::Event| {
            let checked = event_target_checked(&ev);
            bots_sig.set(checked);
            t(UserGroup::CreatingBotAllowed, checked);
        }
    };
    let toggle_publish = {
        let t = toggle.clone();
        move |ev: leptos::ev::Event| {
            let checked = event_target_checked(&ev);
            publish_sig.set(checked);
            t(UserGroup::PublishAllowed, checked);
        }
    };

    view! {
        <tr>
            <td class="email-cell">{email}</td>
            <td class="mono truncate id-cell">{user.id}</td>
            <td class="center">
                <input
                    type="checkbox"
                    prop:checked=admin_sig
                    on:change=toggle_admin
                />
            </td>
            <td class="center">
                <input
                    type="checkbox"
                    prop:checked=bots_sig
                    on:change=toggle_bots
                />
            </td>
            <td class="center">
                <input
                    type="checkbox"
                    prop:checked=publish_sig
                    on:change=toggle_publish
                />
            </td>
            <td class="center">
                {if enabled {
                    view! { <span class="badge badge-active">"Active"</span> }.into_any()
                } else {
                    view! { <span class="badge badge-inactive">"Disabled"</span> }.into_any()
                }}
            </td>
        </tr>
    }
}
