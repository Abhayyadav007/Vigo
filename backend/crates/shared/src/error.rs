use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("database error")]
    Db(#[from] sqlx::Error),

    #[error("not found")]
    NotFound,

    #[error("{0}")]
    BadRequest(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // Log the real cause; return something generic. Never leak SQL errors
        // or connection strings to a client.
        let (status, message) = match &self {
            AppError::Db(err) => {
                tracing::error!(error = ?err, "database error");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
            }
            AppError::NotFound => (StatusCode::NOT_FOUND, "not found"),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.as_str()),
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}
