/// Client-side API helpers.
///
/// All functions are guarded with `#[cfg(feature = "hydrate")]` because they
/// rely on `web_sys` / `wasm_bindgen` which are only available in the browser.
/// The module itself is compiled in both SSR and hydrate builds so that
/// `use crate::api` can appear unconditionally at the top of other modules.

// ── Models ────────────────────────────────────────────────────────────────────

#[cfg(feature = "hydrate")]
pub async fn fetch_models() -> Result<Vec<shared::ModelInfo>, String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let window = web_sys::window().ok_or("no window")?;
    let resp_val = JsFuture::from(window.fetch_with_str("/api/models"))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let response: web_sys::Response = resp_val.dyn_into().map_err(|e| format!("{e:?}"))?;
    let text = get_text(&response).await?;
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    serde_json::from_value(v["models"].clone()).map_err(|e| e.to_string())
}

// ── Conversations ─────────────────────────────────────────────────────────────

#[cfg(feature = "hydrate")]
pub async fn fetch_conversations(token: &str) -> Result<Vec<shared::ConversationMeta>, String> {
    let response = authed_get("/api/conversations", token).await?;
    let text = get_text(&response).await?;
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    serde_json::from_value(v["conversations"].clone()).map_err(|e| e.to_string())
}

#[cfg(feature = "hydrate")]
pub async fn fetch_conversation(
    id: &str,
    token: &str,
) -> Result<shared::Conversation, String> {
    let url = format!("/api/conversations/{id}");
    let response = authed_get(&url, token).await?;
    let text = get_text(&response).await?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

// ── Chat streaming ────────────────────────────────────────────────────────────

/// Stream chat events from `POST /api/chat` via SSE over a `fetch` ReadableStream.
#[cfg(feature = "hydrate")]
pub async fn stream_chat(
    request:  shared::SendMessageRequest,
    token:    String,
    on_event: impl Fn(shared::StreamEvent) + 'static,
) -> Result<(), String> {
    use js_sys::{Reflect, Uint8Array};
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Headers, RequestInit, Response};

    let window = web_sys::window().ok_or("no window")?;

    let headers = Headers::new().map_err(|e| format!("{e:?}"))?;
    headers
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{e:?}"))?;
    headers
        .set("Authorization", &format!("Bearer {token}"))
        .map_err(|e| format!("{e:?}"))?;

    let body = serde_json::to_string(&request).map_err(|e| e.to_string())?;

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_headers(&headers);
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body));

    let req_obj = web_sys::Request::new_with_str_and_init("/api/chat", &opts)
        .map_err(|e| format!("{e:?}"))?;

    let resp_val = JsFuture::from(window.fetch_with_request(&req_obj))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let response: Response = resp_val.dyn_into().map_err(|e| format!("{e:?}"))?;

    if !response.ok() {
        return Err(format!("HTTP {}", response.status()));
    }

    let body_stream = response.body().ok_or("no response body")?;
    let reader: web_sys::ReadableStreamDefaultReader = body_stream
        .get_reader()
        .dyn_into()
        .map_err(|e| format!("{e:?}"))?;

    let mut buf = String::new();

    loop {
        let chunk = JsFuture::from(reader.read())
            .await
            .map_err(|e| format!("{e:?}"))?;

        let done = Reflect::get(&chunk, &wasm_bindgen::JsValue::from_str("done"))
            .map_err(|e| format!("{e:?}"))?
            .as_bool()
            .unwrap_or(false);
        if done { break; }

        let value = Reflect::get(&chunk, &wasm_bindgen::JsValue::from_str("value"))
            .map_err(|e| format!("{e:?}"))?;
        let arr: Uint8Array = value.dyn_into().map_err(|e| format!("{e:?}"))?;
        buf.push_str(&String::from_utf8_lossy(&arr.to_vec()));

        loop {
            match buf.find('\n') {
                None => break,
                Some(nl) => {
                    let line = buf[..nl].trim_end_matches('\r').to_string();
                    buf = buf[nl + 1..].to_string();
                    if let Some(data) = line.strip_prefix("data: ") {
                        if let Ok(ev) =
                            serde_json::from_str::<shared::StreamEvent>(data)
                        {
                            on_event(ev);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

// ── Upload ────────────────────────────────────────────────────────────────────

/// Upload a file to S3 via `POST /api/upload` (multipart).
///
/// The caller builds a `FormData` with a field named `"file"` and passes the
/// auth token.  Returns the S3 key, content type, and original filename.
#[cfg(feature = "hydrate")]
pub async fn upload_file(
    form_data: web_sys::FormData,
    token: &str,
) -> Result<shared::api::UploadResponse, String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Headers, RequestInit};

    let window = web_sys::window().ok_or("no window")?;

    let headers = Headers::new().map_err(|e| format!("{e:?}"))?;
    headers
        .set("Authorization", &format!("Bearer {token}"))
        .map_err(|e| format!("{e:?}"))?;
    // Do NOT set Content-Type here — the browser must set it with the boundary.

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_headers(&headers);
    opts.set_body(&form_data);

    let req = web_sys::Request::new_with_str_and_init("/api/upload", &opts)
        .map_err(|e| format!("{e:?}"))?;

    let resp_val = JsFuture::from(window.fetch_with_request(&req))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let response: web_sys::Response = resp_val.dyn_into().map_err(|e| format!("{e:?}"))?;

    if !response.ok() {
        return Err(format!("HTTP {}", response.status()));
    }

    let text = get_text(&response).await?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

// ── Private helpers ───────────────────────────────────────────────────────────

#[cfg(feature = "hydrate")]
async fn authed_get(url: &str, token: &str) -> Result<web_sys::Response, String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Headers, RequestInit};

    let window = web_sys::window().ok_or("no window")?;

    let headers = Headers::new().map_err(|e| format!("{e:?}"))?;
    headers
        .set("Authorization", &format!("Bearer {token}"))
        .map_err(|e| format!("{e:?}"))?;

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_headers(&headers);

    let req = web_sys::Request::new_with_str_and_init(url, &opts)
        .map_err(|e| format!("{e:?}"))?;
    let resp_val = JsFuture::from(window.fetch_with_request(&req))
        .await
        .map_err(|e| format!("{e:?}"))?;
    resp_val
        .dyn_into::<web_sys::Response>()
        .map_err(|e| format!("{e:?}"))
}

#[cfg(feature = "hydrate")]
async fn get_text(response: &web_sys::Response) -> Result<String, String> {
    use wasm_bindgen_futures::JsFuture;
    let promise = response.text().map_err(|e| format!("{e:?}"))?;
    let val = JsFuture::from(promise).await.map_err(|e| format!("{e:?}"))?;
    val.as_string().ok_or_else(|| "response is not a string".into())
}
