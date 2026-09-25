//! Router declarations only: paths are wired to handler functions, no logic here.

use axum::{Router, routing::get};

use crate::{error::AppError, handlers, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/healthz", get(handlers::health::healthz))
        .nest("/v1", v1())
        .fallback(|| async { AppError::NotFound("route") })
}

fn v1() -> Router<AppState> {
    // TODO(phase-2): nest auth, customer, picker, rider, admin and ws routers here.
    Router::new()
}
