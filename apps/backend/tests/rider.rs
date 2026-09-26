// Tests may panic freely; the no-unwrap/expect rule is for production code.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cloned_ref_to_slice_refs
)]

mod common;

use std::time::Duration;

use axum::http::{Method, StatusCode};
use backend::{
    models::order::OrderStatus,
    services::{dispatch_service, order_service},
};
use common::{TestApp, admin_token, create_store, error_code, next_json, next_of, random_phone};
use futures::SinkExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

// The test store sits at Indiranagar (12.9719, 77.6412) with a 2 km area.
const STORE: (f64, f64) = (12.9719, 77.6412);
/// Customer ~900 m north of the store.
const DROP: (f64, f64) = (12.9800, 77.6412);

struct World {
    store: Uuid,
    admin: String,
    /// (id, barcode)
    products: Vec<(Uuid, String)>,
}

async fn world(app: &TestApp) -> World {
    let admin = admin_token(app).await;
    let store = create_store(app, &admin, "BLR-RIDE").await;
    let store = Uuid::parse_str(store.as_str().unwrap()).unwrap();
    let (_, cat) = app
        .request(
            Method::POST,
            "/v1/admin/categories",
            Some(&admin),
            Some(json!({ "name": "Staples", "isActive": true })),
        )
        .await;
    let mut products = Vec::new();
    for (i, name) in ["Atta", "Dal"].iter().enumerate() {
        let barcode = format!("20000000001{i}2");
        let (_, p) = app
            .request(
                Method::POST,
                "/v1/admin/products",
                Some(&admin),
                Some(json!({
                    "categoryId": cat["id"], "name": name, "unitLabel": "1 kg", "barcode": barcode,
                    "mrpPaise": 12_000, "pricePaise": 10_000, "isActive": true,
                })),
            )
            .await;
        let id = Uuid::parse_str(p["id"].as_str().unwrap()).unwrap();
        app.request(
            Method::PUT,
            &format!("/v1/admin/stores/{store}/inventory/{id}"),
            Some(&admin),
            Some(
                json!({ "quantity": 50, "binLocation": format!("A-0{i}-1"), "isAvailable": true }),
            ),
        )
        .await;
        products.push((id, barcode));
    }
    World {
        store,
        admin,
        products,
    }
}

async fn staff(app: &TestApp, w: &World, role: &str) -> String {
    let (token, me) = app.signed_in(&random_phone()).await;
    let (status, body) = app
        .request(
            Method::PATCH,
            &format!("/v1/admin/users/{}/role", me["id"].as_str().unwrap()),
            Some(&w.admin),
            Some(json!({ "role": role, "storeId": w.store })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    token
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

async fn locate(app: &TestApp, token: &str, at: (f64, f64)) -> StatusCode {
    locate_at(app, token, at, now_ms()).await
}

async fn locate_at(app: &TestApp, token: &str, at: (f64, f64), recorded_at: i64) -> StatusCode {
    app.request(
        Method::POST,
        "/v1/rider/location",
        Some(token),
        Some(json!({ "points": [{ "lat": at.0, "lng": at.1, "recordedAt": recorded_at }] })),
    )
    .await
    .0
}

/// A rider online at `at`.
async fn rider(app: &TestApp, w: &World, at: (f64, f64)) -> String {
    let token = staff(app, w, "RIDER").await;
    let (status, me) = app
        .request(
            Method::PUT,
            "/v1/rider/status",
            Some(&token),
            Some(json!({ "online": true })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{me}");
    assert_eq!(locate(app, &token, at).await, StatusCode::NO_CONTENT);
    token
}

/// A customer (at DROP) places a COD order; a picker picks and packs it in 2 bags.
/// Returns (order id, customer token). Packing triggers dispatch.
async fn packed_order(app: &TestApp, w: &World, picker: &str, short: bool) -> (String, String) {
    let (token, _) = app.signed_in(&random_phone()).await;
    let (_, addr) = app
        .request(
            Method::POST,
            "/v1/customer/addresses",
            Some(&token),
            Some(json!({
                "label": "Home", "line1": "12th Main", "city": "Bengaluru", "pincode": "560038",
                "location": { "lat": DROP.0, "lng": DROP.1 },
            })),
        )
        .await;
    for (id, _) in &w.products {
        app.request(
            Method::PUT,
            &format!("/v1/customer/cart/items/{id}"),
            Some(&token),
            Some(json!({ "storeId": w.store, "quantity": 1 })),
        )
        .await;
    }
    let body =
        json!({ "storeId": w.store, "addressId": addr["id"], "paymentMethod": "COD" }).to_string();
    let (status, res) = app
        .post_with_headers(
            "/v1/customer/checkout",
            Some(&token),
            &[("idempotency-key", &format!("k-{}", Uuid::new_v4().simple()))],
            body.as_bytes(),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{res}");
    let order = res["order"]["id"].as_str().unwrap().to_owned();

    app.request(
        Method::POST,
        &format!("/v1/picker/orders/{order}/start"),
        Some(picker),
        Some(json!({})),
    )
    .await;
    for (i, (id, barcode)) in w.products.iter().enumerate() {
        if short && i == 1 {
            app.request(
                Method::PUT,
                &format!("/v1/picker/orders/{order}/items/{id}"),
                Some(picker),
                Some(json!({ "pickedQuantity": 0 })),
            )
            .await;
        } else {
            app.request(
                Method::POST,
                &format!("/v1/picker/orders/{order}/scan"),
                Some(picker),
                Some(json!({ "barcode": barcode })),
            )
            .await;
        }
    }
    let (status, res) = app
        .request(
            Method::POST,
            &format!("/v1/picker/orders/{order}/pack"),
            Some(picker),
            Some(json!({ "bagCount": 2, "stagingSlot": "S-02" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{res}");
    (order, token)
}

async fn offers(app: &TestApp, rider: &str) -> Vec<String> {
    let (status, body) = app.get("/v1/rider/offers", Some(rider)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body.as_array()
        .unwrap()
        .iter()
        .map(|o| o["orderId"].as_str().unwrap().to_owned())
        .collect()
}

async fn post(app: &TestApp, token: &str, uri: &str, body: Value) -> (StatusCode, Value) {
    app.request(Method::POST, uri, Some(token), Some(body))
        .await
}

async fn order_status(db: &PgPool, order: &str) -> String {
    sqlx::query_scalar("SELECT status::text FROM orders WHERE id = $1::uuid")
        .bind(order)
        .fetch_one(db)
        .await
        .unwrap()
}

#[sqlx::test]
async fn nearest_available_riders_are_offered_and_first_accept_wins(db: PgPool) {
    let app = TestApp::new(db.clone());
    let w = world(&app).await;
    let picker = staff(&app, &w, "PICKER").await;
    let near = rider(&app, &w, (12.9725, 77.6412)).await; // ~70 m
    let mid = rider(&app, &w, (12.9800, 77.6412)).await; // ~900 m
    let far = rider(&app, &w, (13.0300, 77.6412)).await; // ~6.5 km, outside the 5 km radius
    let stale = staff(&app, &w, "RIDER").await; // online, but last fix 5 minutes old
    app.request(
        Method::PUT,
        "/v1/rider/status",
        Some(&stale),
        Some(json!({ "online": true })),
    )
    .await;
    locate_at(&app, &stale, (12.9720, 77.6412), now_ms() - 5 * 60_000).await;
    let offline = rider(&app, &w, (12.9721, 77.6412)).await;
    app.request(
        Method::PUT,
        "/v1/rider/status",
        Some(&offline),
        Some(json!({ "online": false })),
    )
    .await;

    let (order, customer) = packed_order(&app, &w, &picker, false).await;

    assert_eq!(offers(&app, &near).await, [order.clone()]);
    assert_eq!(offers(&app, &mid).await, [order.clone()]);
    assert!(offers(&app, &far).await.is_empty(), "outside radius");
    assert!(offers(&app, &stale).await.is_empty(), "stale location");
    assert!(offers(&app, &offline).await.is_empty(), "offline");
    let (_, offer_list) = app.get("/v1/rider/offers", Some(&near)).await;
    let offer = &offer_list[0];
    assert_eq!(
        offer["collectPaise"], 20_000,
        "COD total (free delivery ≥ ₹199)"
    );
    assert_eq!(offer["bagCount"], 2);
    assert!(offer["toStoreM"].as_f64().unwrap() < 100.0);
    assert!(offer["dropArea"].as_str().unwrap().contains("560038"));

    // Both race; exactly one wins.
    let accept_uri = format!("/v1/rider/offers/{order}/accept");
    let (a, b) = tokio::join!(
        post(&app, &mid, &accept_uri, json!({})),
        post(&app, &near, &accept_uri, json!({})),
    );
    let wins = [&a, &b]
        .iter()
        .filter(|(s, _)| *s == StatusCode::OK)
        .count();
    assert_eq!(wins, 1, "{a:?} {b:?}");
    let loser = if a.0 == StatusCode::OK { &b } else { &a };
    assert_eq!(loser.0, StatusCode::CONFLICT);
    assert_eq!(error_code(&loser.1), "OFFER_TAKEN");
    let winner_body = if a.0 == StatusCode::OK { &a.1 } else { &b.1 };
    assert_eq!(winner_body["status"], "RIDER_ASSIGNED");
    assert_eq!(winner_body["stagingSlot"], "S-02");
    assert_eq!(order_status(&db, &order).await, "RIDER_ASSIGNED");
    assert!(offers(&app, &near).await.is_empty() && offers(&app, &mid).await.is_empty());

    // The customer sees who's coming.
    let (_, detail) = app
        .get(&format!("/v1/customer/orders/{order}"), Some(&customer))
        .await;
    assert!(
        detail["rider"]["phone"]
            .as_str()
            .unwrap()
            .starts_with("+91")
    );
}

#[sqlx::test]
async fn pickup_depart_and_deliver_with_otp_and_cash(db: PgPool) {
    let app = TestApp::new(db.clone());
    let w = world(&app).await;
    let picker = staff(&app, &w, "PICKER").await;
    let r = rider(&app, &w, STORE).await;
    let (order, customer) = packed_order(&app, &w, &picker, false).await;
    let (status, _) = post(
        &app,
        &r,
        &format!("/v1/rider/offers/{order}/accept"),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Delivering before pickup is refused; so is going offline.
    let (status, _) = post(
        &app,
        &r,
        &format!("/v1/rider/deliveries/{order}/deliver"),
        json!({ "otp": "0000" }),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (status, body) = app
        .request(
            Method::PUT,
            "/v1/rider/status",
            Some(&r),
            Some(json!({ "online": false })),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error_code(&body), "ON_DELIVERY");

    let (status, body) = post(
        &app,
        &r,
        &format!("/v1/rider/deliveries/{order}/pickup"),
        json!({ "bagCount": 1 }),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error_code(&body), "BAG_MISMATCH");
    let (status, active) = post(
        &app,
        &r,
        &format!("/v1/rider/deliveries/{order}/pickup"),
        json!({ "bagCount": 2 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{active}");
    assert_eq!(active["status"], "PICKED_UP");
    assert_eq!(active["collectPaise"], 20_000);
    assert_eq!(active["drop"]["pincode"], "560038");

    let (_, detail) = app
        .get(&format!("/v1/customer/orders/{order}"), Some(&customer))
        .await;
    let otp = detail["deliveryOtp"].as_str().unwrap().to_owned();
    let wrong = if otp == "1111" { "2222" } else { "1111" };

    // Still at the store: too far from the customer.
    let (status, body) = post(
        &app,
        &r,
        &format!("/v1/rider/deliveries/{order}/deliver"),
        json!({ "otp": otp, "codCollectedPaise": 20_000 }),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error_code(&body), "TOO_FAR");

    // Riding off: leaving the store area flips it to OUT_FOR_DELIVERY.
    locate(&app, &r, (12.9760, 77.6412)).await;
    assert_eq!(order_status(&db, &order).await, "OUT_FOR_DELIVERY");
    locate(&app, &r, (12.9798, 77.6413)).await;

    let (status, body) = post(
        &app,
        &r,
        &format!("/v1/rider/deliveries/{order}/deliver"),
        json!({ "otp": wrong, "codCollectedPaise": 20_000 }),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error_code(&body), "WRONG_OTP");
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("4 attempt")
    );
    let (status, body) = post(
        &app,
        &r,
        &format!("/v1/rider/deliveries/{order}/deliver"),
        json!({ "otp": otp, "codCollectedPaise": 15_000 }),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error_code(&body), "COD_AMOUNT_MISMATCH");

    let (status, done) = post(
        &app,
        &r,
        &format!("/v1/rider/deliveries/{order}/deliver"),
        json!({ "otp": otp, "codCollectedPaise": 20_000 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{done}");
    assert_eq!(done["status"], "DELIVERED");
    let (_, detail) = app
        .get(&format!("/v1/customer/orders/{order}"), Some(&customer))
        .await;
    assert_eq!(detail["status"], "DELIVERED");
    assert_eq!(detail["paymentStatus"], "PAID");
    assert!(detail["deliveryOtp"].is_null());
    assert!(detail["rider"].is_null());

    let (status, _) = app.get("/v1/rider/delivery", Some(&r)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, me) = app.get("/v1/rider/me", Some(&r)).await;
    assert_eq!(me["deliveredToday"], 1);
    let (_, history) = app.get("/v1/rider/deliveries", Some(&r)).await;
    assert_eq!(history["items"][0]["codCollectedPaise"], 20_000);

    let events: Vec<String> = sqlx::query_scalar(
        "SELECT to_status::text FROM order_status_events WHERE order_id = $1::uuid ORDER BY id",
    )
    .bind(&order)
    .fetch_all(&db)
    .await
    .unwrap();
    assert_eq!(
        events,
        [
            "PLACED",
            "CONFIRMED",
            "PICKING",
            "PACKED",
            "RIDER_ASSIGNED",
            "PICKED_UP",
            "OUT_FOR_DELIVERY",
            "DELIVERED"
        ]
    );
}

#[sqlx::test]
async fn otp_locks_after_five_wrong_tries(db: PgPool) {
    let app = TestApp::new(db.clone());
    let w = world(&app).await;
    let picker = staff(&app, &w, "PICKER").await;
    let r = rider(&app, &w, STORE).await;
    let (order, customer) = packed_order(&app, &w, &picker, false).await;
    post(
        &app,
        &r,
        &format!("/v1/rider/offers/{order}/accept"),
        json!({}),
    )
    .await;
    post(
        &app,
        &r,
        &format!("/v1/rider/deliveries/{order}/pickup"),
        json!({ "bagCount": 2 }),
    )
    .await;
    locate(&app, &r, DROP).await;
    let (_, detail) = app
        .get(&format!("/v1/customer/orders/{order}"), Some(&customer))
        .await;
    let otp = detail["deliveryOtp"].as_str().unwrap().to_owned();
    let wrong = if otp == "9999" { "8888" } else { "9999" };

    for i in 1..=5 {
        let (status, body) = post(
            &app,
            &r,
            &format!("/v1/rider/deliveries/{order}/deliver"),
            json!({ "otp": wrong, "codCollectedPaise": 20_000 }),
        )
        .await;
        let expected = if i < 5 {
            StatusCode::UNPROCESSABLE_ENTITY
        } else {
            StatusCode::LOCKED
        };
        assert_eq!(status, expected, "attempt {i}: {body}");
    }
    let (status, body) = post(
        &app,
        &r,
        &format!("/v1/rider/deliveries/{order}/deliver"),
        json!({ "otp": otp, "codCollectedPaise": 20_000 }),
    )
    .await;
    assert_eq!(status, StatusCode::LOCKED, "even the right OTP: {body}");
    assert_eq!(error_code(&body), "OTP_LOCKED");
}

#[sqlx::test]
async fn declines_and_expiry_move_the_offer_on(db: PgPool) {
    let app = TestApp::with_config(db.clone(), |c| {
        c.dispatch.wave_size = 1;
        c.dispatch.offer_ttl = Duration::from_millis(300);
    });
    let w = world(&app).await;
    let picker = staff(&app, &w, "PICKER").await;
    let a = rider(&app, &w, (12.9722, 77.6412)).await; // nearest
    let b = rider(&app, &w, (12.9760, 77.6412)).await;
    let (order, _) = packed_order(&app, &w, &picker, false).await;

    assert_eq!(
        offers(&app, &a).await,
        [order.clone()],
        "one at a time, nearest first"
    );
    assert!(offers(&app, &b).await.is_empty());

    // A declines: B gets it straight away.
    let (status, _) = post(
        &app,
        &a,
        &format!("/v1/rider/offers/{order}/decline"),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(offers(&app, &b).await, [order.clone()]);

    // B lets it expire; accepting late fails.
    tokio::time::sleep(Duration::from_millis(400)).await;
    let (status, body) = post(
        &app,
        &b,
        &format!("/v1/rider/offers/{order}/accept"),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::GONE, "{body}");
    assert_eq!(error_code(&body), "OFFER_EXPIRED");

    // Both tried: the dispatcher starts over.
    let order_id = Uuid::parse_str(&order).unwrap();
    assert_eq!(
        dispatch_service::dispatch(&app.state, order_id)
            .await
            .unwrap(),
        0,
        "list resets first"
    );
    assert_eq!(dispatch_service::tick(&app.state).await.unwrap(), 1);
    assert_eq!(offers(&app, &a).await, [order.clone()]);
    let (status, _) = post(
        &app,
        &a,
        &format!("/v1/rider/offers/{order}/accept"),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test]
async fn busy_riders_are_skipped_and_dropped_jobs_are_redispatched(db: PgPool) {
    let app = TestApp::new(db.clone());
    let w = world(&app).await;
    let picker = staff(&app, &w, "PICKER").await;
    let a = rider(&app, &w, (12.9722, 77.6412)).await;
    let (first, _) = packed_order(&app, &w, &picker, false).await;
    post(
        &app,
        &a,
        &format!("/v1/rider/offers/{first}/accept"),
        json!({}),
    )
    .await;

    // A is busy: a second order isn't offered to them.
    let (second, _) = packed_order(&app, &w, &picker, false).await;
    assert!(offers(&app, &a).await.is_empty());
    let b = rider(&app, &w, (12.9760, 77.6412)).await;
    dispatch_service::tick(&app.state).await.unwrap();
    assert_eq!(offers(&app, &b).await, [second.clone()]);

    // A drops the first order before pickup: back to PACKED, offered elsewhere.
    let (status, _) = post(
        &app,
        &a,
        &format!("/v1/rider/deliveries/{first}/unassign"),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(order_status(&db, &first).await, "PACKED");
    let b_offers = offers(&app, &b).await;
    assert!(b_offers.contains(&first), "{b_offers:?}");
    let (status, _) = post(
        &app,
        &a,
        &format!("/v1/rider/deliveries/{first}/pickup"),
        json!({ "bagCount": 2 }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "no longer theirs");

    // Customers and storeless riders can't use the rider API.
    let (customer, _) = app.signed_in(&random_phone()).await;
    let (status, _) = app.get("/v1/rider/me", Some(&customer)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn shortage_ends_partially_fulfilled_and_cancel_frees_the_rider(db: PgPool) {
    let app = TestApp::new(db.clone());
    let w = world(&app).await;
    let picker = staff(&app, &w, "PICKER").await;
    let r = rider(&app, &w, STORE).await;

    let (order, customer) = packed_order(&app, &w, &picker, true).await;
    post(
        &app,
        &r,
        &format!("/v1/rider/offers/{order}/accept"),
        json!({}),
    )
    .await;
    let (_, active) = post(
        &app,
        &r,
        &format!("/v1/rider/deliveries/{order}/pickup"),
        json!({ "bagCount": 2 }),
    )
    .await;
    assert_eq!(
        active["collectPaise"], 10_000,
        "1 of 2 items found; delivery stays free as charged at checkout (₹200 ordered)"
    );
    locate(&app, &r, DROP).await;
    let (_, detail) = app
        .get(&format!("/v1/customer/orders/{order}"), Some(&customer))
        .await;
    let (status, done) = post(
        &app,
        &r,
        &format!("/v1/rider/deliveries/{order}/deliver"),
        json!({ "otp": detail["deliveryOtp"], "codCollectedPaise": 10_000 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{done}");
    assert_eq!(done["status"], "PARTIALLY_FULFILLED");

    // A store-side cancel mid-assignment frees the rider.
    let (order2, _) = packed_order(&app, &w, &picker, false).await;
    locate(&app, &r, STORE).await;
    post(
        &app,
        &r,
        &format!("/v1/rider/offers/{order2}/accept"),
        json!({}),
    )
    .await;
    let id = Uuid::parse_str(&order2).unwrap();
    order_service::cancel(
        &app.state,
        id,
        None,
        "store closed",
        &[OrderStatus::RiderAssigned],
    )
    .await
    .unwrap();
    let (status, _) = app.get("/v1/rider/delivery", Some(&r)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = app
        .request(
            Method::PUT,
            "/v1/rider/status",
            Some(&r),
            Some(json!({ "online": false })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "free to go offline");
}

#[sqlx::test]
async fn rider_websocket_gets_offers_and_revocations(db: PgPool) {
    let app = TestApp::new(db.clone());
    let w = world(&app).await;
    let picker = staff(&app, &w, "PICKER").await;
    let a = rider(&app, &w, (12.9722, 77.6412)).await;
    let b = rider(&app, &w, (12.9760, 77.6412)).await;
    let addr = app.serve().await;

    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/v1/ws/rider"))
        .await
        .unwrap();
    ws.send(Message::text(
        json!({ "type": "auth", "token": a }).to_string(),
    ))
    .await
    .unwrap();
    assert_eq!(next_json(&mut ws).await["type"], "ready");

    let (order, _) = packed_order(&app, &w, &picker, false).await;
    let offer = next_of(&mut ws, "offer").await;
    assert_eq!(offer["offer"]["orderId"], order);
    assert_eq!(offer["offer"]["bagCount"], 2);

    let (status, _) = post(
        &app,
        &b,
        &format!("/v1/rider/offers/{order}/accept"),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let revoked = next_of(&mut ws, "offerRevoked").await;
    assert_eq!(revoked["orderId"], order);

    // Pickers' tokens don't open the rider feed.
    let (mut ws2, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/v1/ws/rider"))
        .await
        .unwrap();
    ws2.send(Message::text(
        json!({ "type": "auth", "token": picker }).to_string(),
    ))
    .await
    .unwrap();
    assert_eq!(next_json(&mut ws2).await["code"], "FORBIDDEN");
}

// ---------- phase 7: live tracking + admin board ----------

async fn ws_auth(
    addr: std::net::SocketAddr,
    path: &str,
    token: &str,
) -> tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>> {
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}{path}"))
        .await
        .unwrap();
    ws.send(Message::text(
        json!({ "type": "auth", "token": token }).to_string(),
    ))
    .await
    .unwrap();
    ws
}

#[sqlx::test]
async fn customer_tracks_their_order_live(db: PgPool) {
    let app = TestApp::new(db.clone());
    let w = world(&app).await;
    let picker = staff(&app, &w, "PICKER").await;
    let r = rider(&app, &w, STORE).await;
    let (order, customer) = packed_order(&app, &w, &picker, false).await;
    let addr = app.serve().await;

    // Someone else can't watch it.
    let (stranger, _) = app.signed_in(&random_phone()).await;
    let mut ws = ws_auth(addr, &format!("/v1/ws/orders/{order}"), &stranger).await;
    assert_eq!(next_json(&mut ws).await["code"], "FORBIDDEN");

    let mut ws = ws_auth(addr, &format!("/v1/ws/orders/{order}"), &customer).await;
    assert_eq!(next_json(&mut ws).await["type"], "ready");

    post(
        &app,
        &r,
        &format!("/v1/rider/offers/{order}/accept"),
        json!({}),
    )
    .await;
    assert_eq!(
        next_of(&mut ws, "order").await["event"]["status"],
        "RIDER_ASSIGNED"
    );
    post(
        &app,
        &r,
        &format!("/v1/rider/deliveries/{order}/pickup"),
        json!({ "bagCount": 2 }),
    )
    .await;
    assert_eq!(
        next_of(&mut ws, "order").await["event"]["status"],
        "PICKED_UP"
    );

    locate(&app, &r, (12.9760, 77.6412)).await;
    let loc = next_of(&mut ws, "riderLocation").await;
    assert_eq!(loc["location"]["orderId"], order);
    assert!((loc["location"]["lat"].as_f64().unwrap() - 12.9760).abs() < 1e-6);

    // The detail endpoint carries the last position for the first render.
    let (_, detail) = app
        .get(&format!("/v1/customer/orders/{order}"), Some(&customer))
        .await;
    assert!((detail["rider"]["location"]["lat"].as_f64().unwrap() - 12.9760).abs() < 1e-6);
}

#[sqlx::test]
async fn admin_board_and_metrics(db: PgPool) {
    let app = TestApp::new(db.clone());
    let w = world(&app).await;
    let picker = staff(&app, &w, "PICKER").await;
    let r = rider(&app, &w, STORE).await;
    let addr = app.serve().await;
    let mut ws = ws_auth(addr, "/v1/ws/admin", &w.admin).await;
    assert_eq!(next_json(&mut ws).await["type"], "ready");

    let (order, customer) = packed_order(&app, &w, &picker, false).await;
    // The admin channel is global (other tests share this Redis): find ours.
    loop {
        let msg = next_of(&mut ws, "order").await;
        if msg["event"]["orderId"] == order.as_str() {
            break;
        }
    }

    let (status, board) = app
        .get(
            &format!("/v1/admin/orders?storeId={}", w.store),
            Some(&w.admin),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{board}");
    assert_eq!(board[0]["id"], order);
    assert_eq!(board[0]["status"], "PACKED");
    assert_eq!(board[0]["storeCode"], "BLR-RIDE");

    post(
        &app,
        &r,
        &format!("/v1/rider/offers/{order}/accept"),
        json!({}),
    )
    .await;
    let (_, board) = app.get("/v1/admin/orders", Some(&w.admin)).await;
    assert!(board[0]["riderPhone"].as_str().unwrap().starts_with("+91"));

    post(
        &app,
        &r,
        &format!("/v1/rider/deliveries/{order}/pickup"),
        json!({ "bagCount": 2 }),
    )
    .await;
    locate(&app, &r, DROP).await;
    let (_, detail) = app
        .get(&format!("/v1/customer/orders/{order}"), Some(&customer))
        .await;
    post(
        &app,
        &r,
        &format!("/v1/rider/deliveries/{order}/deliver"),
        json!({ "otp": detail["deliveryOtp"], "codCollectedPaise": 20_000 }),
    )
    .await;

    let (_, board) = app
        .get(
            &format!("/v1/admin/orders?storeId={}", w.store),
            Some(&w.admin),
        )
        .await;
    assert_eq!(board, json!([]), "delivered orders leave the board");
    let (status, m) = app
        .get(
            &format!("/v1/admin/metrics?storeId={}", w.store),
            Some(&w.admin),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{m}");
    assert_eq!(m["ordersToday"], 1);
    assert_eq!(m["deliveredToday"], 1);
    assert_eq!(m["gmvTodayPaise"], 20_000);
    assert_eq!(m["ridersOnline"], 1);
    assert!(m["avgDeliveryMinutes"].as_f64().unwrap() >= 0.0);

    let (status, _) = app.get("/v1/admin/metrics", Some(&customer)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
