use leptos::prelude::*;

#[component]
pub fn NotFound() -> impl IntoView {
    view! {
        <div class="page-center">
            <div class="empty-state">
                <h2>"404 — Page not found"</h2>
                <p>"The page you are looking for does not exist."</p>
            </div>
        </div>
    }
}
