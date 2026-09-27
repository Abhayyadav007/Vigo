use axum::{
    Router,
    routing::{get, post, put},
};

use crate::{handlers::rider, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/me", get(rider::me))
        .route("/profile", put(rider::update_profile))
        .route("/status", put(rider::set_status))
        .route("/location", post(rider::location))
        .route("/offers", get(rider::offers))
        .route("/offers/{order_id}/accept", post(rider::accept))
        .route("/offers/{order_id}/decline", post(rider::decline))
        .route("/delivery", get(rider::active))
        .route("/deliveries", get(rider::history))
        .route("/deliveries/{order_id}/pickup", post(rider::pickup))
        .route("/deliveries/{order_id}/depart", post(rider::depart))
        .route("/deliveries/{order_id}/deliver", post(rider::deliver))
        .route("/deliveries/{order_id}/unassign", post(rider::unassign))
}
