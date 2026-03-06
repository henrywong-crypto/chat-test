/// Client-side auth context — holds the current logged-in user signal.
///
/// The signal is initially `None` (no user on SSR / before hydration).
/// After hydration the app reads the Cognito JWT from `localStorage` and
/// populates this signal.

use leptos::prelude::*;

// ── Domain type ───────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
pub struct AuthUser {
    pub id:       String,
    pub email:    String,
    pub is_admin: bool,
    /// Raw JWT — sent as `Authorization: Bearer <token>` on API calls.
    pub token:    String,
}

// ── Context helpers ───────────────────────────────────────────────────────────

/// Call once at the root `App` component to make the auth signal available
/// to all descendants via `use_auth()`.
pub fn provide_auth_context() {
    // When DEV_AUTH_BYPASS is set, pre-populate the signal so SSR HTML matches
    // what the client will render after hydration (avoids hydration mismatch).
    let initial = dev_auth_initial();

    let user: RwSignal<Option<AuthUser>> = RwSignal::new(initial);

    // On the client, read the Cognito token from localStorage and decode the
    // JWT claims without verifying the signature (verification happens
    // server-side; this is just for display / routing purposes).
    #[cfg(feature = "hydrate")]
    {
        Effect::new(move |_| {
            // Check for dev auth bypass meta tag injected by the server.
            if let Some(dev_user) = dev_auth_user() {
                user.set(Some(dev_user));
                return;
            }
            if let Some(auth_user) = load_from_storage() {
                user.set(Some(auth_user));
            }
        });
    }

    provide_context(user);
}

fn dev_auth_initial() -> Option<AuthUser> {
    #[cfg(feature = "ssr")]
    {
        if std::env::var("DEV_AUTH_BYPASS").ok().as_deref() == Some("true") {
            return Some(dev_auth_user_value());
        }
    }
    #[cfg(feature = "hydrate")]
    {
        // Read the meta tag synchronously so the initial signal matches SSR.
        if let Some(_) = dev_auth_user() {
            return Some(dev_auth_user_value());
        }
    }
    None
}

fn dev_auth_user_value() -> AuthUser {
    AuthUser {
        id:       "dev-user-0000".into(),
        email:    "dev@localhost".into(),
        is_admin: true,
        token:    "dev-bypass-token".into(),
    }
}

/// Returns the auth signal.  Must be called inside a descendant of the
/// component that called `provide_auth_context()`.
pub fn use_auth() -> RwSignal<Option<AuthUser>> {
    expect_context()
}

// ── Dev auth bypass ──────────────────────────────────────────────────────────

/// Check for `<meta name="dev-auth-bypass" content="true">` injected by the
/// server when `DEV_AUTH_BYPASS=true`.  Returns a fake dev user so the UI is
/// usable without Cognito.
#[cfg(feature = "hydrate")]
fn dev_auth_user() -> Option<AuthUser> {
    let doc = web_sys::window()?.document()?;
    let meta = doc.query_selector(r#"meta[name="dev-auth-bypass"]"#).ok()??;
    let content = meta.get_attribute("content")?;
    if content != "true" {
        return None;
    }
    Some(dev_auth_user_value())
}

// ── Client-side JWT decode ────────────────────────────────────────────────────

#[cfg(feature = "hydrate")]
fn load_from_storage() -> Option<AuthUser> {
    let window   = web_sys::window()?;
    let storage  = window.local_storage().ok()??;

    // Cognito stores the token under various keys; try common ones.
    let token = ["id_token", "access_token", "CognitoIdentityServiceProvider"]
        .iter()
        .find_map(|prefix| {
            // Full match first, then scan all keys for this prefix.
            storage.get_item(prefix).ok().flatten().or_else(|| {
                let len = storage.length().ok()?;
                (0..len).find_map(|i| {
                    let key = storage.key(i).ok()??;
                    if key.contains(prefix) && key.ends_with(".idToken") {
                        storage.get_item(&key).ok().flatten()
                    } else {
                        None
                    }
                })
            })
        })?;

    decode_jwt_claims(&token)
}

#[cfg(feature = "hydrate")]
fn decode_jwt_claims(token: &str) -> Option<AuthUser> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

    let payload_b64 = token.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload_b64).ok()?;
    let claims: serde_json::Value = serde_json::from_slice(&bytes).ok()?;

    let sub   = claims["sub"].as_str()?.to_string();
    let email = claims["email"].as_str().unwrap_or(&sub).to_string();
    let groups: Vec<&str> = claims["cognito:groups"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();
    let is_admin = groups.contains(&"Administrators");

    Some(AuthUser { id: sub, email, is_admin, token: token.to_string() })
}
