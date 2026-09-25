use axum::{Json, extract::State};
use uuid::Uuid;

use crate::{
    dto::{
        admin::{AdminUser, UpdateRoleRequest, UserListQuery},
        page::Page,
    },
    error::AppResult,
    extractors::{Admin, Pagination, PathParam, ValidJson, ValidQuery},
    repositories::users::UserFilter,
    services::user_service,
    state::AppState,
};

/// `GET /v1/admin/users?phone=&role=&limit=&offset=`
pub async fn list_users(
    State(state): State<AppState>,
    _admin: Admin,
    page: Pagination,
    ValidQuery(query): ValidQuery<UserListQuery>,
) -> AppResult<Json<Page<AdminUser>>> {
    let filter = UserFilter {
        phone: query.phone.as_deref(),
        role: query.role,
        limit: page.limit,
        offset: page.offset,
    };
    let (users, total) = user_service::list(&state, &filter).await?;
    Ok(Json(Page {
        items: users.into_iter().map(AdminUser::from).collect(),
        total,
        limit: page.limit,
        offset: page.offset,
    }))
}

/// `PATCH /v1/admin/users/{id}/role`
pub async fn update_role(
    State(state): State<AppState>,
    admin: Admin,
    PathParam(user_id): PathParam<Uuid>,
    ValidJson(body): ValidJson<UpdateRoleRequest>,
) -> AppResult<Json<AdminUser>> {
    let user = user_service::change_role(&state, &admin, user_id, body.role, body.store_id).await?;
    Ok(Json(user.into()))
}
