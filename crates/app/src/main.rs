/// Server entry point — SSR only (compiled with feature = "ssr").
///
/// cargo-leptos compiles this binary and also compiles the lib with
/// feature = "hydrate" to produce the WASM bundle.  The server then
/// serves both the HTML (SSR) and the WASM bundle.

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use app::server::state::AppState;
    use app::server::router::build_router;
    use tracing_subscriber::{EnvFilter, fmt};

    fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let state  = AppState::from_env().await.expect("failed to build AppState");
    let router = build_router(state).await;

    let addr = std::env::var("SITE_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".into());
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("listening on http://{}", addr);
    axum::serve(listener, router).await.unwrap();
}

// Satisfy the compiler when building the lib target without ssr.
#[cfg(not(feature = "ssr"))]
fn main() {}
