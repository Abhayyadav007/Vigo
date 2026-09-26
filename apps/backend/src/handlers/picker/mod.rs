use axum::{Json, extract::State};
use uuid::Uuid;

use crate::{
    dto::picker::{
        PackRequest, PickList, PickerCancelRequest, PickerQueueItem, ScanRequest, ScanResult,
        SetPickedRequest,
    },
    error::AppResult,
    extractors::{PathParam, Picker, ValidJson},
    services::picker_service,
    state::AppState,
};

/// `GET /v1/picker/orders`: the store's queue (CONFIRMED, PICKING, PACKED).
pub async fn queue(
    State(state): State<AppState>,
    picker: Picker,
) -> AppResult<Json<Vec<PickerQueueItem>>> {
    Ok(Json(picker_service::queue(&state, &picker).await?))
}

/// `GET /v1/picker/orders/{id}`: pick list sorted by bin.
pub async fn pick_list(
    State(state): State<AppState>,
    picker: Picker,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<PickList>> {
    Ok(Json(picker_service::pick_list(&state, &picker, id).await?))
}

/// `POST /v1/picker/orders/{id}/start`
pub async fn start(
    State(state): State<AppState>,
    picker: Picker,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<PickList>> {
    Ok(Json(picker_service::start(&state, &picker, id).await?))
}

/// `POST /v1/picker/orders/{id}/scan`
pub async fn scan(
    State(state): State<AppState>,
    picker: Picker,
    PathParam(id): PathParam<Uuid>,
    ValidJson(body): ValidJson<ScanRequest>,
) -> AppResult<Json<ScanResult>> {
    Ok(Json(
        picker_service::scan(&state, &picker, id, &body.barcode).await?,
    ))
}

/// `PUT /v1/picker/orders/{id}/items/{productId}`: manual count / mark missing.
pub async fn set_picked(
    State(state): State<AppState>,
    picker: Picker,
    PathParam((id, product_id)): PathParam<(Uuid, Uuid)>,
    ValidJson(body): ValidJson<SetPickedRequest>,
) -> AppResult<Json<PickList>> {
    Ok(Json(
        picker_service::set_picked(&state, &picker, id, product_id, body.picked_quantity).await?,
    ))
}

/// `POST /v1/picker/orders/{id}/pack`
pub async fn pack(
    State(state): State<AppState>,
    picker: Picker,
    PathParam(id): PathParam<Uuid>,
    ValidJson(body): ValidJson<PackRequest>,
) -> AppResult<Json<PickList>> {
    Ok(Json(
        picker_service::pack(&state, &picker, id, &body).await?,
    ))
}

/// `POST /v1/picker/orders/{id}/release`
pub async fn release(
    State(state): State<AppState>,
    picker: Picker,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<PickList>> {
    Ok(Json(picker_service::release(&state, &picker, id).await?))
}

/// `POST /v1/picker/orders/{id}/cancel`
pub async fn cancel(
    State(state): State<AppState>,
    picker: Picker,
    PathParam(id): PathParam<Uuid>,
    ValidJson(body): ValidJson<PickerCancelRequest>,
) -> AppResult<Json<PickList>> {
    Ok(Json(
        picker_service::cancel(&state, &picker, id, &body.reason).await?,
    ))
}
