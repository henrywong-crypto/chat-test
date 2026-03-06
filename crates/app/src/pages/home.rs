/// Home page — `/`.

use leptos::prelude::*;
use templates::{Page, Subpage};

#[component]
pub fn HomePage() -> impl IntoView {
    Page {
        title: "Bedrock RS".to_string(),
        subpages: vec![
            Subpage::new("Conversations", "/conversations", ""),
        ],
        ..Default::default()
    }
    .into_view()
}
