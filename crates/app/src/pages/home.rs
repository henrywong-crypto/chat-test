/// Home page — `/`.

use leptos::prelude::*;
use templates::{Page, Subpage};

use crate::context::auth::use_auth;

#[component]
pub fn HomePage() -> impl IntoView {
    let auth     = use_auth();
    let is_admin = move || auth.get().map(|u| u.is_admin).unwrap_or(false);

    move || {
        let mut subpages = vec![
            Subpage::new("Conversations", "/conversations", ""),
            Subpage::new("My Bots",       "/bots",         ""),
            Subpage::new("Bot Store",     "/bots/store",   ""),
        ];
        if is_admin() {
            subpages.push(Subpage::new("Admin: Users",     "/admin/users",     ""));
            subpages.push(Subpage::new("Admin: Analytics", "/admin/analytics", ""));
        }
        Page {
            title: "Bedrock RS".to_string(),
            subpages,
            ..Default::default()
        }
        .into_view()
        .into_any()
    }
}
