use axum::{Json, extract::State, http::StatusCode};
use uuid::Uuid;

use crate::{
    dto::order::{AddressDto, AddressRequest},
    error::AppResult,
    extractors::{Customer, PathParam, ValidJson},
    services::address_service,
    state::AppState,
};

/// `GET /v1/customer/addresses`
pub async fn list(
    State(state): State<AppState>,
    user: Customer,
) -> AppResult<Json<Vec<AddressDto>>> {
    Ok(Json(address_service::list(&state, user.user_id).await?))
}

/// `POST /v1/customer/addresses`
pub async fn create(
    State(state): State<AppState>,
    user: Customer,
    ValidJson(body): ValidJson<AddressRequest>,
) -> AppResult<(StatusCode, Json<AddressDto>)> {
    let a = address_service::create(&state, user.user_id, &body).await?;
    Ok((StatusCode::CREATED, Json(a)))
}

/// `PUT /v1/customer/addresses/{id}`
pub async fn update(
    State(state): State<AppState>,
    user: Customer,
    PathParam(id): PathParam<Uuid>,
    ValidJson(body): ValidJson<AddressRequest>,
) -> AppResult<Json<AddressDto>> {
    Ok(Json(
        address_service::update(&state, user.user_id, id, &body).await?,
    ))
}

/// `DELETE /v1/customer/addresses/{id}`
pub async fn delete(
    State(state): State<AppState>,
    user: Customer,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<StatusCode> {
    address_service::delete(&state, user.user_id, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
