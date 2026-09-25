use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::dto::error::{ErrorBody, ErrorDetail};

/// Every fallible handler returns `Result<_, AppError>`. The response body is
/// always `{ "error": { "code": "...", "message": "..." } }`.
#[derive(Debug, thiserror::Error)]
#[expect(
    dead_code,
    reason = "variants are constructed by handlers from phase 2 on"
)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Validation(String),
    #[error("authentication required")]
    Unauthorized,
    #[error("insufficient permissions")]
    Forbidden,
    #[error("{0} not found")]
    NotFound(&'static str),
    #[error("{0}")]
    Conflict(String),
    #[error("too many requests")]
    RateLimited,
    #[error("{0}")]
    ServiceUnavailable(String),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Redis(#[from] deadpool_redis::redis::RedisError),
    #[error(transparent)]
    RedisPool(#[from] deadpool_redis::PoolError),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl AppError {
    fn status(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            Self::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::Database(sqlx::Error::RowNotFound) => StatusCode::NOT_FOUND,
            Self::Database(_) | Self::Redis(_) | Self::RedisPool(_) | Self::Internal(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::BadRequest(_) => "BAD_REQUEST",
            Self::Validation(_) => "VALIDATION_FAILED",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::Forbidden => "FORBIDDEN",
            Self::NotFound(_) | Self::Database(sqlx::Error::RowNotFound) => "NOT_FOUND",
            Self::Conflict(_) => "CONFLICT",
            Self::RateLimited => "RATE_LIMITED",
            Self::ServiceUnavailable(_) => "SERVICE_UNAVAILABLE",
            Self::Database(_) | Self::Redis(_) | Self::RedisPool(_) | Self::Internal(_) => {
                "INTERNAL"
            }
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        // Never leak driver/internal error details to clients.
        let message = if status.is_server_error() && status != StatusCode::SERVICE_UNAVAILABLE {
            tracing::error!(error = ?self, "request failed");
            "internal server error".to_owned()
        } else {
            self.to_string()
        };

        let body = ErrorBody {
            error: ErrorDetail {
                code: self.code().to_owned(),
                message,
            },
        };
        (status, Json(body)).into_response()
    }
}

#[expect(dead_code, reason = "used by handlers from phase 2 on")]
pub type AppResult<T> = Result<T, AppError>;
