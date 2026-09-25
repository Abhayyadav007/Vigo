use axum::{
    Router,
    routing::{get, patch},
};

use crate::{handlers::admin::staff, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/users", get(staff::list_users))
        .route("/users/{id}/role", patch(staff::update_role))
}
