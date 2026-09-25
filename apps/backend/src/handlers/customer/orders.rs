use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use uuid::Uuid;

use crate::{
    dto::{
        order::{CancelOrderRequest, CheckoutRequest, CheckoutResponse, OrderDetail, OrderSummary},
        page::Page,
    },
    error::{AppError, AppResult},
    extractors::{Customer, Pagination, PathParam, ValidJson},
    repositories::orders,
    services::order_service,
    state::AppState,
};

fn idempotency_key(headers: &HeaderMap) -> AppResult<&str> {
    let key = headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::BadRequest("Idempotency-Key header is required".into()))?;
    let ok = (8..=64).contains(&key.len())
        && key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    if ok {
        Ok(key)
    } else {
        Err(AppError::BadRequest(
            "Idempotency-Key must be 8-64 characters of A-Z, a-z, 0-9, - and _".into(),
        ))
    }
}

/// `POST /v1/customer/checkout` (requires `Idempotency-Key`)
pub async fn checkout(
    State(state): State<AppState>,
    user: Customer,
    headers: HeaderMap,
    ValidJson(body): ValidJson<CheckoutRequest>,
) -> AppResult<(StatusCode, Json<CheckoutResponse>)> {
    // TODO(phase-8): rate-limit checkout per user.
    let key = idempotency_key(&headers)?;
    let res = order_service::checkout(&state, &user, key, &body).await?;
    Ok((StatusCode::CREATED, Json(res)))
}

/// `GET /v1/customer/orders`
pub async fn list(
    State(state): State<AppState>,
    user: Customer,
    page: Pagination,
) -> AppResult<Json<Page<OrderSummary>>> {
    let (rows, total) =
        orders::list_for_user(&state.db, user.user_id, page.limit, page.offset).await?;
    Ok(Json(Page {
        items: rows
            .iter()
            .map(|r| order_service::summary(&r.order, r.item_count))
            .collect(),
        total,
        limit: page.limit,
        offset: page.offset,
    }))
}

/// `GET /v1/customer/orders/{id}`
pub async fn get(
    State(state): State<AppState>,
    user: Customer,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<OrderDetail>> {
    Ok(Json(
        order_service::get_for_customer(&state, &user, id).await?,
    ))
}

/// `POST /v1/customer/orders/{id}/cancel`
pub async fn cancel(
    State(state): State<AppState>,
    user: Customer,
    PathParam(id): PathParam<Uuid>,
    ValidJson(body): ValidJson<CancelOrderRequest>,
) -> AppResult<Json<OrderDetail>> {
    Ok(Json(
        order_service::cancel_by_customer(&state, &user, id, body.reason.as_deref()).await?,
    ))
}
