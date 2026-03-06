use leptos::prelude::*;
use shared::GenerationParams;

#[component]
pub fn GenerationParamsEditor(params: RwSignal<GenerationParams>) -> impl IntoView {
    view! {
        <div class="gen-params">
            // Max tokens
            <div class="form-group">
                <label class="form-label">"Max tokens"
                    <span class="form-hint">
                        {move || params.get().max_tokens.to_string()}
                    </span>
                </label>
                <input
                    type="number"
                    class="form-input"
                    min="1"
                    max="32000"
                    prop:value=move || params.get().max_tokens
                    on:input=move |ev| {
                        if let Ok(v) = event_target_value(&ev).parse::<u32>() {
                            params.update(|p| p.max_tokens = v);
                        }
                    }
                />
            </div>

            // Temperature
            <div class="form-group">
                <label class="form-label">"Temperature"
                    <span class="form-hint">
                        {move || format!("{:.2}", params.get().temperature)}
                    </span>
                </label>
                <input
                    type="range"
                    class="form-range"
                    min="0"
                    max="1"
                    step="0.01"
                    prop:value=move || params.get().temperature.to_string()
                    on:input=move |ev| {
                        if let Ok(v) = event_target_value(&ev).parse::<f32>() {
                            params.update(|p| p.temperature = v);
                        }
                    }
                />
            </div>

            // Top-P
            <div class="form-group">
                <label class="form-label">"Top-P"
                    <span class="form-hint">
                        {move || format!("{:.2}", params.get().top_p)}
                    </span>
                </label>
                <input
                    type="range"
                    class="form-range"
                    min="0"
                    max="1"
                    step="0.01"
                    prop:value=move || params.get().top_p.to_string()
                    on:input=move |ev| {
                        if let Ok(v) = event_target_value(&ev).parse::<f32>() {
                            params.update(|p| p.top_p = v);
                        }
                    }
                />
            </div>

            // Stop sequences
            <div class="form-group">
                <label class="form-label">"Stop sequences"</label>
                <input
                    type="text"
                    class="form-input"
                    placeholder="Comma-separated, e.g.  Human:, AI:"
                    prop:value=move || params.get().stop_sequences.join(", ")
                    on:input=move |ev| {
                        let raw = event_target_value(&ev);
                        let seqs: Vec<String> = raw
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                        params.update(|p| p.stop_sequences = seqs);
                    }
                />
            </div>
        </div>
    }
}
