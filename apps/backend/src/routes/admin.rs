use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, patch, post, put},
};

use crate::{
    handlers::admin::{catalog, inventory, staff, stores, uploads},
    services::media_service::MAX_IMAGE_BYTES,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/users", get(staff::list_users))
        .route("/users/{id}/role", patch(staff::update_role))
        .route("/stores", get(stores::list).post(stores::create))
        .route("/stores/{id}", get(stores::get).put(stores::update))
        .route("/stores/{id}/inventory", get(inventory::list))
        .route(
            "/stores/{id}/inventory/{product_id}",
            put(inventory::upsert),
        )
        .route(
            "/categories",
            get(catalog::list_categories).post(catalog::create_category),
        )
        .route("/categories/{id}", put(catalog::update_category))
        .route(
            "/products",
            get(catalog::list_products).post(catalog::create_product),
        )
        .route(
            "/products/{id}",
            get(catalog::get_product).put(catalog::update_product),
        )
        .route(
            "/uploads",
            // Room for multipart framing around a max-size image.
            post(uploads::upload_image).layer(DefaultBodyLimit::max(MAX_IMAGE_BYTES + 64 * 1024)),
        )
}
