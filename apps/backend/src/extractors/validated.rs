//! Extractors that reject with our JSON error body (axum's defaults reply in
//! plain text) and run `validator` rules where the type has them.

use axum::{
    extract::{
        FromRequest, FromRequestParts, Request,
        rejection::{JsonRejection, PathRejection, QueryRejection},
    },
    http::request::Parts,
};
use serde::de::DeserializeOwned;
use validator::Validate;

use crate::error::AppError;

impl From<JsonRejection> for AppError {
    fn from(r: JsonRejection) -> Self {
        Self::BadRequest(r.body_text())
    }
}

impl From<QueryRejection> for AppError {
    fn from(r: QueryRejection) -> Self {
        Self::BadRequest(r.body_text())
    }
}

impl From<PathRejection> for AppError {
    fn from(r: PathRejection) -> Self {
        Self::BadRequest(r.body_text())
    }
}

/// JSON body, deserialised then validated.
pub struct ValidJson<T>(pub T);

impl<S, T> FromRequest<S> for ValidJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Validate,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, AppError> {
        let axum::Json(value) = axum::Json::<T>::from_request(req, state).await?;
        value.validate()?;
        Ok(Self(value))
    }
}

/// Query string, deserialised then validated.
pub struct ValidQuery<T>(pub T);

impl<S, T> FromRequestParts<S> for ValidQuery<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Validate,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, AppError> {
        let axum::extract::Query(value) =
            axum::extract::Query::<T>::from_request_parts(parts, state).await?;
        value.validate()?;
        Ok(Self(value))
    }
}

/// Path parameters with JSON error rejections.
pub struct PathParam<T>(pub T);

impl<S, T> FromRequestParts<S> for PathParam<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Send,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, AppError> {
        let axum::extract::Path(value) =
            axum::extract::Path::<T>::from_request_parts(parts, state).await?;
        Ok(Self(value))
    }
}
