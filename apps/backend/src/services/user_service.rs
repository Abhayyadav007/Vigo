use uuid::Uuid;

use crate::{
    auth::Role,
    error::{AppError, AppResult},
    extractors::auth_user::AuthUser,
    models::user::User,
    repositories::{
        stores,
        users::{self, UserFilter},
    },
    services::auth_service,
    state::AppState,
};

pub async fn list(state: &AppState, filter: &UserFilter<'_>) -> AppResult<(Vec<User>, i64)> {
    Ok(users::list(&state.db, filter).await?)
}

/// Admin changes a user's role (and dark store). Takes effect on the user's
/// next request: the session cache entry is dropped immediately.
pub async fn change_role(
    state: &AppState,
    actor: &AuthUser,
    user_id: Uuid,
    role: Role,
    store_id: Option<Uuid>,
) -> AppResult<User> {
    if actor.user_id == user_id {
        return Err(AppError::Conflict(
            "admins cannot change their own role".into(),
        ));
    }
    // Store staff work at exactly one store; customers and admins have none.
    let store_id = match role {
        Role::Picker | Role::Rider => {
            let id = store_id.ok_or_else(|| {
                AppError::Validation(format!("a {} must be assigned to a store", role.as_str()))
            })?;
            if !stores::is_active(&state.db, id).await? {
                return Err(AppError::Validation(
                    "store does not exist or is inactive".into(),
                ));
            }
            Some(id)
        }
        Role::Customer | Role::Admin => None,
    };
    let user = users::update_role(&state.db, user_id, role, store_id)
        .await?
        .ok_or(AppError::NotFound("user"))?;
    auth_service::invalidate_best_effort(state, &user.firebase_uid).await;
    tracing::info!(actor = %actor.user_id, user_id = %user.id, role = role.as_str(), "role changed");
    Ok(user)
}

/// Bootstrap path for the first admin (`backend promote-admin <phone>`).
pub async fn promote_admin_by_phone(state: &AppState, phone: &str) -> AppResult<User> {
    let user = users::set_role_by_phone(&state.db, phone, Role::Admin)
        .await?
        .ok_or(AppError::NotFound(
            "user with that phone (sign in once first)",
        ))?;
    auth_service::invalidate_best_effort(state, &user.firebase_uid).await;
    Ok(user)
}
