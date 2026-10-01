// Tests may panic freely; the no-unwrap/expect rule is for production code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Online payments against a mock Razorpay API: gateway orders, the checkout
//! callback, refunds on cancellation, and the COD switch.

mod common;

use std::sync::{Arc, Mutex};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{Method, StatusCode},
    routing::post,
};
use backend::{cache::inventory, config::RazorpayConfig};
use common::{
    RAZORPAY_KEY, RAZORPAY_WEBHOOK_SECRET, TestApp, admin_token, create_store, random_phone,
};
use hmac::{Hmac, KeyInit, Mac};
use serde_json::{Value, json};
use sha2::Sha256;
use sqlx::PgPool;
use uuid::Uuid;

const HOME: (f64, f64) = (12.9730, 77.6420);
const KEY_SECRET: &str = "rzp_secret_test";

/// Requests the mock Razorpay API received: (path, JSON body).
type Calls = Arc<Mutex<Vec<(String, Value)>>>;

/// A mock Razorpay API on a random port. `fail` makes every call answer 500.
async fn mock_razorpay(fail: bool) -> (String, Calls) {
    let calls: Calls = Arc::default();
    let app = Router::new()
        .route(
            "/v1/orders",
            post(
                |State((calls, fail)): State<(Calls, bool)>, Json(body): Json<Value>| async move {
                    calls.lock().unwrap().push(("/v1/orders".into(), body));
                    if fail {
                        return Err(StatusCode::INTERNAL_SERVER_ERROR);
                    }
                    Ok(Json(
                        json!({ "id": format!("order_mock{}", &Uuid::new_v4().simple().to_string()[..10]) }),
                    ))
                },
            ),
        )
        .route(
            "/v1/payments/{id}/refund",
            post(
                |State((calls, _)): State<(Calls, bool)>,
                 Path(id): Path<String>,
                 Json(body): Json<Value>| async move {
                    calls
                        .lock()
                        .unwrap()
                        .push((format!("/v1/payments/{id}/refund"), body));
                    Json(json!({ "id": "rfnd_mock1" }))
                },
            ),
        )
        .with_state((calls.clone(), fail));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{addr}"), calls)
}

fn live_app(db: PgPool, api_base: String, cod_enabled: bool) -> TestApp {
    TestApp::with_config(db, |c| {
        c.razorpay = Some(RazorpayConfig {
            key_id: RAZORPAY_KEY.into(),
            key_secret: Some(KEY_SECRET.into()),
            webhook_secret: RAZORPAY_WEBHOOK_SECRET.into(),
            api_base,
        });
        c.cod_enabled = cod_enabled;
    })
}

/// A store with one stocked product and a customer with a cart; returns
/// (customer token, store id, product id, address id).
async fn ready_to_checkout(app: &TestApp, stock: i32) -> (String, String, String, String) {
    let admin = admin_token(app).await;
    let store = create_store(app, &admin, "BLR-PAY").await;
    let store = store.as_str().unwrap().to_owned();
    let (_, cat) = app
        .request(
            Method::POST,
            "/v1/admin/categories",
            Some(&admin),
            Some(json!({ "name": format!("Cat {}", Uuid::new_v4().simple()), "isActive": true })),
        )
        .await;
    let (status, p) = app
        .request(
            Method::POST,
            "/v1/admin/products",
            Some(&admin),
            Some(json!({
                "categoryId": cat["id"], "name": "Basmati rice 1 kg", "unitLabel": "1 kg",
                "slug": format!("p-{}", Uuid::new_v4().simple()),
                "mrpPaise": 30_000, "pricePaise": 25_000, "isActive": true,
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{p}");
    let product = p["id"].as_str().unwrap().to_owned();
    let (status, body) = app
        .request(
            Method::PUT,
            &format!("/v1/admin/stores/{store}/inventory/{product}"),
            Some(&admin),
            Some(json!({ "quantity": stock, "isAvailable": true })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (token, _) = app.signed_in(&random_phone()).await;
    let (status, addr) = app
        .request(
            Method::POST,
            "/v1/customer/addresses",
            Some(&token),
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
            &format!("/v1/customer/cart/items/{product}"),
            Some(&token),
            Some(json!({ "storeId": store, "quantity": 1 })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    (
        token,
        store,
        product,
        addr["id"].as_str().unwrap().to_owned(),
    )
}

async fn checkout(
    app: &TestApp,
    token: &str,
    store: &str,
    address: &str,
    method: &str,
) -> (StatusCode, Value) {
    let body = json!({ "storeId": store, "addressId": address, "paymentMethod": method });
    let key = format!("key-{}", Uuid::new_v4().simple());
    app.post_with_headers(
        "/v1/customer/checkout",
        Some(token),
        &[("idempotency-key", &key)],
        body.to_string().as_bytes(),
    )
    .await
}

fn checkout_signature(order_id: &str, payment_id: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(KEY_SECRET.as_bytes()).unwrap();
    mac.update(format!("{order_id}|{payment_id}").as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

#[sqlx::test]
async fn checkout_callback_confirms_and_cancel_refunds(db: PgPool) {
    let (api, calls) = mock_razorpay(false).await;
    let app = live_app(db.clone(), api, true);
    let (token, store, _, address) = ready_to_checkout(&app, 5).await;

    let (status, res) = checkout(&app, &token, &store, &address, "ONLINE").await;
    assert_eq!(status, StatusCode::CREATED, "{res}");
    let gw = res["razorpay"]["gatewayOrderId"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(gw.starts_with("order_mock"), "real gateway order: {gw}");
    let order_id = res["order"]["id"].as_str().unwrap().to_owned();
    {
        let calls = calls.lock().unwrap();
        let (path, body) = &calls[0];
        assert_eq!(path, "/v1/orders");
        assert_eq!(body["amount"], res["order"]["bill"]["totalPaise"]);
        assert_eq!(body["currency"], "INR");
        assert_eq!(body["notes"]["order_id"], order_id.as_str());
    }

    let verify_uri = format!("/v1/customer/orders/{order_id}/payment");
    let forged = json!({
        "razorpayOrderId": gw, "razorpayPaymentId": "pay_X1",
        "razorpaySignature": checkout_signature(&gw, "pay_OTHER"),
    });
    let (status, _) = app
        .request(Method::POST, &verify_uri, Some(&token), Some(forged))
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "bad signature");

    let good = json!({
        "razorpayOrderId": gw, "razorpayPaymentId": "pay_X1",
        "razorpaySignature": checkout_signature(&gw, "pay_X1"),
    });
    let (status, order) = app
        .request(Method::POST, &verify_uri, Some(&token), Some(good.clone()))
        .await;
    assert_eq!(status, StatusCode::OK, "{order}");
    assert_eq!(order["status"], "CONFIRMED");
    assert_eq!(order["paymentStatus"], "PAID");

    // Someone else can't verify (or even see) this order.
    let (other, _) = app.signed_in(&random_phone()).await;
    let (status, _) = app
        .request(Method::POST, &verify_uri, Some(&other), Some(good.clone()))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    // A repeated callback is harmless.
    let (status, _) = app
        .request(Method::POST, &verify_uri, Some(&token), Some(good))
        .await;
    assert_eq!(status, StatusCode::OK);

    // Cancelling a paid order refunds it through Razorpay.
    let (status, cancelled) = app
        .request(
            Method::POST,
            &format!("/v1/customer/orders/{order_id}/cancel"),
            Some(&token),
            Some(json!({ "reason": "Ordered by mistake" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{cancelled}");
    assert_eq!(cancelled["status"], "CANCELLED");
    assert_eq!(cancelled["paymentStatus"], "REFUNDED");
    let refund = calls
        .lock()
        .unwrap()
        .iter()
        .find(|(p, _)| p == "/v1/payments/pay_X1/refund")
        .cloned()
        .expect("refund requested");
    assert_eq!(refund.1["amount"], cancelled["bill"]["totalPaise"]);
    let refund_id: Option<String> =
        sqlx::query_scalar("SELECT refund_id FROM order_payments WHERE order_id = $1")
            .bind(Uuid::parse_str(&order_id).unwrap())
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(refund_id.as_deref(), Some("rfnd_mock1"));
}

#[sqlx::test]
async fn gateway_failure_releases_the_reservation(db: PgPool) {
    let (api, _) = mock_razorpay(true).await;
    let app = live_app(db, api, true);
    let (token, store, product, address) = ready_to_checkout(&app, 5).await;

    let (status, body) = checkout(&app, &token, &store, &address, "ONLINE").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    let store = Uuid::parse_str(&store).unwrap();
    let product = Uuid::parse_str(&product).unwrap();
    assert_eq!(
        inventory::held(&app.state.redis, store, product)
            .await
            .unwrap(),
        0,
        "nothing stays reserved"
    );
}

#[sqlx::test]
async fn online_only_launch_turns_cod_off(db: PgPool) {
    let (api, _) = mock_razorpay(false).await;
    let app = live_app(db, api, false);
    let (token, store, _, address) = ready_to_checkout(&app, 5).await;

    let (status, cart) = app
        .get(&format!("/v1/customer/cart?storeId={store}"), Some(&token))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(cart["paymentMethods"], json!(["ONLINE"]));

    let (status, body) = checkout(&app, &token, &store, &address, "COD").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    let (status, body) = checkout(&app, &token, &store, &address, "ONLINE").await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
}
