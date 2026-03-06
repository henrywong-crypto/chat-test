/// Grid shell: sidebar on the left, main column (navbar + content) on the right.

use leptos::prelude::*;

use super::{navbar::Navbar, sidebar::Sidebar};

#[component]
pub fn AppShell(children: Children) -> impl IntoView {
    view! {
        <div class="app-shell">
            <Sidebar/>
            <div class="app-main">
                <Navbar/>
                {children()}
            </div>
        </div>
    }
}
