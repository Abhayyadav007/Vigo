use axum::{Json, extract::State, http::StatusCode};
use uuid::Uuid;

use crate::{
    dto::{
        admin::{AdminCancelRequest, BoardOrder, Metrics, StoreFilter},
        order::display_number,
    },
    error::{AppError, AppResult},
    extractors::{OrderStaff, PathParam, ValidJson, ValidQuery},
    models::order::OrderStatus,
    repositories::orders,
    services::order_service,
    state::AppState,
};

/// `GET /v1/admin/orders?storeId=`: orders in flight (live board; updates on `/v1/ws/admin`).
pub async fn board(
    State(state): State<AppState>,
    user: OrderStaff,
    ValidQuery(q): ValidQuery<StoreFilter>,
) -> AppResult<Json<Vec<BoardOrder>>> {
    let store_id = user.scoped_store_filter(q.store_id)?;
    let rows = orders::board(&state.db, store_id).await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| BoardOrder {
                id: r.id,
                number: display_number(r.number),
                status: r.status,
                store_code: r.store_code,
                payment_method: r.payment_method,
                total_paise: r.total_paise,
                item_count: r.item_count,
                rider_phone: r.rider_phone,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
            .collect(),
    ))
}

/// `GET /v1/admin/metrics?storeId=`: today's numbers (IST).
pub async fn metrics(
    State(state): State<AppState>,
    user: OrderStaff,
    ValidQuery(q): ValidQuery<StoreFilter>,
) -> AppResult<Json<Metrics>> {
    let store_id = user.scoped_store_filter(q.store_id)?;
    let m = orders::metrics_today(&state.db, store_id).await?;
    Ok(Json(Metrics {
        orders_today: m.orders,
        delivered_today: m.delivered,
        cancelled_today: m.cancelled,
        gmv_today_paise: m.gmv_paise,
        avg_delivery_minutes: m.avg_delivery_minutes.map(|v| (v * 10.0).round() / 10.0),
        riders_online: m.riders_online,
    }))
}

/// `POST /v1/admin/orders/{id}/cancel`: support or the store cancels an order
/// that hasn't been picked up yet. Prepaid orders are marked refunded.
pub async fn cancel(
    State(state): State<AppState>,
    user: OrderStaff,
    PathParam(id): PathParam<Uuid>,
    ValidJson(body): ValidJson<AdminCancelRequest>,
) -> AppResult<StatusCode> {
    let order = orders::find(&state.db, id)
        .await?
        .ok_or(AppError::NotFound("order"))?;
    if user.store_scope().is_some_and(|own| own != order.store_id) {
        return Err(AppError::NotFound("order"));
    }
    let reason = body.reason.trim();
    let from = OrderStatus::predecessors(OrderStatus::Cancelled);
    order_service::cancel(&state, id, Some(user.user_id), reason, &from).await?;
    tracing::info!(actor = %user.user_id, role = user.role.as_str(), order_id = %id, "order cancelled by staff");
    Ok(StatusCode::NO_CONTENT)
}
