use axum::{Json, extract::State};
use uuid::Uuid;

use crate::{
    dto::order::{CartQuery, CartResponse, SetCartItemRequest},
    error::AppResult,
    extractors::{Customer, PathParam, ValidJson, ValidQuery},
    services::cart_service,
    state::AppState,
};

/// `GET /v1/customer/cart?storeId=`
pub async fn get(
    State(state): State<AppState>,
    user: Customer,
    ValidQuery(q): ValidQuery<CartQuery>,
) -> AppResult<Json<CartResponse>> {
    Ok(Json(
        cart_service::get(&state, user.user_id, q.store_id).await?,
    ))
}

/// `PUT /v1/customer/cart/items/{productId}` (quantity 0 removes)
pub async fn set_item(
    State(state): State<AppState>,
    user: Customer,
    PathParam(product_id): PathParam<Uuid>,
    ValidJson(body): ValidJson<SetCartItemRequest>,
) -> AppResult<Json<CartResponse>> {
    Ok(Json(
        cart_service::set_item(
            &state,
            user.user_id,
            body.store_id,
            product_id,
            body.quantity,
        )
        .await?,
    ))
}

/// `DELETE /v1/customer/cart?storeId=`
pub async fn clear(
    State(state): State<AppState>,
    user: Customer,
    ValidQuery(q): ValidQuery<CartQuery>,
) -> AppResult<Json<CartResponse>> {
    Ok(Json(
        cart_service::clear(&state, user.user_id, q.store_id).await?,
    ))
}
