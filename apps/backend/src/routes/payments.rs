use axum::{Router, routing::post};

use crate::{handlers::payments, state::AppState};

pub fn router() -> Router<AppState> {
    // Called by the gateway, authenticated by signature (not a user token).
    Router::new().route("/razorpay/webhook", post(payments::razorpay_webhook))
}
