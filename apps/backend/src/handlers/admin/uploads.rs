use axum::{
    Json,
    extract::{Multipart, State, multipart::MultipartError},
    http::StatusCode,
};

use crate::{
    dto::catalog::UploadResponse,
    error::{AppError, AppResult},
    extractors::Admin,
    services::media_service,
    state::AppState,
};

/// `POST /v1/admin/uploads` (multipart, one `file` field). Returns the URL to
/// put in `imageUrls` / `imageUrl`.
pub async fn upload_image(
    State(state): State<AppState>,
    _admin: Admin,
    mut multipart: Multipart,
) -> AppResult<(StatusCode, Json<UploadResponse>)> {
    while let Some(field) = multipart.next_field().await.map_err(multipart_error)? {
        if field.name() != Some("file") {
            continue;
        }
        let bytes = field.bytes().await.map_err(multipart_error)?;
        let url = media_service::save_image(&state, &bytes).await?;
        return Ok((StatusCode::CREATED, Json(UploadResponse { url })));
    }
    Err(AppError::BadRequest(
        "expected a multipart field named `file`".into(),
    ))
}

fn multipart_error(e: MultipartError) -> AppError {
    if e.status() == StatusCode::PAYLOAD_TOO_LARGE {
        AppError::Validation("image must be 5 MB or smaller".into())
    } else {
        AppError::BadRequest(e.body_text())
    }
}
