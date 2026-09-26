use std::time::Duration;

use axum::{Json, extract::State};

use crate::{
    cache::rate_limit,
    dto::auth::MeResponse,
    error::{AppError, AppResult},
    extractors::{AuthUser, FirebaseIdentity},
    repositories::users,
    services::auth_service,
    state::AppState,
};

/// `POST /v1/auth/sync`: called by every client right after Firebase sign-in.
pub async fn sync(
    State(state): State<AppState>,
    FirebaseIdentity(claims): FirebaseIdentity,
) -> AppResult<Json<MeResponse>> {
    rate_limit::check(
        &state.redis,
        &format!("sync:{}", claims.sub),
        10,
        Duration::from_secs(60),
    )
    .await?;
    let user = auth_service::sync(&state, &claims).await?;
    Ok(Json(user.into()))
}

/// `GET /v1/auth/me`
pub async fn me(State(state): State<AppState>, auth: AuthUser) -> AppResult<Json<MeResponse>> {
    let user = users::find_by_id(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::NotFound("user"))?;
    Ok(Json(user.into()))
}
