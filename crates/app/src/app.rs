/// Root Leptos application component.

use leptos::prelude::*;
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};

use crate::{
    context::{
        auth::provide_auth_context,
        conversations::provide_conversation_context,
    },
    pages::{
        chat::ChatPage,
        conversations::ConversationsPage,
        home::HomePage,
        not_found::NotFound,
    },
};

#[component]
pub fn App() -> impl IntoView {
    provide_auth_context();
    provide_conversation_context();

    view! {
        <Router>
            <Routes fallback=|| view! { <NotFound/> }>
                <Route path=path!("/")              view=|| view! { <HomePage/> }/>
                <Route path=path!("/conversations") view=|| view! { <ConversationsPage/> }/>
                <Route path=path!("/c/new")         view=|| view! { <ChatPage/> }/>
                <Route path=path!("/c/:id")         view=|| view! { <ChatPage/> }/>
            </Routes>
        </Router>
    }
}
