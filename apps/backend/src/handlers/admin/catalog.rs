use axum::{Json, extract::State, http::StatusCode};
use uuid::Uuid;

use crate::{
    dto::{
        catalog::{AdminCategory, AdminProduct, CategoryRequest, ProductListQuery, ProductRequest},
        page::Page,
    },
    error::{AppError, AppResult},
    extractors::{Admin, Pagination, PathParam, ValidJson, ValidQuery},
    repositories::{categories, products},
    services::catalog_service,
    state::AppState,
};

/// `GET /v1/admin/categories` (all; the tree is small)
pub async fn list_categories(
    State(state): State<AppState>,
    _admin: Admin,
) -> AppResult<Json<Vec<AdminCategory>>> {
    let all = categories::list_all(&state.db).await?;
    Ok(Json(all.into_iter().map(AdminCategory::from).collect()))
}

/// `POST /v1/admin/categories`
pub async fn create_category(
    State(state): State<AppState>,
    _admin: Admin,
    ValidJson(body): ValidJson<CategoryRequest>,
) -> AppResult<(StatusCode, Json<AdminCategory>)> {
    let c = catalog_service::create_category(&state, &body).await?;
    Ok((StatusCode::CREATED, Json(c.into())))
}

/// `PUT /v1/admin/categories/{id}`
pub async fn update_category(
    State(state): State<AppState>,
    _admin: Admin,
    PathParam(id): PathParam<Uuid>,
    ValidJson(body): ValidJson<CategoryRequest>,
) -> AppResult<Json<AdminCategory>> {
    Ok(Json(
        catalog_service::update_category(&state, id, &body)
            .await?
            .into(),
    ))
}

/// `GET /v1/admin/products?q=&categoryId=&isActive=`
pub async fn list_products(
    State(state): State<AppState>,
    _admin: Admin,
    page: Pagination,
    ValidQuery(query): ValidQuery<ProductListQuery>,
) -> AppResult<Json<Page<AdminProduct>>> {
    let filter = products::ProductFilter {
        q: query.q.as_deref().map(str::trim),
        category_id: query.category_id,
        is_active: query.is_active,
        limit: page.limit,
        offset: page.offset,
    };
    let (items, total) = products::list(&state.db, &filter).await?;
    Ok(Json(Page {
        items: items.into_iter().map(AdminProduct::from).collect(),
        total,
        limit: page.limit,
        offset: page.offset,
    }))
}

/// `GET /v1/admin/products/{id}`
pub async fn get_product(
    State(state): State<AppState>,
    _admin: Admin,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<AdminProduct>> {
    let p = products::find(&state.db, id)
        .await?
        .ok_or(AppError::NotFound("product"))?;
    Ok(Json(p.into()))
}

/// `POST /v1/admin/products`
pub async fn create_product(
    State(state): State<AppState>,
    _admin: Admin,
    ValidJson(body): ValidJson<ProductRequest>,
) -> AppResult<(StatusCode, Json<AdminProduct>)> {
    let p = catalog_service::create_product(&state, &body).await?;
    Ok((StatusCode::CREATED, Json(p.into())))
}

/// `PUT /v1/admin/products/{id}`
pub async fn update_product(
    State(state): State<AppState>,
    _admin: Admin,
    PathParam(id): PathParam<Uuid>,
    ValidJson(body): ValidJson<ProductRequest>,
) -> AppResult<Json<AdminProduct>> {
    Ok(Json(
        catalog_service::update_product(&state, id, &body)
            .await?
            .into(),
    ))
}
