use axum::{Json, extract::State, http::StatusCode};
use uuid::Uuid;

use crate::{
    dto::{
        page::Page,
        store::{AdminStore, StoreListQuery, StoreRequest},
    },
    error::{AppError, AppResult},
    extractors::{Admin, Pagination, PathParam, ValidJson, ValidQuery},
    repositories::stores,
    services::store_service,
    state::AppState,
};

/// `GET /v1/admin/stores`
pub async fn list(
    State(state): State<AppState>,
    _admin: Admin,
    page: Pagination,
    ValidQuery(query): ValidQuery<StoreListQuery>,
) -> AppResult<Json<Page<AdminStore>>> {
    let (items, total) =
        stores::list(&state.db, query.q.as_deref(), page.limit, page.offset).await?;
    Ok(Json(Page {
        items: items.into_iter().map(AdminStore::from).collect(),
        total,
        limit: page.limit,
        offset: page.offset,
    }))
}

/// `GET /v1/admin/stores/{id}`
pub async fn get(
    State(state): State<AppState>,
    _admin: Admin,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<AdminStore>> {
    let store = stores::find(&state.db, id)
        .await?
        .ok_or(AppError::NotFound("store"))?;
    Ok(Json(store.into()))
}

/// `POST /v1/admin/stores`
pub async fn create(
    State(state): State<AppState>,
    _admin: Admin,
    ValidJson(body): ValidJson<StoreRequest>,
) -> AppResult<(StatusCode, Json<AdminStore>)> {
    let store = store_service::create(&state, body).await?;
    Ok((StatusCode::CREATED, Json(store.into())))
}

/// `PUT /v1/admin/stores/{id}`
pub async fn update(
    State(state): State<AppState>,
    _admin: Admin,
    PathParam(id): PathParam<Uuid>,
    ValidJson(body): ValidJson<StoreRequest>,
) -> AppResult<Json<AdminStore>> {
    Ok(Json(store_service::update(&state, id, body).await?.into()))
}
