use axum::{extract::FromRequestParts, http::request::Parts};
use serde::Deserialize;
use validator::Validate;

use super::validated::ValidQuery;
use crate::error::AppError;

const DEFAULT_LIMIT: i64 = 20;

/// `?limit=&offset=` with bounds. Parsed separately from a handler's other
/// query params, which can live in their own `ValidQuery<...>`.
#[derive(Debug, Clone, Copy, Deserialize, Validate)]
pub struct Pagination {
    #[serde(default = "default_limit")]
    #[validate(range(min = 1, max = 100, message = "must be between 1 and 100"))]
    pub limit: i64,
    #[serde(default)]
    #[validate(range(min = 0, message = "must not be negative"))]
    pub offset: i64,
}

fn default_limit() -> i64 {
    DEFAULT_LIMIT
}

impl<S: Send + Sync> FromRequestParts<S> for Pagination {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, AppError> {
        let ValidQuery(p) = ValidQuery::<Self>::from_request_parts(parts, state).await?;
        Ok(p)
    }
}
