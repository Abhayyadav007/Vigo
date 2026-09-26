use axum::{
    Router,
    routing::{get, post, put},
};

use crate::{handlers::picker, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/orders", get(picker::queue))
        .route("/orders/{id}", get(picker::pick_list))
        .route("/orders/{id}/start", post(picker::start))
        .route("/orders/{id}/scan", post(picker::scan))
        .route("/orders/{id}/items/{product_id}", put(picker::set_picked))
        .route("/orders/{id}/pack", post(picker::pack))
        .route("/orders/{id}/release", post(picker::release))
        .route("/orders/{id}/cancel", post(picker::cancel))
}
