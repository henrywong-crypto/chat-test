/// POST /api/upload — multipart file upload to S3.
///
/// Accepts a multipart body with a single field named `"file"`.
/// Stores the bytes in S3 and returns the S3 key, content type, and filename.

use axum::{extract::State, http::StatusCode, Json};
use uuid::Uuid;

use auth::extractor::CurrentUser;
use db::S3Store;
use shared::api::UploadResponse;

use crate::server::state::AppState;

pub async fn upload_file(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<UploadResponse>, (StatusCode, String)> {
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        (StatusCode::BAD_REQUEST, format!("multipart error: {e}"))
    })? {
        if field.name() != Some("file") {
            continue;
        }

        let filename = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "upload".to_string());

        let content_type = field
            .content_type()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "application/octet-stream".to_string());

        let ext = ext_for_content_type(&content_type);

        let data = field.bytes().await.map_err(|e| {
            (StatusCode::BAD_REQUEST, format!("read field: {e}"))
        })?;

        let uuid = Uuid::new_v4().to_string();
        let key  = S3Store::upload_key(&user.id, &uuid, ext);

        state.s3.put_upload(&key, data.to_vec(), &content_type).await.map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("S3 upload: {e}"))
        })?;

        return Ok(Json(UploadResponse { key, content_type, name: filename }));
    }

    Err((StatusCode::BAD_REQUEST, "no 'file' field in multipart body".to_string()))
}

fn ext_for_content_type(ct: &str) -> &'static str {
    match ct {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/png"                => "png",
        "image/gif"                => "gif",
        "image/webp"               => "webp",
        _                          => "bin",
    }
}
