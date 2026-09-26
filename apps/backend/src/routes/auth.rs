use axum::{
    Router,
    routing::{get, post},
};

use crate::{handlers::auth, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/sync", post(auth::sync))
        .route("/me", get(auth::me))
}
