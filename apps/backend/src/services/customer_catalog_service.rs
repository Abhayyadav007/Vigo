use uuid::Uuid;

use crate::{
    dto::customer::{ServiceabilityResponse, StoreSummary},
    error::{AppError, AppResult},
    models::catalog::{CatalogCategoryRow, CatalogProductRow},
    repositories::{catalog, stores},
    services::store_service::eta_minutes,
    state::AppState,
};

pub async fn serviceability(
    state: &AppState,
    lat: f64,
    lng: f64,
) -> AppResult<ServiceabilityResponse> {
    let store = stores::find_serving(&state.db, lat, lng).await?;
    Ok(ServiceabilityResponse {
        serviceable: store.is_some(),
        store: store.map(|s| StoreSummary {
            id: s.id,
            code: s.code,
            name: s.name,
            eta_minutes: eta_minutes(s.distance_m),
        }),
    })
}

async fn require_active_store(state: &AppState, store_id: Uuid) -> AppResult<()> {
    if stores::is_active(&state.db, store_id).await? {
        Ok(())
    } else {
        Err(AppError::NotFound("store"))
    }
}

pub async fn categories(state: &AppState, store_id: Uuid) -> AppResult<Vec<CatalogCategoryRow>> {
    require_active_store(state, store_id).await?;
    Ok(catalog::categories(&state.db, store_id).await?)
}

pub async fn products(
    state: &AppState,
    store_id: Uuid,
    filter: &catalog::CatalogFilter<'_>,
) -> AppResult<(Vec<CatalogProductRow>, i64)> {
    require_active_store(state, store_id).await?;
    Ok(catalog::products(&state.db, store_id, filter).await?)
}

pub async fn product(
    state: &AppState,
    store_id: Uuid,
    product_id: Uuid,
) -> AppResult<CatalogProductRow> {
    catalog::product(&state.db, store_id, product_id)
        .await?
        .ok_or(AppError::NotFound("product"))
}
