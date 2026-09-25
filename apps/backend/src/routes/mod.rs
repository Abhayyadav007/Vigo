//! Router declarations only: paths are wired to handler functions, no logic here.

use axum::{Router, routing::get};

use crate::{error::AppError, handlers, state::AppState};

mod admin;
mod auth;
mod customer;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/healthz", get(handlers::health::healthz))
        .nest("/v1", v1())
        .fallback(|| async { AppError::NotFound("route") })
}

fn v1() -> Router<AppState> {
    // TODO(phase-5): picker router + ws; TODO(phase-6): rider router.
    Router::new()
        .nest("/auth", auth::router())
        .nest("/customer", customer::router())
        .nest("/admin", admin::router())
}
