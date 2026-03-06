/// Root Leptos application component.

use leptos::prelude::*;
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};

use crate::{
    components::{
        app_shell::AppShell,
        settings::settings_modal::SettingsModal,
        toast::{provide_toast_context, Toasts},
    },
    context::{
        auth::provide_auth_context,
        conversations::provide_conversation_context,
        settings::provide_settings_context,
    },
    pages::{
        admin::{
            analytics::AdminAnalyticsPage,
            users::AdminUsersPage,
        },
        bot_editor::BotEditorPage,
        bot_store::BotStorePage,
        chat::ChatPage,
        my_bots::MyBotsPage,
        not_found::NotFound,
    },
};

#[component]
pub fn App() -> impl IntoView {
    provide_auth_context();
    provide_conversation_context();
    provide_settings_context();
    provide_toast_context();

    view! {
        <Router>
            <AppShell>
                <Routes fallback=|| view! { <NotFound/> }>
                    <Route path=path!("/")                    view=|| view! { <ChatPage/> }/>
                    <Route path=path!("/c/:id")               view=|| view! { <ChatPage/> }/>
                    <Route path=path!("/bots")                view=|| view! { <MyBotsPage/> }/>
                    <Route path=path!("/bots/store")          view=|| view! { <BotStorePage/> }/>
                    <Route path=path!("/bots/new")            view=|| view! { <BotEditorPage/> }/>
                    <Route path=path!("/bots/:id/edit")       view=|| view! { <BotEditorPage/> }/>
                    <Route path=path!("/admin/users")         view=|| view! { <AdminUsersPage/> }/>
                    <Route path=path!("/admin/analytics")     view=|| view! { <AdminAnalyticsPage/> }/>
                </Routes>
            </AppShell>
        </Router>

        <SettingsModal/>
        <Toasts/>
    }
}
