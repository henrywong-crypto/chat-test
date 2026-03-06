/// In-app toast notifications.
///
/// Usage:
/// 1.  Call `provide_toast_context()` once at the app root (e.g. `App`).
/// 2.  Render `<Toasts/>` somewhere in the view tree.
/// 3.  From any descendant: `use_toasts().push_error("oops")`.

use std::sync::atomic::{AtomicU32, Ordering};

use leptos::prelude::*;

static NEXT_ID: AtomicU32 = AtomicU32::new(1);

fn next_id() -> u32 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
pub enum ToastKind {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Clone, Debug)]
pub struct Toast {
    pub id:      u32,
    pub message: String,
    pub kind:    ToastKind,
}

// ── Context ───────────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct ToastContext(RwSignal<Vec<Toast>>);

impl ToastContext {
    pub fn push(&self, message: impl Into<String>, kind: ToastKind) {
        self.0.update(|v| {
            v.push(Toast { id: next_id(), message: message.into(), kind });
        });
    }
    pub fn push_error(&self, message: impl Into<String>) {
        self.push(message, ToastKind::Error);
    }
    pub fn push_info(&self, message: impl Into<String>) {
        self.push(message, ToastKind::Info);
    }
    pub fn dismiss(&self, id: u32) {
        self.0.update(|v| v.retain(|t| t.id != id));
    }
}

pub fn provide_toast_context() {
    provide_context(ToastContext(RwSignal::new(vec![])));
}

pub fn use_toasts() -> ToastContext {
    expect_context()
}

// ── Component ─────────────────────────────────────────────────────────────────

#[component]
pub fn Toasts() -> impl IntoView {
    let ctx = use_toasts();

    view! {
        <div class="toasts">
            {move || {
                ctx.0.get().into_iter().map(|t| {
                    let id = t.id;
                    let kind_class = match t.kind {
                        ToastKind::Info    => "toast toast-info",
                        ToastKind::Success => "toast toast-success",
                        ToastKind::Warning => "toast toast-warning",
                        ToastKind::Error   => "toast toast-error",
                    };
                    view! {
                        <div class=kind_class>
                            <span class="toast-msg">{t.message}</span>
                            <button class="toast-close"
                                on:click=move |_| ctx.dismiss(id)
                            >"×"</button>
                        </div>
                    }
                }).collect_view()
            }}
        </div>
    }
}
