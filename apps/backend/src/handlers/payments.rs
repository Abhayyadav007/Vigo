use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
};

use crate::{error::AppResult, services::order_service, state::AppState};

/// `POST /v1/payments/razorpay/webhook`: signature-verified, idempotent per event id.
pub async fn razorpay_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> AppResult<StatusCode> {
    let header = |name: &str| headers.get(name).and_then(|v| v.to_str().ok());
    order_service::handle_razorpay_webhook(
        &state,
        &body,
        header("x-razorpay-signature"),
        header("x-razorpay-event-id"),
    )
    .await?;
    Ok(StatusCode::OK)
}
