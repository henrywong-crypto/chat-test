use leptos::{ev::KeyboardEvent, prelude::*};

#[component]
pub fn MessageInput(
    on_send:  impl Fn(String) + 'static,
    disabled: Signal<bool>,
) -> impl IntoView {
    let (text, set_text)       = signal(String::new());
    let (pending, set_pending) = signal::<Option<String>>(None);

    // Fire on_send reactively whenever a pending value appears.
    // The closure captures only signals (Copy) plus the `on_send` callback.
    Effect::new(move |_| {
        if let Some(t) = pending.get() {
            set_pending.set(None);
            on_send(t);
        }
    });

    // This closure captures only Copy signals — so it is Clone.
    let do_send = move || {
        let t = text.get_untracked();
        if !t.trim().is_empty() {
            set_text.set(String::new());
            set_pending.set(Some(t));
        }
    };
    let do_send2 = do_send.clone();

    view! {
        <div class="message-input-bar">
            <textarea
                class="message-input"
                placeholder="Message Bedrock…"
                rows="1"
                prop:value=text
                prop:disabled=disabled
                on:input=move |ev| set_text.set(event_target_value(&ev))
                on:keydown=move |ev: KeyboardEvent| {
                    if ev.key() == "Enter" && !ev.shift_key() {
                        ev.prevent_default();
                        do_send2();
                    }
                }
            />
            <button
                class="btn-send"
                prop:disabled=disabled
                on:click=move |_| do_send()
            >
                "Send"
            </button>
        </div>
    }
}
