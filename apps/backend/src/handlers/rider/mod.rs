use axum::{Json, extract::State, http::StatusCode};
use uuid::Uuid;

use crate::{
    dto::{
        page::Page,
        rider::{
            ActiveDelivery, DeliverRequest, DeliveryHistoryItem, DeliveryOffer, LocationBatch,
            PickupRequest, RiderMe, RiderProfileRequest, RiderStatusRequest,
        },
    },
    error::AppResult,
    extractors::{Pagination, PathParam, Rider, ValidJson},
    services::{dispatch_service, rider_service},
    state::AppState,
};

/// `GET /v1/rider/me`
pub async fn me(State(state): State<AppState>, rider: Rider) -> AppResult<Json<RiderMe>> {
    Ok(Json(rider_service::me(&state, &rider).await?))
}

/// `PUT /v1/rider/profile`
pub async fn update_profile(
    State(state): State<AppState>,
    rider: Rider,
    ValidJson(body): ValidJson<RiderProfileRequest>,
) -> AppResult<Json<RiderMe>> {
    Ok(Json(
        rider_service::update_profile(&state, &rider, &body).await?,
    ))
}

/// `PUT /v1/rider/status` (go online / offline)
pub async fn set_status(
    State(state): State<AppState>,
    rider: Rider,
    ValidJson(body): ValidJson<RiderStatusRequest>,
) -> AppResult<Json<RiderMe>> {
    Ok(Json(
        rider_service::set_online(&state, &rider, body.online).await?,
    ))
}

/// `POST /v1/rider/location` (batched background GPS)
pub async fn location(
    State(state): State<AppState>,
    rider: Rider,
    ValidJson(body): ValidJson<LocationBatch>,
) -> AppResult<StatusCode> {
    rider_service::record_location(&state, &rider, &body).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /v1/rider/offers`
pub async fn offers(
    State(state): State<AppState>,
    rider: Rider,
) -> AppResult<Json<Vec<DeliveryOffer>>> {
    Ok(Json(
        dispatch_service::pending_offers(&state, &rider).await?,
    ))
}

/// `POST /v1/rider/offers/{orderId}/accept`
pub async fn accept(
    State(state): State<AppState>,
    rider: Rider,
    PathParam(order_id): PathParam<Uuid>,
) -> AppResult<Json<ActiveDelivery>> {
    Ok(Json(
        dispatch_service::accept(&state, &rider, order_id).await?,
    ))
}

/// `POST /v1/rider/offers/{orderId}/decline`
pub async fn decline(
    State(state): State<AppState>,
    rider: Rider,
    PathParam(order_id): PathParam<Uuid>,
) -> AppResult<StatusCode> {
    dispatch_service::decline(&state, &rider, order_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /v1/rider/delivery` (the current one; 204 if none)
pub async fn active(
    State(state): State<AppState>,
    rider: Rider,
) -> AppResult<Result<Json<ActiveDelivery>, StatusCode>> {
    Ok(rider_service::active(&state, &rider)
        .await?
        .map(Json)
        .ok_or(StatusCode::NO_CONTENT))
}

/// `POST /v1/rider/deliveries/{orderId}/pickup`
pub async fn pickup(
    State(state): State<AppState>,
    rider: Rider,
    PathParam(order_id): PathParam<Uuid>,
    ValidJson(body): ValidJson<PickupRequest>,
) -> AppResult<Json<ActiveDelivery>> {
    Ok(Json(
        rider_service::pickup(&state, &rider, order_id, body.bag_count).await?,
    ))
}

/// `POST /v1/rider/deliveries/{orderId}/depart`
pub async fn depart(
    State(state): State<AppState>,
    rider: Rider,
    PathParam(order_id): PathParam<Uuid>,
) -> AppResult<Json<ActiveDelivery>> {
    Ok(Json(rider_service::depart(&state, &rider, order_id).await?))
}

/// `POST /v1/rider/deliveries/{orderId}/deliver`
pub async fn deliver(
    State(state): State<AppState>,
    rider: Rider,
    PathParam(order_id): PathParam<Uuid>,
    ValidJson(body): ValidJson<DeliverRequest>,
) -> AppResult<Json<DeliveryHistoryItem>> {
    Ok(Json(
        rider_service::deliver(&state, &rider, order_id, &body).await?,
    ))
}

/// `POST /v1/rider/deliveries/{orderId}/unassign` (before pickup)
pub async fn unassign(
    State(state): State<AppState>,
    rider: Rider,
    PathParam(order_id): PathParam<Uuid>,
) -> AppResult<StatusCode> {
    rider_service::unassign(&state, &rider, order_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /v1/rider/deliveries`
pub async fn history(
    State(state): State<AppState>,
    rider: Rider,
    page: Pagination,
) -> AppResult<Json<Page<DeliveryHistoryItem>>> {
    Ok(Json(
        rider_service::history(&state, &rider, page.limit, page.offset).await?,
    ))
}
