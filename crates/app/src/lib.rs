/// Root crate lib.  Re-exports the Leptos App component and, behind
/// `#[cfg(feature = "ssr")]`, all server-side modules.

pub mod app;
pub mod api;
pub mod components;
pub mod context;
pub mod pages;

#[cfg(feature = "ssr")]
pub mod server;

// WASM hydration entry point.
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::App;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
