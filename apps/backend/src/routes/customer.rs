use axum::{
    Router,
    routing::{get, post, put},
};

use crate::{
    handlers::customer::{addresses, cart, catalog, orders},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        // Public browsing.
        .route("/serviceability", get(catalog::serviceability))
        .route("/catalog/categories", get(catalog::categories))
        .route("/catalog/products", get(catalog::products))
        .route("/catalog/products/{id}", get(catalog::product))
        // Signed-in customers (handlers take the `Customer` guard).
        .route("/addresses", get(addresses::list).post(addresses::create))
        .route(
            "/addresses/{id}",
            put(addresses::update).delete(addresses::delete),
        )
        .route("/cart", get(cart::get).delete(cart::clear))
        .route("/cart/items/{product_id}", put(cart::set_item))
        .route("/checkout", post(orders::checkout))
        .route("/orders", get(orders::list))
        .route("/orders/{id}", get(orders::get))
        .route("/orders/{id}/cancel", post(orders::cancel))
}
