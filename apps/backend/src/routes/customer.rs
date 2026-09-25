use axum::{Router, routing::get};

use crate::{handlers::customer::catalog, state::AppState};

pub fn router() -> Router<AppState> {
    // Browsing is public; cart/checkout (phase 4) will require `Customer`.
    Router::new()
        .route("/serviceability", get(catalog::serviceability))
        .route("/catalog/categories", get(catalog::categories))
        .route("/catalog/products", get(catalog::products))
        .route("/catalog/products/{id}", get(catalog::product))
}
