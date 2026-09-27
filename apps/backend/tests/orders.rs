// Tests may panic freely; the no-unwrap/expect rule is for production code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::time::Duration;

use axum::http::{Method, StatusCode};
use backend::{cache::inventory, models::order::OrderStatus, services::order_service};
use common::{
    RAZORPAY_WEBHOOK_SECRET, TestApp, admin_token, create_store, error_code, random_phone,
    store_body,
};
use futures::StreamExt;
use hmac::{Hmac, KeyInit, Mac};
use serde_json::{Value, json};
use sha2::Sha256;
use sqlx::PgPool;
use uuid::Uuid;

/// Inside the test store's area (Indiranagar, Bengaluru).
const HOME: (f64, f64) = (12.9730, 77.6420);

struct Shop {
    store: Uuid,
    admin: String,
}

async fn shop(app: &TestApp) -> Shop {
    let admin = admin_token(app).await;
    let store = create_store(app, &admin, "BLR-ORD").await;
    Shop {
        store: Uuid::parse_str(store.as_str().unwrap()).unwrap(),
        admin,
    }
}

/// Creates a product priced `price` paise with `stock` units at the shop.
async fn product(app: &TestApp, shop: &Shop, name: &str, price: i64, stock: i32) -> Uuid {
    let (_, cat) = app
        .request(Method::POST, "/v1/admin/categories", Some(&shop.admin),
            Some(json!({ "name": format!("Cat {name} {}", Uuid::new_v4().simple()), "isActive": true })))
        .await;
    let (status, p) = app
        .request(
            Method::POST,
            "/v1/admin/products",
            Some(&shop.admin),
            Some(json!({
                "categoryId": cat["id"], "name": name, "unitLabel": "1 pc",
                "slug": format!("p-{}", Uuid::new_v4().simple()),
                "mrpPaise": price + 500, "pricePaise": price, "isActive": true,
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{p}");
    let id = p["id"].as_str().unwrap();
    let (status, body) = app
        .request(
            Method::PUT,
            &format!("/v1/admin/stores/{}/inventory/{id}", shop.store),
            Some(&shop.admin),
            Some(json!({ "quantity": stock, "isAvailable": true })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    Uuid::parse_str(id).unwrap()
}

/// A signed-in customer with a saved address at `at`; returns (token, address id).
async fn customer(app: &TestApp, at: (f64, f64)) -> (String, String) {
    let (token, _) = app.signed_in(&random_phone()).await;
    let (status, addr) = app
        .request(Method::POST, "/v1/customer/addresses", Some(&token), Some(json!({
            "label": "Home", "line1": "Flat 4B, Lotus Apartments", "line2": "12th Main, HAL 2nd Stage",
            "city": "Bengaluru", "pincode": "560038", "location": { "lat": at.0, "lng": at.1 },
        })))
        .await;
    assert_eq!(status, StatusCode::CREATED, "{addr}");
    (token, addr["id"].as_str().unwrap().to_owned())
}

async fn set_cart(
    app: &TestApp,
    token: &str,
    store: Uuid,
    product: Uuid,
    qty: i32,
) -> (StatusCode, Value) {
    app.request(
        Method::PUT,
        &format!("/v1/customer/cart/items/{product}"),
        Some(token),
        Some(json!({ "storeId": store, "quantity": qty })),
    )
    .await
}

async fn checkout(
    app: &TestApp,
    token: &str,
    store: Uuid,
    address: &str,
    method: &str,
    key: &str,
) -> (StatusCode, Value) {
    let body =
        json!({ "storeId": store, "addressId": address, "paymentMethod": method }).to_string();
    app.post_with_headers(
        "/v1/customer/checkout",
        Some(token),
        &[("idempotency-key", key)],
        body.as_bytes(),
    )
    .await
}

async fn pg_stock(db: &PgPool, store: Uuid, product: Uuid) -> i32 {
    sqlx::query_scalar(
        "SELECT quantity FROM store_inventory WHERE store_id = $1 AND product_id = $2",
    )
    .bind(store)
    .bind(product)
    .fetch_one(db)
    .await
    .unwrap()
}

fn key() -> String {
    format!("key-{}", Uuid::new_v4().simple())
}

fn sign(body: &[u8]) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(RAZORPAY_WEBHOOK_SECRET.as_bytes()).unwrap();
    mac.update(body);
    hex::encode(mac.finalize().into_bytes())
}

fn webhook(event: &str, gateway_order_id: &str, amount: i64) -> Vec<u8> {
    json!({
        "event": event,
        "payload": { "payment": { "entity": {
            "id": format!("pay_{}", Uuid::new_v4().simple()), "order_id": gateway_order_id,
            "amount": amount, "currency": "INR", "status": "captured",
        } } },
    })
    .to_string()
    .into_bytes()
}

#[sqlx::test]
async fn cod_checkout_happy_path(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = shop(&app).await;
    let milk = product(&app, &s, "Milk", 2_800, 5).await;
    let (token, address) = customer(&app, HOME).await;

    let (status, cart) = set_cart(&app, &token, s.store, milk, 2).await;
    assert_eq!(status, StatusCode::OK, "{cart}");
    assert_eq!(cart["itemCount"], 2);
    assert_eq!(cart["bill"]["itemTotalPaise"], 5_600);
    assert_eq!(cart["bill"]["deliveryFeePaise"], 3_000, "below ₹199");
    assert_eq!(cart["bill"]["totalPaise"], 8_600);
    assert_eq!(cart["canCheckout"], true);

    let (status, _) = set_cart(&app, &token, s.store, milk, 6).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "more than stock");

    let (status, res) = checkout(&app, &token, s.store, &address, "COD", &key()).await;
    assert_eq!(status, StatusCode::CREATED, "{res}");
    let order = &res["order"];
    assert_eq!(order["status"], "CONFIRMED");
    assert_eq!(order["paymentMethod"], "COD");
    assert_eq!(order["paymentStatus"], "PENDING");
    assert!(order["number"].as_str().unwrap().starts_with("VG"));
    assert_eq!(order["bill"]["totalPaise"], 8_600);
    assert_eq!(order["items"][0]["quantity"], 2);
    assert_eq!(order["address"]["pincode"], "560038");
    assert!(order["deliveryOtp"].as_str().unwrap().len() == 4);
    assert_eq!(order["canCancel"], true);
    let statuses: Vec<&str> = order["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["status"].as_str().unwrap())
        .collect();
    assert_eq!(statuses, ["PLACED", "CONFIRMED"]);
    assert!(res["razorpay"].is_null());

    assert_eq!(pg_stock(&db, s.store, milk).await, 3);
    assert_eq!(
        inventory::stock(&app.state.redis, s.store, milk)
            .await
            .unwrap(),
        Some(3)
    );
    assert_eq!(
        inventory::held(&app.state.redis, s.store, milk)
            .await
            .unwrap(),
        0
    );

    let (_, cart) = app
        .get(
            &format!("/v1/customer/cart?storeId={}", s.store),
            Some(&token),
        )
        .await;
    assert_eq!(cart["items"].as_array().unwrap().len(), 0, "cart cleared");

    let (_, list) = app.get("/v1/customer/orders", Some(&token)).await;
    assert_eq!(list["total"], 1);
    assert_eq!(list["items"][0]["itemCount"], 2);
}

#[sqlx::test]
async fn checkout_is_idempotent(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = shop(&app).await;
    let milk = product(&app, &s, "Milk", 2_800, 5).await;
    let (token, address) = customer(&app, HOME).await;
    set_cart(&app, &token, s.store, milk, 2).await;

    let k = key();
    let (a, first) = checkout(&app, &token, s.store, &address, "COD", &k).await;
    let (b, second) = checkout(&app, &token, s.store, &address, "COD", &k).await;
    assert_eq!((a, b), (StatusCode::CREATED, StatusCode::CREATED));
    assert_eq!(first["order"]["id"], second["order"]["id"]);
    assert_eq!(pg_stock(&db, s.store, milk).await, 3, "stock taken once");

    let body =
        json!({ "storeId": s.store, "addressId": address, "paymentMethod": "COD" }).to_string();
    let (status, _) = app
        .post_with_headers("/v1/customer/checkout", Some(&token), &[], body.as_bytes())
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "Idempotency-Key required");
}

/// The core promise: concurrent checkouts can never sell more than the stock.
#[sqlx::test]
async fn concurrent_checkouts_never_oversell(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = shop(&app).await;
    let stock = 5;
    let buyers = 16;
    let item = product(&app, &s, "Last Mangoes", 9_900, stock).await;

    let mut shoppers = Vec::new();
    for _ in 0..buyers {
        let (token, address) = customer(&app, HOME).await;
        let (status, _) = set_cart(&app, &token, s.store, item, 1).await;
        assert_eq!(status, StatusCode::OK);
        shoppers.push((token, address));
    }

    let results = futures::future::join_all(shoppers.iter().map(|(token, address)| {
        checkout(
            &app,
            token,
            s.store,
            address,
            "COD",
            Box::leak(key().into_boxed_str()),
        )
    }))
    .await;

    let confirmed = results
        .iter()
        .filter(|(st, r)| *st == StatusCode::CREATED && r["order"]["status"] == "CONFIRMED")
        .count();
    let sold_out = results
        .iter()
        .filter(|(st, r)| *st == StatusCode::CONFLICT && error_code(r) == "CONFLICT")
        .count();
    assert_eq!(confirmed, stock as usize, "{results:?}");
    assert_eq!(sold_out, buyers - stock as usize);
    assert_eq!(pg_stock(&db, s.store, item).await, 0, "never negative");
    // Losers were stopped by the Redis reservation, before any order existed
    // (Postgres' conditional UPDATE is only the backstop).
    let orders: i64 = sqlx::query_scalar("SELECT count(*) FROM orders")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(orders, i64::from(stock));
    assert_eq!(
        inventory::held(&app.state.redis, s.store, item)
            .await
            .unwrap(),
        0
    );
}

#[sqlx::test]
async fn checkout_validation(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = shop(&app).await;
    let milk = product(&app, &s, "Milk", 2_800, 5).await;
    let (token, address) = customer(&app, HOME).await;

    let (status, res) = checkout(&app, &token, s.store, &address, "COD", &key()).await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "empty cart: {res}"
    );

    set_cart(&app, &token, s.store, milk, 1).await;
    // An address outside every store.
    let (_, far) = app
        .request(
            Method::POST,
            "/v1/customer/addresses",
            Some(&token),
            Some(json!({
                "label": "Work", "line1": "Tower B", "city": "Mysuru", "pincode": "570001",
                "location": { "lat": 12.2958, "lng": 76.6394 },
            })),
        )
        .await;
    assert!(far["servingStoreId"].is_null());
    let (status, res) = checkout(
        &app,
        &token,
        s.store,
        far["id"].as_str().unwrap(),
        "COD",
        &key(),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{res}");

    // An address served by a different store than the cart's.
    let (_, other) = app
        .request(
            Method::POST,
            "/v1/admin/stores",
            Some(&s.admin),
            Some(store_body("BLR-OTHER", 12.9352, 77.6245, 2.0)),
        )
        .await;
    let (_, kor) = app
        .request(
            Method::POST,
            "/v1/customer/addresses",
            Some(&token),
            Some(json!({
                "label": "Other", "line1": "5th Block", "city": "Bengaluru", "pincode": "560095",
                "location": { "lat": 12.9352, "lng": 77.6245 },
            })),
        )
        .await;
    assert_eq!(kor["servingStoreId"], other["id"]);
    let (status, res) = checkout(
        &app,
        &token,
        s.store,
        kor["id"].as_str().unwrap(),
        "COD",
        &key(),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{res}");

    // Someone else's address.
    let (_, stranger_address) = customer(&app, HOME).await;
    let (status, _) = checkout(&app, &token, s.store, &stranger_address, "COD", &key()).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Stock drops below the cart quantity before checkout.
    set_cart(&app, &token, s.store, milk, 3).await;
    app.request(
        Method::PUT,
        &format!("/v1/admin/stores/{}/inventory/{milk}", s.store),
        Some(&s.admin),
        Some(json!({ "quantity": 2, "isAvailable": true })),
    )
    .await;
    let (_, cart) = app
        .get(
            &format!("/v1/customer/cart?storeId={}", s.store),
            Some(&token),
        )
        .await;
    assert_eq!(cart["canCheckout"], false);
    assert_eq!(cart["items"][0]["available"], false);
    let (status, res) = checkout(&app, &token, s.store, &address, "COD", &key()).await;
    assert_eq!(status, StatusCode::CONFLICT, "{res}");
    assert_eq!(
        inventory::held(&app.state.redis, s.store, milk)
            .await
            .unwrap(),
        0,
        "nothing left reserved"
    );

    // Addresses are India-only with valid PIN codes.
    let (status, _) = app
        .request(Method::POST, "/v1/customer/addresses", Some(&token), Some(json!({
            "label": "Bad", "line1": "x", "city": "y", "pincode": "012345", "location": { "lat": 12.97, "lng": 77.64 },
        })))
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // Customers only; admins aren't customers.
    let (status, _) = app
        .get(
            &format!("/v1/customer/cart?storeId={}", s.store),
            Some(&s.admin),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn online_payment_confirms_on_signed_webhook(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = shop(&app).await;
    let atta = product(&app, &s, "Atta 5 kg", 26_500, 4).await;
    let (token, address) = customer(&app, HOME).await;
    set_cart(&app, &token, s.store, atta, 1).await;

    let (status, res) = checkout(&app, &token, s.store, &address, "ONLINE", &key()).await;
    assert_eq!(status, StatusCode::CREATED, "{res}");
    assert_eq!(res["order"]["status"], "PLACED");
    assert_eq!(
        res["order"]["bill"]["deliveryFeePaise"], 0,
        "≥ ₹199 ships free"
    );
    assert!(
        res["order"]["deliveryOtp"].is_null(),
        "no OTP before confirmation"
    );
    let rp = &res["razorpay"];
    assert_eq!(rp["amountPaise"], 26_500);
    assert_eq!(rp["currency"], "INR");
    let gw = rp["gatewayOrderId"].as_str().unwrap();
    let order_id = res["order"]["id"].as_str().unwrap();

    assert_eq!(
        pg_stock(&db, s.store, atta).await,
        4,
        "not taken until paid"
    );
    assert_eq!(
        inventory::held(&app.state.redis, s.store, atta)
            .await
            .unwrap(),
        1,
        "but reserved"
    );

    let hook = |body: &[u8], sig: &str, event_id: &str| {
        let body = body.to_vec();
        let sig = sig.to_owned();
        let event_id = event_id.to_owned();
        let app = &app;
        async move {
            app.post_with_headers(
                "/v1/payments/razorpay/webhook",
                None,
                &[
                    ("x-razorpay-signature", &sig),
                    ("x-razorpay-event-id", &event_id),
                ],
                &body,
            )
            .await
        }
    };

    let body = webhook("payment.captured", gw, 26_500);
    let (status, _) = hook(&body, "0badc0de", "evt_1").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "bad signature");
    let wrong_amount = webhook("payment.captured", gw, 100);
    let (status, _) = hook(&wrong_amount, &sign(&wrong_amount), "evt_0").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "amount mismatch");

    let (status, _) = hook(&body, &sign(&body), "evt_1").await;
    assert_eq!(status, StatusCode::OK);
    let (_, order) = app
        .get(&format!("/v1/customer/orders/{order_id}"), Some(&token))
        .await;
    assert_eq!(order["status"], "CONFIRMED");
    assert_eq!(order["paymentStatus"], "PAID");
    assert_eq!(pg_stock(&db, s.store, atta).await, 3);
    assert_eq!(
        inventory::held(&app.state.redis, s.store, atta)
            .await
            .unwrap(),
        0
    );

    // Redelivery of the same event is a no-op.
    let (status, _) = hook(&body, &sign(&body), "evt_1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        pg_stock(&db, s.store, atta).await,
        3,
        "not decremented twice"
    );
}

#[sqlx::test]
async fn failed_payment_and_expired_reservations_release_stock(db: PgPool) {
    let app = TestApp::with_config(db.clone(), |c| {
        c.reservation_ttl = Duration::from_millis(200)
    });
    let s = shop(&app).await;
    let rice = product(&app, &s, "Rice", 16_500, 3).await;

    // payment.failed webhook cancels and releases.
    let (token, address) = customer(&app, HOME).await;
    set_cart(&app, &token, s.store, rice, 2).await;
    let (_, res) = checkout(&app, &token, s.store, &address, "ONLINE", &key()).await;
    let gw = res["razorpay"]["gatewayOrderId"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        inventory::held(&app.state.redis, s.store, rice)
            .await
            .unwrap(),
        2
    );
    let body = webhook("payment.failed", &gw, 16_500 * 2 + 3_000);
    app.post_with_headers(
        "/v1/payments/razorpay/webhook",
        None,
        &[
            ("x-razorpay-signature", &sign(&body)),
            ("x-razorpay-event-id", "evt_fail"),
        ],
        &body,
    )
    .await;
    let (_, order) = app
        .get(
            &format!(
                "/v1/customer/orders/{}",
                res["order"]["id"].as_str().unwrap()
            ),
            Some(&token),
        )
        .await;
    assert_eq!(order["status"], "CANCELLED");
    assert_eq!(order["paymentStatus"], "FAILED");
    assert_eq!(
        inventory::held(&app.state.redis, s.store, rice)
            .await
            .unwrap(),
        0
    );

    // Abandoned online checkout: the sweeper cancels after the TTL.
    let (token2, address2) = customer(&app, HOME).await;
    set_cart(&app, &token2, s.store, rice, 3).await;
    let (_, res) = checkout(&app, &token2, s.store, &address2, "ONLINE", &key()).await;
    assert_eq!(
        inventory::held(&app.state.redis, s.store, rice)
            .await
            .unwrap(),
        3
    );
    // All 3 units are held, so another shopper can't have them meanwhile.
    let (token3, address3) = customer(&app, HOME).await;
    set_cart(&app, &token3, s.store, rice, 1).await;
    let (status, _) = checkout(&app, &token3, s.store, &address3, "COD", &key()).await;
    assert_eq!(status, StatusCode::CONFLICT);

    tokio::time::sleep(Duration::from_millis(300)).await;
    let swept = order_service::sweep_expired_reservations(&app.state)
        .await
        .unwrap();
    assert!(swept >= 1);
    let (_, order) = app
        .get(
            &format!(
                "/v1/customer/orders/{}",
                res["order"]["id"].as_str().unwrap()
            ),
            Some(&token2),
        )
        .await;
    assert_eq!(order["status"], "CANCELLED");
    assert_eq!(order["cancelReason"], "Payment wasn't completed in time");
    assert_eq!(
        inventory::held(&app.state.redis, s.store, rice)
            .await
            .unwrap(),
        0
    );
    assert_eq!(pg_stock(&db, s.store, rice).await, 3, "never taken");

    // Now the stock is free again.
    let (status, _) = checkout(&app, &token3, s.store, &address3, "COD", &key()).await;
    assert_eq!(status, StatusCode::CREATED);
}

#[sqlx::test]
async fn cancellation_and_state_machine(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = shop(&app).await;
    let eggs = product(&app, &s, "Eggs", 6_600, 10).await;
    let (token, address) = customer(&app, HOME).await;

    set_cart(&app, &token, s.store, eggs, 3).await;
    let (_, res) = checkout(&app, &token, s.store, &address, "COD", &key()).await;
    let id = res["order"]["id"].as_str().unwrap().to_owned();
    assert_eq!(pg_stock(&db, s.store, eggs).await, 7);

    // Other customers can't see or cancel it.
    let (stranger, _) = app.signed_in(&random_phone()).await;
    let (status, _) = app
        .get(&format!("/v1/customer/orders/{id}"), Some(&stranger))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, order) = app
        .request(
            Method::POST,
            &format!("/v1/customer/orders/{id}/cancel"),
            Some(&token),
            Some(json!({ "reason": "Ordered by mistake" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{order}");
    assert_eq!(order["status"], "CANCELLED");
    assert_eq!(order["cancelReason"], "Ordered by mistake");
    assert!(order["deliveryOtp"].is_null());
    assert_eq!(pg_stock(&db, s.store, eggs).await, 10, "restocked");
    assert_eq!(
        inventory::stock(&app.state.redis, s.store, eggs)
            .await
            .unwrap(),
        Some(10)
    );

    let (status, _) = app
        .request(
            Method::POST,
            &format!("/v1/customer/orders/{id}/cancel"),
            Some(&token),
            Some(json!({})),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "already cancelled");

    // Once picking starts the customer can't cancel; illegal jumps are 409.
    set_cart(&app, &token, s.store, eggs, 1).await;
    let (_, res) = checkout(&app, &token, s.store, &address, "COD", &key()).await;
    let id = Uuid::parse_str(res["order"]["id"].as_str().unwrap()).unwrap();
    let err = order_service::transition(&app.state, id, OrderStatus::Delivered, None, None).await;
    assert!(
        matches!(err, Err(backend::error::AppError::Conflict(_))),
        "{err:?}"
    );
    order_service::transition(
        &app.state,
        id,
        OrderStatus::Picking,
        None,
        Some("picker started"),
    )
    .await
    .unwrap();
    let (status, body) = app
        .request(
            Method::POST,
            &format!("/v1/customer/orders/{id}/cancel"),
            Some(&token),
            Some(json!({})),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    let (_, order) = app
        .get(&format!("/v1/customer/orders/{id}"), Some(&token))
        .await;
    assert_eq!(order["canCancel"], false);
    let statuses: Vec<&str> = order["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["status"].as_str().unwrap())
        .collect();
    assert_eq!(statuses, ["PLACED", "CONFIRMED", "PICKING"]);
}

#[sqlx::test]
async fn status_changes_are_published_to_redis(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = shop(&app).await;
    let bread = product(&app, &s, "Bread", 5_000, 5).await;
    let (token, address) = customer(&app, HOME).await;
    set_cart(&app, &token, s.store, bread, 1).await;

    let redis_url = app.state.config.redis_url.clone();
    let client = redis::Client::open(redis_url).unwrap();
    let mut pubsub = client.get_async_pubsub().await.unwrap();
    pubsub
        .subscribe(format!("stores:{}:orders", s.store))
        .await
        .unwrap();
    let mut messages = pubsub.into_on_message();

    checkout(&app, &token, s.store, &address, "COD", &key()).await;
    let mut seen = Vec::new();
    for _ in 0..2 {
        let msg = tokio::time::timeout(Duration::from_secs(3), messages.next())
            .await
            .unwrap()
            .unwrap();
        let event: Value = serde_json::from_str(&msg.get_payload::<String>().unwrap()).unwrap();
        seen.push(event["status"].as_str().unwrap().to_owned());
        assert_eq!(event["storeId"], s.store.to_string());
    }
    assert_eq!(seen, ["PLACED", "CONFIRMED"]);
}

// ---------- phase 8: rate limits + reconciliation ----------

#[sqlx::test]
async fn checkout_is_rate_limited_per_user(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = shop(&app).await;
    let (token, address) = customer(&app, HOME).await;
    // Empty cart: each attempt is a cheap 422, but they still count.
    for i in 1..=10 {
        let (status, _) = checkout(&app, &token, s.store, &address, "COD", &key()).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "attempt {i}");
    }
    let (status, body) = checkout(&app, &token, s.store, &address, "COD", &key()).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(error_code(&body), "RATE_LIMITED");

    // Another customer isn't affected.
    let (other, other_address) = customer(&app, HOME).await;
    let (status, _) = checkout(&app, &other, s.store, &other_address, "COD", &key()).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test]
async fn reconciliation_repairs_the_redis_mirror(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = shop(&app).await;
    let milk = product(&app, &s, "Milk", 2_800, 5).await;
    let (token, address) = customer(&app, HOME).await;
    set_cart(&app, &token, s.store, milk, 2).await;
    checkout(&app, &token, s.store, &address, "COD", &key()).await;
    assert_eq!(pg_stock(&db, s.store, milk).await, 3);

    // The mirror drifts (e.g. a Redis write failed after a Postgres commit).
    inventory::set_stock(&app.state.redis, s.store, milk, 99)
        .await
        .unwrap();
    let synced = backend::services::catalog_service::reconcile_stock(&app.state)
        .await
        .unwrap();
    assert!(synced >= 1);
    assert_eq!(
        inventory::stock(&app.state.redis, s.store, milk)
            .await
            .unwrap(),
        Some(3)
    );
}
