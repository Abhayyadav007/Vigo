use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::dto::error::{ErrorBody, ErrorDetail};

/// Every fallible handler returns `Result<_, AppError>`. The response body is
/// always `{ "error": { "code": "...", "message": "..." } }`.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Validation(String),
    /// Missing, malformed, expired or otherwise unverifiable credentials.
    #[error("authentication required")]
    Unauthorized,
    /// Authenticated but not allowed. `code` lets clients tell cases apart
    /// (e.g. `USER_NOT_REGISTERED`, `ACCOUNT_DISABLED`, `FORBIDDEN`).
    #[error("{message}")]
    Forbidden {
        code: &'static str,
        message: &'static str,
    },
    #[error("{0} not found")]
    NotFound(&'static str),
    /// A client error with a specific machine-readable code, e.g.
    /// 422 `SCAN_MISMATCH`, so apps can react to the exact case.
    #[error("{message}")]
    Coded {
        status: StatusCode,
        code: &'static str,
        message: String,
    },
    #[error("{0}")]
    Conflict(String),
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
    pub const fn forbidden() -> Self {
        Self::Forbidden {
            code: "FORBIDDEN",
            message: "insufficient permissions",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden { .. } => StatusCode::FORBIDDEN,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Coded { status, .. } => *status,
            Self::Conflict(_) => StatusCode::CONFLICT,
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
            Self::Forbidden { code, .. } | Self::Coded { code, .. } => code,
            Self::NotFound(_) | Self::Database(sqlx::Error::RowNotFound) => "NOT_FOUND",
            Self::Conflict(_) => "CONFLICT",
            Self::ServiceUnavailable(_) => "SERVICE_UNAVAILABLE",
            Self::Database(_) | Self::Redis(_) | Self::RedisPool(_) | Self::Internal(_) => {
                "INTERNAL"
            }
        }
    }
}

impl From<validator::ValidationErrors> for AppError {
    fn from(errors: validator::ValidationErrors) -> Self {
        let fields: Vec<String> = errors
            .field_errors()
            .into_iter()
            .map(|(field, errs)| {
                let reason = errs
                    .first()
                    .and_then(|e| e.message.as_deref().map(str::to_owned))
                    .unwrap_or_else(|| "is invalid".to_owned());
                format!("{field} {reason}")
            })
            .collect();
        Self::Validation(fields.join("; "))
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

pub type AppResult<T> = Result<T, AppError>;

/// How to report a violated Postgres constraint.
#[derive(Debug, Clone, Copy)]
pub enum On {
    /// 409: duplicate of something that exists (unique constraints).
    Conflict,
    /// 422: the request refers to something invalid (FKs, checks).
    Invalid,
}

/// Maps known constraint violations to client errors; anything else stays a 500.
pub fn map_constraint(e: sqlx::Error, known: &[(&str, On, &str)]) -> AppError {
    if let sqlx::Error::Database(db) = &e
        && let Some(name) = db.constraint()
        && let Some((_, on, message)) = known.iter().find(|(c, ..)| *c == name)
    {
        return match on {
            On::Conflict => AppError::Conflict((*message).to_owned()),
            On::Invalid => AppError::Validation((*message).to_owned()),
        };
    }
    e.into()
}
