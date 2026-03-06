/// Full Settings modal — three tabs: Profile, Inference Profiles, Preferences.

use leptos::prelude::*;
use shared::{InferenceProfile, ModelInfo};

use crate::components::modal::Modal;
use crate::context::auth::use_auth;
use crate::context::settings::use_settings_context;

// ── SettingsModal ─────────────────────────────────────────────────────────────

#[component]
pub fn SettingsModal() -> impl IntoView {
    let ctx  = use_settings_context();
    let show = Signal::derive(move || ctx.open.get());

    let (tab, set_tab) = signal("profile");

    view! {
        <Modal
            show=show
            on_close=move || ctx.open.set(false)
            title="Settings"
        >
            // ── Tab bar ───────────────────────────────────────────────────────
            <div class="settings-tabs">
                {["profile", "inference", "preferences"].map(|t| {
                    let label = match t {
                        "inference"   => "Inference Profiles",
                        "preferences" => "Preferences",
                        _             => "Profile",
                    };
                    view! {
                        <button
                            class=move || if tab.get() == t {
                                "tab-btn tab-btn-active"
                            } else {
                                "tab-btn"
                            }
                            on:click=move |_| set_tab.set(t)
                        >
                            {label}
                        </button>
                    }
                })}
            </div>

            // ── Tab content ───────────────────────────────────────────────────
            <div class="settings-body">
                {move || match tab.get() {
                    "inference"   => view! { <InferenceProfilesTab/> }.into_any(),
                    "preferences" => view! { <PreferencesTab/> }.into_any(),
                    _             => view! { <ProfileTab/> }.into_any(),
                }}
            </div>
        </Modal>
    }
}

// ── Profile tab ───────────────────────────────────────────────────────────────

#[component]
fn ProfileTab() -> impl IntoView {
    let auth = use_auth();

    view! {
        <div class="settings-tab-content">
            {move || {
                let user = auth.get();
                let email   = user.as_ref().map(|u| u.email.clone())
                    .unwrap_or_else(|| "Not signed in".into());
                let user_id = user.as_ref().map(|u| u.id.clone())
                    .unwrap_or_default();
                let role    = user.as_ref().map(|u| {
                    if u.is_admin { "Admin" } else { "User" }
                }).unwrap_or("—");
                view! {
                    <table class="info-table">
                        <tr>
                            <th>"Email"</th>
                            <td>{email}</td>
                        </tr>
                        <tr>
                            <th>"User ID"</th>
                            <td class="mono truncate">{user_id}</td>
                        </tr>
                        <tr>
                            <th>"Role"</th>
                            <td>{role}</td>
                        </tr>
                    </table>
                }
            }}

            <div class="settings-actions">
                <button
                    class="btn btn-danger"
                    on:click=move |_| {
                        #[cfg(feature = "hydrate")]
                        {
                            // Clear localStorage tokens and reload to the login page.
                            if let Some(w) = web_sys::window() {
                                let _ = w.local_storage().ok().flatten().map(|s| {
                                    let len = s.length().unwrap_or(0);
                                    let keys: Vec<String> = (0..len)
                                        .filter_map(|i| s.key(i).ok().flatten())
                                        .collect();
                                    for k in keys {
                                        if k.contains("CognitoIdentityServiceProvider")
                                            || k == "id_token" || k == "access_token"
                                        {
                                            let _ = s.remove_item(&k);
                                        }
                                    }
                                });
                                let _ = w.location().reload();
                            }
                        }
                    }
                >
                    "Sign out"
                </button>
            </div>
        </div>
    }
}

// ── Inference Profiles tab ────────────────────────────────────────────────────

#[component]
fn InferenceProfilesTab() -> impl IntoView {
    let auth    = use_auth();
    let version = RwSignal::new(0u32);

    let profiles = LocalResource::new(move || {
        let v     = version.get();
        let token = auth.get().map(|u| u.token).unwrap_or_default();
        async move {
            let _ = v;
            if token.is_empty() { return Vec::<InferenceProfile>::new(); }
            #[cfg(feature = "hydrate")]
            { crate::api::fetch_inference_profiles(&token).await.unwrap_or_default() }
            #[cfg(not(feature = "hydrate"))]
            { Vec::<InferenceProfile>::new() }
        }
    });

    let models = LocalResource::new(move || async move {
        #[cfg(feature = "hydrate")]
        { crate::api::fetch_models().await.unwrap_or_default() }
        #[cfg(not(feature = "hydrate"))]
        { Vec::<ModelInfo>::new() }
    });

    let (new_model_id, set_new_model_id) = signal(String::new());
    let (creating, set_creating) = signal(false);

    view! {
        <div class="settings-tab-content">
            // Profiles table
            <div class="profiles-table-wrap">
                {move || {
                    profiles.get().map(|wrap| {
                        let list = (*wrap).clone();
                        if list.is_empty() {
                            view! {
                                <p class="empty-hint">"No inference profiles yet."</p>
                            }.into_any()
                        } else {
                            view! {
                                <table class="data-table">
                                    <thead>
                                        <tr>
                                            <th>"Model"</th>
                                            <th>"Status"</th>
                                            <th>"Region"</th>
                                            <th></th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {list.into_iter().map(|p| {
                                            let model = p.model_id.clone();
                                            let status = format!("{:?}", p.status);
                                            let region = p.region.clone();
                                            view! {
                                                <tr>
                                                    <td class="mono">{model.clone()}</td>
                                                    <td>{status}</td>
                                                    <td>{region}</td>
                                                    <td>
                                                        <button
                                                            class="btn btn-sm btn-danger"
                                                            on:click=move |_| {
                                                                #[cfg(feature = "hydrate")]
                                                                {
                                                                    let model = model.clone();
                                                                    let token = auth.get().map(|u| u.token).unwrap_or_default();
                                                                    wasm_bindgen_futures::spawn_local(async move {
                                                                        let _ = crate::api::delete_inference_profile(&model, &token).await;
                                                                        version.update(|v| *v += 1);
                                                                    });
                                                                }
                                                            }
                                                        >
                                                            "Delete"
                                                        </button>
                                                    </td>
                                                </tr>
                                            }
                                        }).collect_view()}
                                    </tbody>
                                </table>
                            }.into_any()
                        }
                    })
                }}
            </div>

            // Create new profile
            <div class="create-profile-form">
                <h3 class="form-section-title">"Create Profile"</h3>
                <div class="form-row">
                    <select
                        class="model-selector"
                        prop:value=new_model_id
                        on:change=move |ev| set_new_model_id.set(event_target_value(&ev))
                    >
                        <option value="">"— select model —"</option>
                        {move || {
                            models.get().map(|wrap| {
                                (*wrap).clone().into_iter().map(|m: ModelInfo| view! {
                                    <option value=m.id.clone()>{m.display_name}</option>
                                }).collect_view()
                            })
                        }}
                    </select>
                    <button
                        class="btn btn-primary"
                        prop:disabled=creating
                        on:click=move |_| {
                            let mid = new_model_id.get_untracked();
                            if mid.is_empty() { return; }
                            set_creating.set(true);
                            #[cfg(feature = "hydrate")]
                            {
                                let token = auth.get_untracked().map(|u| u.token).unwrap_or_default();
                                wasm_bindgen_futures::spawn_local(async move {
                                    let _ = crate::api::create_inference_profile(&mid, &token).await;
                                    set_creating.set(false);
                                    version.update(|v| *v += 1);
                                });
                            }
                            #[cfg(not(feature = "hydrate"))]
                            set_creating.set(false);
                        }
                    >
                        {move || if creating.get() { "Creating…" } else { "Create" }}
                    </button>
                </div>
            </div>
        </div>
    }
}

// ── Preferences tab ───────────────────────────────────────────────────────────

#[component]
fn PreferencesTab() -> impl IntoView {
    let (dark, set_dark) = signal({
        #[cfg(feature = "hydrate")]
        {
            web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.document_element())
                .and_then(|el| el.get_attribute("data-theme"))
                .map(|t| t == "dark")
                .unwrap_or(false)
        }
        #[cfg(not(feature = "hydrate"))]
        { false }
    });

    view! {
        <div class="settings-tab-content">
            <label class="toggle-row">
                <span class="toggle-label">"Dark mode"</span>
                <input
                    type="checkbox"
                    class="toggle"
                    prop:checked=dark
                    on:change=move |ev| {
                        let checked = event_target_checked(&ev);
                        set_dark.set(checked);
                        #[cfg(feature = "hydrate")]
                        {
                            let theme = if checked { "dark" } else { "light" };
                            if let Some(el) = web_sys::window()
                                .and_then(|w| w.document())
                                .and_then(|d| d.document_element())
                            {
                                let _ = el.set_attribute("data-theme", theme);
                                // Persist to localStorage.
                                if let Some(storage) = web_sys::window()
                                    .and_then(|w| w.local_storage().ok().flatten())
                                {
                                    let _ = storage.set_item("theme", theme);
                                }
                            }
                        }
                    }
                />
            </label>
        </div>
    }
}
