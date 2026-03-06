use leptos::prelude::*;
use shared::ModelInfo;

#[component]
pub fn ModelSelector(
    selected: RwSignal<Option<String>>,
    models:   Signal<Vec<ModelInfo>>,
) -> impl IntoView {
    view! {
        <select
            class="model-selector"
            prop:value=move || selected.get().unwrap_or_default()
            on:change=move |ev| {
                let v = event_target_value(&ev);
                selected.set(if v.is_empty() { None } else { Some(v) });
            }
        >
            <option value="">"Default model"</option>
            {move || {
                models.get().into_iter().map(|m| {
                    view! {
                        <option value=m.id.clone()>{m.display_name}</option>
                    }
                }).collect_view()
            }}
        </select>
    }
}
