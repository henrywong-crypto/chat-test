/// Bot editor page — `/bots/new` (create) or `/bots/:id/edit` (update).

use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map};
use shared::{Bot, BotVisibility, GenerationParams, ModelInfo};
#[cfg(feature = "hydrate")]
use shared::{CreateBotRequest, UpdateBotRequest};
use templates::{Breadcrumb, NavLink, Page};

use crate::components::bots::generation_params_editor::GenerationParamsEditor;
use crate::context::auth::use_auth;

// ── BotEditorPage ─────────────────────────────────────────────────────────────

#[component]
pub fn BotEditorPage() -> impl IntoView {
    let auth     = use_auth();
    let params   = use_params_map();
    let navigate = use_navigate();

    // Route param — present when editing, absent when creating.
    let bot_id = move || params.with(|p| p.get("id").map(|s| s.to_string()));
    let is_edit = move || bot_id().is_some();

    // ── Form state ────────────────────────────────────────────────────────────
    let (title,       set_title)       = signal(String::new());
    let (description, set_description) = signal(String::new());
    let (instruction, set_instruction) = signal(String::new());
    let (model_id,    set_model_id)    = signal(String::new());
    let (visibility,  set_visibility)  = signal("private".to_string());
    let show_params  = RwSignal::new(false);
    let gen_params   = RwSignal::new(GenerationParams::default());
    let (submitting, set_submitting)   = signal(false);
    let (error_msg,  set_error_msg)    = signal::<Option<String>>(None);

    // ── Load existing bot if editing ─────────────────────────────────────────
    let bot_res = LocalResource::new(move || {
        let id    = bot_id();
        let token = auth.get().map(|u| u.token).unwrap_or_default();
        async move {
            let Some(id) = id else { return None::<Bot>; };
            #[cfg(not(feature = "hydrate"))]
            let _ = &id;
            if token.is_empty() { return None; }
            #[cfg(feature = "hydrate")]
            { crate::api::fetch_bot(&id, &token).await.ok() }
            #[cfg(not(feature = "hydrate"))]
            { None }
        }
    });

    // Populate form when bot loads.
    Effect::new(move |_| {
        if let Some(wrap) = bot_res.get() {
            if let Some(bot) = (*wrap).clone() {
                set_title.set(bot.title);
                set_description.set(bot.description);
                set_instruction.set(bot.instruction);
                set_model_id.set(bot.model_id.unwrap_or_default());
                set_visibility.set(match bot.visibility {
                    BotVisibility::Public   => "public",
                    BotVisibility::Unlisted => "unlisted",
                    BotVisibility::Private  => "private",
                }.to_string());
                gen_params.set(bot.generation_params);
            }
        }
    });

    let models = LocalResource::new(move || async move {
        #[cfg(feature = "hydrate")]
        { crate::api::fetch_models().await.unwrap_or_default() }
        #[cfg(not(feature = "hydrate"))]
        { vec![] }
    });

    // ── Submit ────────────────────────────────────────────────────────────────
    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if submitting.get_untracked() { return; }

        set_submitting.set(true);
        set_error_msg.set(None);

        let vis = match visibility.get_untracked().as_str() {
            "public"   => BotVisibility::Public,
            "unlisted" => BotVisibility::Unlisted,
            _          => BotVisibility::Private,
        };
        let mid = model_id.get_untracked();
        let nav = navigate.clone();
        #[cfg(not(feature = "hydrate"))]
        let _ = (&vis, &mid, &nav);

        #[cfg(feature = "hydrate")]
        {
            let token   = auth.get_untracked().map(|u| u.token).unwrap_or_default();
            let title_v = title.get_untracked();
            let desc_v  = description.get_untracked();
            let instr_v = instruction.get_untracked();
            let gp      = gen_params.get_untracked();
            let edit_id = bot_id();

            wasm_bindgen_futures::spawn_local(async move {
                let result = if let Some(id) = edit_id {
                    let req = UpdateBotRequest {
                        title:             Some(title_v),
                        description:       Some(desc_v),
                        instruction:       Some(instr_v),
                        model_id:          if mid.is_empty() { None } else { Some(mid) },
                        generation_params: Some(gp),
                        visibility:        Some(vis),
                        knowledge_base_id: None,
                    };
                    crate::api::update_bot(&id, &req, &token).await.map(|_| ())
                } else {
                    let req = CreateBotRequest {
                        title:             title_v,
                        description:       desc_v,
                        instruction:       instr_v,
                        model_id:          if mid.is_empty() { None } else { Some(mid) },
                        generation_params: Some(gp),
                        knowledge_base_id: None,
                        visibility:        vis,
                    };
                    crate::api::create_bot(&req, &token).await.map(|_| ())
                };

                set_submitting.set(false);
                match result {
                    Ok(_)    => nav("/bots", Default::default()),
                    Err(err) => set_error_msg.set(Some(err)),
                }
            });
        }

        #[cfg(not(feature = "hydrate"))]
        set_submitting.set(false);
    };

    let form = view! {
        <form on:submit=on_submit>
            {move || error_msg.get().map(|e| view! { <p>{e}</p> })}

            <table>
                <tr>
                    <td><label for="bot-title">"Title"</label></td>
                    <td>
                        <input id="bot-title" type="text" required=true
                            placeholder="My Assistant"
                            prop:value=title
                            on:input=move |ev| set_title.set(event_target_value(&ev))
                        />
                    </td>
                </tr>
                <tr>
                    <td><label for="bot-desc">"Description"</label></td>
                    <td>
                        <input id="bot-desc" type="text"
                            placeholder="Short description shown in the Bot Store"
                            prop:value=description
                            on:input=move |ev| set_description.set(event_target_value(&ev))
                        />
                    </td>
                </tr>
                <tr>
                    <td><label for="bot-instr">"Instruction"</label></td>
                    <td>
                        <textarea id="bot-instr" rows="6" required=true
                            placeholder="You are a helpful assistant that…"
                            prop:value=instruction
                            on:input=move |ev| set_instruction.set(event_target_value(&ev))
                        />
                    </td>
                </tr>
                <tr>
                    <td><label for="bot-model">"Model"</label></td>
                    <td>
                        <select id="bot-model" prop:value=model_id
                            on:change=move |ev| set_model_id.set(event_target_value(&ev))
                        >
                            <option value="">"User default"</option>
                            {move || {
                                models.get().map(|wrap| {
                                    (*wrap).clone().into_iter().map(|m: ModelInfo| view! {
                                        <option value=m.id.clone()>{m.display_name}</option>
                                    }).collect::<Vec<_>>()
                                })
                            }}
                        </select>
                    </td>
                </tr>
                <tr>
                    <td><label for="bot-vis">"Visibility"</label></td>
                    <td>
                        <select id="bot-vis" prop:value=visibility
                            on:change=move |ev| set_visibility.set(event_target_value(&ev))
                        >
                            <option value="private">"Private"</option>
                            <option value="unlisted">"Unlisted"</option>
                            <option value="public">"Public"</option>
                        </select>
                    </td>
                </tr>
            </table>

            <p>
                <button type="button"
                    on:click=move |_| show_params.update(|v| *v = !*v)
                >
                    {move || if show_params.get() { "▲ Hide parameters" } else { "▼ Show parameters" }}
                </button>
            </p>

            {move || show_params.get().then(|| view! {
                <GenerationParamsEditor params=gen_params/>
            })}

            <p>
                <a href="/bots">"Cancel"</a>
                " "
                <input type="submit"
                    prop:value=move || if submitting.get() { "Saving…" } else if is_edit() { "Save changes" } else { "Create bot" }
                    prop:disabled=submitting
                />
            </p>
        </form>
    };

    Page {
        title: if is_edit() { "Edit Bot".to_string() } else { "Create Bot".to_string() },
        breadcrumbs: vec![
            Breadcrumb::link("My Bots", "/bots"),
            Breadcrumb::current(if is_edit() { "Edit" } else { "New" }),
        ],
        nav_links: vec![NavLink::back()],
        info_rows: vec![],
        content: form,
        subpages: vec![],
    }
    .into_view()
}
