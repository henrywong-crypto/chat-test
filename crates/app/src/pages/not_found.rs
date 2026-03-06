use leptos::prelude::*;

#[component]
pub fn NotFound() -> impl IntoView {
    view! {
        <div class="hub-page">
            <h1>"404 — Page not found"</h1>
            <p>"The page you are looking for does not exist."</p>
            <p><a href="/">"← Home"</a></p>
        </div>
    }
}
