//! Public (no sign-in needed) browsing: serviceability and the store catalog.

use axum::{Json, extract::State};
use uuid::Uuid;

use crate::{
    dto::{
        customer::{
            CatalogCategory, CatalogProduct, CatalogProductQuery, ServiceabilityQuery,
            ServiceabilityResponse, StoreScope,
        },
        page::Page,
    },
    error::AppResult,
    extractors::{Pagination, PathParam, ValidQuery},
    repositories::catalog::CatalogFilter,
    services::customer_catalog_service,
    state::AppState,
};

/// `GET /v1/customer/serviceability?lat=&lng=`
pub async fn serviceability(
    State(state): State<AppState>,
    ValidQuery(q): ValidQuery<ServiceabilityQuery>,
) -> AppResult<Json<ServiceabilityResponse>> {
    Ok(Json(
        customer_catalog_service::serviceability(&state, q.lat, q.lng).await?,
    ))
}

/// `GET /v1/customer/catalog/categories?storeId=`
pub async fn categories(
    State(state): State<AppState>,
    ValidQuery(scope): ValidQuery<StoreScope>,
) -> AppResult<Json<Vec<CatalogCategory>>> {
    let rows = customer_catalog_service::categories(&state, scope.store_id).await?;
    Ok(Json(rows.into_iter().map(CatalogCategory::from).collect()))
}

/// `GET /v1/customer/catalog/products?storeId=&categoryId=&q=`
pub async fn products(
    State(state): State<AppState>,
    page: Pagination,
    ValidQuery(q): ValidQuery<CatalogProductQuery>,
) -> AppResult<Json<Page<CatalogProduct>>> {
    let filter = CatalogFilter {
        category_id: q.category_id,
        q: q.q.as_deref().map(str::trim).filter(|s| !s.is_empty()),
        limit: page.limit,
        offset: page.offset,
    };
    let (rows, total) = customer_catalog_service::products(&state, q.store_id, &filter).await?;
    Ok(Json(Page {
        items: rows.into_iter().map(CatalogProduct::from).collect(),
        total,
        limit: page.limit,
        offset: page.offset,
    }))
}

/// `GET /v1/customer/catalog/products/{id}?storeId=`
pub async fn product(
    State(state): State<AppState>,
    PathParam(id): PathParam<Uuid>,
    ValidQuery(scope): ValidQuery<StoreScope>,
) -> AppResult<Json<CatalogProduct>> {
    let row = customer_catalog_service::product(&state, scope.store_id, id).await?;
    Ok(Json(row.into()))
}
