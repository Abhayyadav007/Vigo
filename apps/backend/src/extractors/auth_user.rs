use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};
use uuid::Uuid;

use crate::{
    auth::{FirebaseClaims, Role},
    error::AppError,
    services::auth_service,
    state::AppState,
};

/// A caller holding a valid Firebase ID token, who may not have a Postgres
/// user yet. Only `/v1/auth/sync` should need this; everything else uses
/// [`AuthUser`].
#[derive(Debug, Clone)]
pub struct FirebaseIdentity(pub FirebaseClaims);

impl FromRequestParts<AppState> for FirebaseIdentity {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        let token = bearer_token(parts).ok_or(AppError::Unauthorized)?;
        match state.verifier.verify(token).await {
            Ok(claims) => Ok(Self(claims)),
            Err(e) if e.is_upstream() => {
                tracing::error!(error = %e, "cannot verify Firebase tokens");
                Err(AppError::ServiceUnavailable(
                    "authentication is temporarily unavailable".into(),
                ))
            }
            Err(e) => {
                tracing::debug!(error = %e, "rejected Firebase token");
                Err(AppError::Unauthorized)
            }
        }
    }
}

/// A verified caller with a Postgres account. The role comes from Postgres
/// (via the session cache), never from the token.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub firebase_uid: String,
    pub role: Role,
    pub store_id: Option<Uuid>,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        if let Some(user) = parts.extensions.get::<Self>() {
            return Ok(user.clone());
        }
        let FirebaseIdentity(claims) = FirebaseIdentity::from_request_parts(parts, state).await?;
        let session = auth_service::resolve_session(state, &claims.sub).await?;
        let user = Self {
            user_id: session.user_id,
            firebase_uid: claims.sub,
            role: session.role,
            store_id: session.store_id,
        };
        // Handlers that take several auth extractors verify the token once.
        parts.extensions.insert(user.clone());
        Ok(user)
    }
}

fn bearer_token(parts: &Parts) -> Option<&str> {
    let value = parts.headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    let token = token.trim();
    (scheme.eq_ignore_ascii_case("bearer") && !token.is_empty()).then_some(token)
}
