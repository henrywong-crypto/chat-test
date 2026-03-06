/// Axum router — API routes + Leptos SSR fallback.

use axum::{
    routing::{delete, get, patch, post, put},
    Router,
};
use leptos::prelude::*;
use leptos_axum::{generate_route_list, LeptosRoutes};
use tower_http::{compression::CompressionLayer, trace::TraceLayer};

use crate::app::App;
use super::{
    handlers::{admin, bots, chat, conversations, profiles},
    state::AppState,
};

pub async fn build_router(state: AppState) -> Router {
    let conf           = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    let routes         = generate_route_list(App);

    // ── API routes ────────────────────────────────────────────────────────────
    let api = Router::new()
        // Chat
        .route("/chat", post(chat::chat_stream))
        // Conversations
        .route("/conversations",              get(conversations::list_conversations))
        .route("/conversations/:id",          get(conversations::get_conversation))
        .route("/conversations/:id",          delete(conversations::delete_conversation))
        .route("/conversations/:id/title",    patch(conversations::update_title))
        // Bots
        .route("/bots",          get(bots::list_my_bots))
        .route("/bots",          post(bots::create_bot))
        .route("/bots/store",    get(bots::list_public_bots))
        .route("/bots/:id",      get(bots::get_bot))
        .route("/bots/:id",      put(bots::update_bot))
        .route("/bots/:id",      delete(bots::delete_bot))
        // Inference profiles
        .route("/inference-profiles",           get(profiles::list_profiles))
        .route("/inference-profiles",           post(profiles::create_profile))
        .route("/inference-profiles/:model_id", delete(profiles::delete_profile))
        // Models list (unauthenticated)
        .route("/models", get(list_models))
        // Admin
        .route("/admin/users",              get(admin::list_users))
        .route("/admin/users/:id/groups",   patch(admin::update_user_groups))
        .route("/admin/analytics",          get(admin::analytics))
        .with_state(state.clone());

    // ── Full router ───────────────────────────────────────────────────────────
    Router::new()
        .nest("/api", api)
        .leptos_routes(&leptos_options, routes, {
            let opts = leptos_options.clone();
            move || shell(opts.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
}

// ── Simple models list ────────────────────────────────────────────────────────

async fn list_models() -> axum::Json<serde_json::Value> {
    use bedrock::models::list_models;
    let models: Vec<_> = list_models()
        .iter()
        .map(|m| serde_json::json!({
            "id":           m.id,
            "display_name": m.display_name,
            "provider":     m.provider.display_name(),
            "vision":       m.capabilities.vision,
            "tool_use":     m.capabilities.tool_use,
            "reasoning":    m.capabilities.reasoning,
        }))
        .collect();
    axum::Json(serde_json::json!({ "models": models }))
}

// ── Leptos shell ──────────────────────────────────────────────────────────────

fn shell(options: LeptosOptions) -> impl IntoView {
    let dev_bypass = if std::env::var("DEV_AUTH_BYPASS").ok().as_deref() == Some("true") {
        "true"
    } else {
        "false"
    };

    view! {
        <!DOCTYPE html>
        <html lang="en" data-theme="light">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <meta name="dev-auth-bypass" content=dev_bypass/>
                <title>"Bedrock RS"</title>
                <link id="leptos" rel="stylesheet" href="/pkg/app.css"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options=options.clone()/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}
