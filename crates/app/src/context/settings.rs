/// Settings modal open/close context.

use leptos::prelude::*;

#[derive(Clone, Copy)]
pub struct SettingsContext {
    pub open: RwSignal<bool>,
}

pub fn provide_settings_context() {
    provide_context(SettingsContext { open: RwSignal::new(false) });
}

pub fn use_settings_context() -> SettingsContext {
    expect_context()
}
