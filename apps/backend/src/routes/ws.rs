use axum::{Router, routing::get};

use crate::{state::AppState, ws};

pub fn router() -> Router<AppState> {
    // Authenticated by the first message, not headers (browsers can't set them).
    Router::new()
        .route("/picker", get(ws::picker))
        .route("/rider", get(ws::rider))
        .route("/orders/{id}", get(ws::order))
        .route("/admin", get(ws::admin))
}
