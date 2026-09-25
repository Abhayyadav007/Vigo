use axum::{
    Router,
    routing::{get, post},
};

use crate::{handlers::auth, state::AppState};

pub fn router() -> Router<AppState> {
    // TODO(phase-8): rate-limit /sync per IP and per Firebase UID.
    Router::new()
        .route("/sync", post(auth::sync))
        .route("/me", get(auth::me))
}
