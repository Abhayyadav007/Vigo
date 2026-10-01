use axum::{Json, extract::State, http::StatusCode};
use uuid::Uuid;

use crate::{
    dto::{
        page::Page,
        store::{AdminStore, StoreActiveRequest, StoreListQuery, StoreRequest},
    },
    error::{AppError, AppResult},
    extractors::{Admin, BackOffice, Pagination, PathParam, StoreStaff, ValidJson, ValidQuery},
    repositories::stores,
    services::store_service,
    state::AppState,
};

/// `GET /v1/admin/stores` (a store manager sees only their own store)
pub async fn list(
    State(state): State<AppState>,
    user: BackOffice,
    page: Pagination,
    ValidQuery(query): ValidQuery<StoreListQuery>,
) -> AppResult<Json<Page<AdminStore>>> {
    if let Some(own) = user.store_scope() {
        let items: Vec<AdminStore> = stores::find(&state.db, own)
            .await?
            .into_iter()
            .map(AdminStore::from)
            .collect();
        return Ok(Json(Page {
            total: items.len() as i64,
            items,
            limit: page.limit,
            offset: page.offset,
        }));
    }
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
    user: BackOffice,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<AdminStore>> {
    user.ensure_store(id)?;
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

/// `PATCH /v1/admin/stores/{id}/active`: open or close a store.
pub async fn set_active(
    State(state): State<AppState>,
    user: StoreStaff,
    PathParam(id): PathParam<Uuid>,
    ValidJson(body): ValidJson<StoreActiveRequest>,
) -> AppResult<Json<AdminStore>> {
    user.ensure_store(id)?;
    let store = store_service::set_active(&state, id, body.is_active).await?;
    tracing::info!(actor = %user.user_id, store_id = %id, is_active = body.is_active, "store switched");
    Ok(Json(store.into()))
}
