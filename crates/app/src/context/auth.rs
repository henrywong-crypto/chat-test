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
    let user: RwSignal<Option<AuthUser>> = RwSignal::new(None);

    // On the client, read the Cognito token from localStorage and decode the
    // JWT claims without verifying the signature (verification happens
    // server-side; this is just for display / routing purposes).
    #[cfg(feature = "hydrate")]
    {
        Effect::new(move |_| {
            if let Some(auth_user) = load_from_storage() {
                user.set(Some(auth_user));
            }
        });
    }

    provide_context(user);
}

/// Returns the auth signal.  Must be called inside a descendant of the
/// component that called `provide_auth_context()`.
pub fn use_auth() -> RwSignal<Option<AuthUser>> {
    expect_context()
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
