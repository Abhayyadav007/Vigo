use axum::{Json, extract::State};

use crate::{
    dto::{
        admin::{BoardOrder, Metrics, StoreFilter},
        order::display_number,
    },
    error::AppResult,
    extractors::{Admin, ValidQuery},
    repositories::orders,
    state::AppState,
};

/// `GET /v1/admin/orders?storeId=`: orders in flight (live board; updates on `/v1/ws/admin`).
pub async fn board(
    State(state): State<AppState>,
    _admin: Admin,
    ValidQuery(q): ValidQuery<StoreFilter>,
) -> AppResult<Json<Vec<BoardOrder>>> {
    let rows = orders::board(&state.db, q.store_id).await?;
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
    _admin: Admin,
    ValidQuery(q): ValidQuery<StoreFilter>,
) -> AppResult<Json<Metrics>> {
    let m = orders::metrics_today(&state.db, q.store_id).await?;
    Ok(Json(Metrics {
        orders_today: m.orders,
        delivered_today: m.delivered,
        cancelled_today: m.cancelled,
        gmv_today_paise: m.gmv_paise,
        avg_delivery_minutes: m.avg_delivery_minutes.map(|v| (v * 10.0).round() / 10.0),
        riders_online: m.riders_online,
    }))
}
