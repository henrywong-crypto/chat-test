/// Generic modal overlay with title bar and close button.

use leptos::prelude::*;

#[component]
pub fn Modal(
    show:     Signal<bool>,
    on_close: impl Fn() + 'static + Clone + Send + Sync,
    #[prop(into)] title: String,
    children: Children,
) -> impl IntoView {
    // Materialise children eagerly — Children is FnOnce.
    // Visibility is controlled via a CSS class so we never need to capture
    // body (AnyView, !Sync) inside a reactive closure.
    let body      = children();
    let on_close2 = on_close.clone();

    view! {
        <div
            class=move || if show.get() { "modal-backdrop" } else { "modal-backdrop modal-hidden" }
            on:click=move |_| on_close()
        >
            <div
                class="modal-panel"
                on:click=|ev| ev.stop_propagation()
            >
                <div class="modal-header">
                    <h2 class="modal-title">{title}</h2>
                    <button
                        class="btn-icon modal-close"
                        on:click=move |_| on_close2()
                    >
                        "×"
                    </button>
                </div>
                <div class="modal-body">
                    {body}
                </div>
            </div>
        </div>
    }
}
