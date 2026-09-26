use axum::{Router, routing::get};

use crate::{state::AppState, ws};

pub fn router() -> Router<AppState> {
    // Authenticated by the first message, not headers (browsers can't set them).
    // TODO(phase-7): customer order tracking + admin board.
    Router::new()
        .route("/picker", get(ws::picker))
        .route("/rider", get(ws::rider))
}
