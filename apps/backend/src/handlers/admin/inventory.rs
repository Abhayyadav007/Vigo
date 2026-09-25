use axum::{Json, extract::State};
use uuid::Uuid;

use crate::{
    dto::{
        catalog::{InventoryItem, InventoryListQuery, InventoryRequest},
        page::Page,
    },
    error::{AppError, AppResult},
    extractors::{Admin, Pagination, PathParam, ValidJson, ValidQuery},
    repositories::{inventory, stores},
    services::catalog_service,
    state::AppState,
};

/// `GET /v1/admin/stores/{storeId}/inventory?q=&stocked=`
pub async fn list(
    State(state): State<AppState>,
    _admin: Admin,
    PathParam(store_id): PathParam<Uuid>,
    page: Pagination,
    ValidQuery(query): ValidQuery<InventoryListQuery>,
) -> AppResult<Json<Page<InventoryItem>>> {
    if stores::find(&state.db, store_id).await?.is_none() {
        return Err(AppError::NotFound("store"));
    }
    let filter = inventory::InventoryFilter {
        q: query.q.as_deref().map(str::trim),
        stocked: query.stocked,
        limit: page.limit,
        offset: page.offset,
    };
    let (items, total) = inventory::list_for_store(&state.db, store_id, &filter).await?;
    Ok(Json(Page {
        items: items.into_iter().map(InventoryItem::from).collect(),
        total,
        limit: page.limit,
        offset: page.offset,
    }))
}

/// `PUT /v1/admin/stores/{storeId}/inventory/{productId}`
pub async fn upsert(
    State(state): State<AppState>,
    _admin: Admin,
    PathParam((store_id, product_id)): PathParam<(Uuid, Uuid)>,
    ValidJson(body): ValidJson<InventoryRequest>,
) -> AppResult<Json<InventoryItem>> {
    let row = catalog_service::set_inventory(&state, store_id, product_id, &body).await?;
    Ok(Json(row.into()))
}
