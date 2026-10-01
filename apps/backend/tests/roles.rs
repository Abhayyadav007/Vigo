// Tests may panic freely; the no-unwrap/expect rule is for production code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Back-office roles: what each one may do, and store managers' store scoping.

mod common;

use axum::http::{Method, StatusCode};
use common::{TestApp, admin_token, create_store, error_code, random_phone};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

/// Inside the test store's area (Indiranagar, Bengaluru).
const HOME: (f64, f64) = (12.9730, 77.6420);

/// Signs in a fresh user and gives them `role` (and `store`); returns their token.
async fn staff(app: &TestApp, admin: &str, role: &str, store: Option<&Value>) -> String {
    let (token, me) = app.signed_in(&random_phone()).await;
    let mut body = json!({ "role": role });
    if let Some(s) = store {
        body["storeId"] = s.clone();
    }
    let uri = format!("/v1/admin/users/{}/role", me["id"].as_str().unwrap());
    let (status, res) = app
        .request(Method::PATCH, &uri, Some(admin), Some(body))
        .await;
    assert_eq!(status, StatusCode::OK, "{res}");
    assert_eq!(res["role"], role);
    token
}

async fn category(app: &TestApp, token: &str) -> (StatusCode, Value) {
    app.request(
        Method::POST,
        "/v1/admin/categories",
        Some(token),
        Some(json!({ "name": format!("Cat {}", Uuid::new_v4().simple()), "isActive": true })),
    )
    .await
}

async fn product(app: &TestApp, token: &str, category_id: &Value) -> (StatusCode, Value) {
    app.request(
        Method::POST,
        "/v1/admin/products",
        Some(token),
        Some(json!({
            "categoryId": category_id, "name": "Toned milk", "unitLabel": "500 ml",
            "slug": format!("p-{}", Uuid::new_v4().simple()),
            "mrpPaise": 3000, "pricePaise": 2800, "isActive": true,
        })),
    )
    .await
}

async fn stock(app: &TestApp, token: &str, store: &Value, product: &Value) -> (StatusCode, Value) {
    let uri = format!(
        "/v1/admin/stores/{}/inventory/{}",
        store.as_str().unwrap(),
        product.as_str().unwrap()
    );
    app.request(
        Method::PUT,
        &uri,
        Some(token),
        Some(json!({ "quantity": 20, "isAvailable": true })),
    )
    .await
}

#[sqlx::test]
async fn store_manager_needs_a_store(db: PgPool) {
    let app = TestApp::new(db);
    let admin = admin_token(&app).await;
    let (_, me) = app.signed_in(&random_phone()).await;
    let uri = format!("/v1/admin/users/{}/role", me["id"].as_str().unwrap());
    let (status, body) = app
        .request(
            Method::PATCH,
            &uri,
            Some(&admin),
            Some(json!({ "role": "STORE_MANAGER" })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");

    // Global back-office roles never keep a store.
    let store = create_store(&app, &admin, "BLR-R0").await;
    let (status, body) = app
        .request(
            Method::PATCH,
            &uri,
            Some(&admin),
            Some(json!({ "role": "SUPPORT_AGENT", "storeId": store })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["storeId"], Value::Null);
}

#[sqlx::test]
async fn catalog_manager_owns_the_catalog_only(db: PgPool) {
    let app = TestApp::new(db);
    let admin = admin_token(&app).await;
    let store = create_store(&app, &admin, "BLR-R1").await;
    let catalog = staff(&app, &admin, "CATALOG_MANAGER", None).await;

    let (status, cat) = category(&app, &catalog).await;
    assert_eq!(status, StatusCode::CREATED, "{cat}");
    let (status, p) = product(&app, &catalog, &cat["id"]).await;
    assert_eq!(status, StatusCode::CREATED, "{p}");
    let (status, _) = app.get("/v1/admin/products", Some(&catalog)).await;
    assert_eq!(status, StatusCode::OK);

    // Not stores, stock, staff or orders.
    let (status, _) = stock(&app, &catalog, &store, &p["id"]).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = app
        .request(
            Method::POST,
            "/v1/admin/stores",
            Some(&catalog),
            Some(common::store_body("BLR-R1X", 12.97, 77.64, 1.0)),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = app.get("/v1/admin/users", Some(&catalog)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = app.get("/v1/admin/orders", Some(&catalog)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn store_manager_is_limited_to_their_store(db: PgPool) {
    let app = TestApp::new(db);
    let admin = admin_token(&app).await;
    let mine = create_store(&app, &admin, "BLR-R2A").await;
    let other = create_store(&app, &admin, "BLR-R2B").await;
    let manager = staff(&app, &admin, "STORE_MANAGER", Some(&mine)).await;
    let (_, cat) = category(&app, &admin).await;
    let (_, p) = product(&app, &admin, &cat["id"]).await;

    // Own store: stock, inventory, open/closed switch, board.
    let (status, body) = stock(&app, &manager, &mine, &p["id"]).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let mine_id = mine.as_str().unwrap();
    let (status, _) = app
        .get(
            &format!("/v1/admin/stores/{mine_id}/inventory"),
            Some(&manager),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = app
        .request(
            Method::PATCH,
            &format!("/v1/admin/stores/{mine_id}/active"),
            Some(&manager),
            Some(json!({ "isActive": false })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["isActive"], false);
    let (status, _) = app.get("/v1/admin/orders", Some(&manager)).await;
    assert_eq!(status, StatusCode::OK);

    // The store list shows only theirs.
    let (status, list) = app.get("/v1/admin/stores", Some(&manager)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list["total"], 1);
    assert_eq!(list["items"][0]["id"], mine);

    // Another store looks like it doesn't exist.
    let other_id = other.as_str().unwrap();
    let (status, body) = stock(&app, &manager, &other, &p["id"]).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    let (status, _) = app
        .get(&format!("/v1/admin/stores/{other_id}"), Some(&manager))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = app
        .request(
            Method::PATCH,
            &format!("/v1/admin/stores/{other_id}/active"),
            Some(&manager),
            Some(json!({ "isActive": false })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = app
        .get(
            &format!("/v1/admin/orders?storeId={other_id}"),
            Some(&manager),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // No catalog edits, store edits or staff changes.
    let (status, _) = category(&app, &manager).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = app
        .request(
            Method::PUT,
            &format!("/v1/admin/stores/{mine_id}"),
            Some(&manager),
            Some(common::store_body("BLR-R2A", 12.9719, 77.6412, 2.0)),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = app.get("/v1/admin/users", Some(&manager)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn support_and_store_cancel_orders(db: PgPool) {
    let app = TestApp::new(db);
    let admin = admin_token(&app).await;
    let store = create_store(&app, &admin, "BLR-R3").await;
    let elsewhere = create_store(&app, &admin, "BLR-R3B").await;
    let (_, cat) = category(&app, &admin).await;
    let (_, p) = product(&app, &admin, &cat["id"]).await;
    let (status, _) = stock(&app, &admin, &store, &p["id"]).await;
    assert_eq!(status, StatusCode::OK);

    let support = staff(&app, &admin, "SUPPORT_AGENT", None).await;
    let other_manager = staff(&app, &admin, "STORE_MANAGER", Some(&elsewhere)).await;

    // A customer places a COD order.
    let (customer, _) = app.signed_in(&random_phone()).await;
    let (status, addr) = app
        .request(
            Method::POST,
            "/v1/customer/addresses",
            Some(&customer),
            Some(json!({
                "label": "Home", "line1": "Flat 4B", "city": "Bengaluru", "pincode": "560038",
                "location": { "lat": HOME.0, "lng": HOME.1 },
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{addr}");
    let (status, body) = app
        .request(
            Method::PUT,
            &format!("/v1/customer/cart/items/{}", p["id"].as_str().unwrap()),
            Some(&customer),
            Some(json!({ "storeId": store, "quantity": 2 })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let checkout = json!({ "storeId": store, "addressId": addr["id"], "paymentMethod": "COD" });
    let (status, placed) = app
        .post_with_headers(
            "/v1/customer/checkout",
            Some(&customer),
            &[("idempotency-key", "roles-test-1")],
            checkout.to_string().as_bytes(),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{placed}");
    let order_id = placed["order"]["id"].as_str().unwrap();
    let cancel_uri = format!("/v1/admin/orders/{order_id}/cancel");

    // Support can see it on the board.
    let (status, board) = app.get("/v1/admin/orders", Some(&support)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        board
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["id"] == order_id)
    );

    // Another store's manager can't touch it; a customer can't use this route.
    let reason = json!({ "reason": "Customer called support" });
    let (status, _) = app
        .request(
            Method::POST,
            &cancel_uri,
            Some(&other_manager),
            Some(reason.clone()),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = app
        .request(
            Method::POST,
            &cancel_uri,
            Some(&customer),
            Some(reason.clone()),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, body) = app
        .request(
            Method::POST,
            &cancel_uri,
            Some(&support),
            Some(reason.clone()),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "{body}");
    let (_, order) = app
        .get(&format!("/v1/customer/orders/{order_id}"), Some(&customer))
        .await;
    assert_eq!(order["status"], "CANCELLED");
    assert_eq!(order["cancelReason"], "Customer called support");

    // Cancelling twice is an invalid transition, not a crash.
    let (status, body) = app
        .request(Method::POST, &cancel_uri, Some(&support), Some(reason))
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_ne!(error_code(&body), "<none>");

    // Support doesn't manage stock.
    let (status, _) = stock(&app, &support, &store, &p["id"]).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
