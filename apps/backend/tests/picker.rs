// Tests may panic freely; the no-unwrap/expect rule is for production code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use axum::http::{Method, StatusCode};
use common::{
    TestApp, TokenBuilder, admin_token, create_store, error_code, next_json, random_phone,
    store_body,
};
use futures::SinkExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

const HOME: (f64, f64) = (12.9730, 77.6420);

struct Setup {
    store: Uuid,
    admin: String,
    /// (name, id, barcode, bin)
    products: Vec<(String, Uuid, String, String)>,
}

/// A store with three products on different shelves, each with 10 units.
async fn setup(app: &TestApp) -> Setup {
    let admin = admin_token(app).await;
    let store = create_store(app, &admin, "BLR-PICK").await;
    let store = Uuid::parse_str(store.as_str().unwrap()).unwrap();
    let (_, cat) = app
        .request(
            Method::POST,
            "/v1/admin/categories",
            Some(&admin),
            Some(json!({ "name": "Groceries", "isActive": true })),
        )
        .await;
    let mut products = Vec::new();
    for (name, barcode, bin) in [
        ("Milk", "2000000000011", "B-10-1"),
        ("Bread", "2000000000028", "A-02-3"),
        ("Eggs", "2000000000035", "B-2-1"),
    ] {
        let (_, p) = app
            .request(
                Method::POST,
                "/v1/admin/products",
                Some(&admin),
                Some(json!({
                    "categoryId": cat["id"], "name": name, "unitLabel": "1 pc", "barcode": barcode,
                    "mrpPaise": 5_000, "pricePaise": 4_000, "isActive": true,
                })),
            )
            .await;
        let id = Uuid::parse_str(p["id"].as_str().unwrap()).unwrap();
        app.request(
            Method::PUT,
            &format!("/v1/admin/stores/{store}/inventory/{id}"),
            Some(&admin),
            Some(json!({ "quantity": 10, "binLocation": bin, "isAvailable": true })),
        )
        .await;
        products.push((name.to_owned(), id, barcode.to_owned(), bin.to_owned()));
    }
    Setup {
        store,
        admin,
        products,
    }
}

/// A new user promoted to PICKER at `store`.
async fn picker(app: &TestApp, admin: &str, store: Uuid) -> String {
    let (token, me) = app.signed_in(&random_phone()).await;
    let (status, body) = app
        .request(
            Method::PATCH,
            &format!("/v1/admin/users/{}/role", me["id"].as_str().unwrap()),
            Some(admin),
            Some(json!({ "role": "PICKER", "storeId": store })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    token
}

/// A COD order for 2× Milk, 1× Bread, 3× Eggs; returns its id.
async fn order(app: &TestApp, s: &Setup) -> String {
    let (token, _) = app.signed_in(&random_phone()).await;
    let (_, addr) = app
        .request(
            Method::POST,
            "/v1/customer/addresses",
            Some(&token),
            Some(json!({
                "label": "Home", "line1": "1", "city": "Bengaluru", "pincode": "560038",
                "location": { "lat": HOME.0, "lng": HOME.1 },
            })),
        )
        .await;
    for ((_, id, _, _), qty) in s.products.iter().zip([2, 1, 3]) {
        app.request(
            Method::PUT,
            &format!("/v1/customer/cart/items/{id}"),
            Some(&token),
            Some(json!({ "storeId": s.store, "quantity": qty })),
        )
        .await;
    }
    let body =
        json!({ "storeId": s.store, "addressId": addr["id"], "paymentMethod": "COD" }).to_string();
    let (status, res) = app
        .post_with_headers(
            "/v1/customer/checkout",
            Some(&token),
            &[("idempotency-key", &format!("k-{}", Uuid::new_v4().simple()))],
            body.as_bytes(),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{res}");
    res["order"]["id"].as_str().unwrap().to_owned()
}

async fn post(app: &TestApp, token: &str, uri: &str, body: Value) -> (StatusCode, Value) {
    app.request(Method::POST, uri, Some(token), Some(body))
        .await
}

async fn scan(app: &TestApp, token: &str, order: &str, barcode: &str) -> (StatusCode, Value) {
    post(
        app,
        token,
        &format!("/v1/picker/orders/{order}/scan"),
        json!({ "barcode": barcode }),
    )
    .await
}

#[sqlx::test]
async fn full_pick_and_pack(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = setup(&app).await;
    let p1 = picker(&app, &s.admin, s.store).await;
    let order = order(&app, &s).await;

    let (status, queue) = app.get("/v1/picker/orders", Some(&p1)).await;
    assert_eq!(status, StatusCode::OK, "{queue}");
    assert_eq!(queue[0]["id"], order);
    assert_eq!(queue[0]["status"], "CONFIRMED");
    assert_eq!(queue[0]["itemCount"], 6);
    assert_eq!(queue[0]["lineCount"], 3);

    let (status, list) = post(
        &app,
        &p1,
        &format!("/v1/picker/orders/{order}/start"),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{list}");
    assert_eq!(list["status"], "PICKING");
    assert_eq!(list["isMine"], true);
    let bins: Vec<&str> = list["lines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["binLocation"].as_str().unwrap())
        .collect();
    assert_eq!(
        bins,
        ["A-02-3", "B-2-1", "B-10-1"],
        "walk order, natural sort"
    );
    assert_eq!(list["canPack"], false);

    // Scan everything; over-scanning and foreign items are rejected.
    let barcode = |name: &str| s.products.iter().find(|p| p.0 == name).unwrap().2.clone();
    let (status, res) = scan(&app, &p1, &order, &barcode("Bread")).await;
    assert_eq!(status, StatusCode::OK, "{res}");
    assert_eq!(res["lineComplete"], true);
    let (status, res) = scan(&app, &p1, &order, &barcode("Bread")).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error_code(&res), "LINE_COMPLETE");
    let (status, res) = scan(&app, &p1, &order, "8901234567890").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error_code(&res), "SCAN_MISMATCH");
    for _ in 0..2 {
        let (status, res) = scan(&app, &p1, &order, &barcode("Milk")).await;
        assert_eq!(status, StatusCode::OK, "{res}");
    }
    for i in 0..3 {
        let (_, res) = scan(&app, &p1, &order, &barcode("Eggs")).await;
        assert_eq!(res["lineComplete"], i == 2);
    }

    let (status, res) = post(
        &app,
        &p1,
        &format!("/v1/picker/orders/{order}/pack"),
        json!({ "bagCount": 0 }),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "bag count ≥ 1: {res}"
    );
    let (status, packed) = post(
        &app,
        &p1,
        &format!("/v1/picker/orders/{order}/pack"),
        json!({ "bagCount": 2, "stagingSlot": "S-03" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{packed}");
    assert_eq!(packed["status"], "PACKED");
    assert_eq!(packed["bagCount"], 2);
    assert_eq!(packed["stagingSlot"], "S-03");

    // Packed orders stay in the queue, waiting for a rider.
    let (_, queue) = app.get("/v1/picker/orders", Some(&p1)).await;
    assert_eq!(queue[0]["status"], "PACKED");
    assert_eq!(queue[0]["linesDone"], 3);

    let events: Vec<String> = sqlx::query_scalar(
        "SELECT to_status::text FROM order_status_events WHERE order_id = $1::uuid ORDER BY id",
    )
    .bind(&order)
    .fetch_all(&db)
    .await
    .unwrap();
    assert_eq!(events, ["PLACED", "CONFIRMED", "PICKING", "PACKED"]);
}

#[sqlx::test]
async fn shortages_rebill_and_zero_the_shelf(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = setup(&app).await;
    let p1 = picker(&app, &s.admin, s.store).await;
    let order = order(&app, &s).await;
    post(
        &app,
        &p1,
        &format!("/v1/picker/orders/{order}/start"),
        json!({}),
    )
    .await;

    let id = |name: &str| s.products.iter().find(|p| p.0 == name).unwrap().1;
    let set = |name: &str, n: i32| {
        let uri = format!("/v1/picker/orders/{order}/items/{}", id(name));
        let app = &app;
        let p1 = p1.clone();
        async move {
            app.request(
                Method::PUT,
                &uri,
                Some(&p1),
                Some(json!({ "pickedQuantity": n })),
            )
            .await
        }
    };
    set("Milk", 2).await;
    set("Bread", 1).await;
    let (status, _) = set("Eggs", 4).await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "more than ordered"
    );
    let (status, list) = set("Eggs", 1).await; // 2 eggs missing
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list["canPack"], true);

    let (status, packed) = post(
        &app,
        &p1,
        &format!("/v1/picker/orders/{order}/pack"),
        json!({ "bagCount": 1 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{packed}");

    // Customer is billed for 4 found units, not 6 ordered. The delivery fee
    // stays as charged at checkout (free here: ₹240 ordered), even though the
    // picked total fell below the free-delivery threshold.
    let (total, item_total, fee): (i64, i64, i64) = sqlx::query_as(
        "SELECT total_paise, item_total_paise, delivery_fee_paise FROM orders WHERE id = $1::uuid",
    )
    .bind(&order)
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(item_total, 4 * 4_000);
    assert_eq!(fee, 0);
    assert_eq!(total, item_total + fee);
    // The eggs shelf was empty in reality.
    let eggs: i32 = sqlx::query_scalar(
        "SELECT quantity FROM store_inventory WHERE store_id = $1 AND product_id = $2",
    )
    .bind(s.store)
    .bind(id("Eggs"))
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(eggs, 0);
    let note: String = sqlx::query_scalar(
        "SELECT note FROM order_status_events WHERE order_id = $1::uuid AND to_status = 'PACKED'",
    )
    .bind(&order)
    .fetch_one(&db)
    .await
    .unwrap();
    assert!(note.contains("missing 2× Eggs"), "{note}");
}

#[sqlx::test]
async fn pickers_are_isolated(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = setup(&app).await;
    let p1 = picker(&app, &s.admin, s.store).await;
    let p2 = picker(&app, &s.admin, s.store).await;
    let order = order(&app, &s).await;

    // First picker wins; the second can't claim or touch it.
    let (a, _) = post(
        &app,
        &p1,
        &format!("/v1/picker/orders/{order}/start"),
        json!({}),
    )
    .await;
    let (b, res) = post(
        &app,
        &p2,
        &format!("/v1/picker/orders/{order}/start"),
        json!({}),
    )
    .await;
    assert_eq!(a, StatusCode::OK);
    assert_eq!(b, StatusCode::CONFLICT);
    assert_eq!(error_code(&res), "ALREADY_CLAIMED");
    let (status, res) = scan(&app, &p2, &order, &s.products[0].2).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error_code(&res), "NOT_YOUR_ORDER");

    // Release hands it back; now p2 can take it, progress kept.
    scan(&app, &p1, &order, &s.products[0].2).await;
    let (status, res) = post(
        &app,
        &p1,
        &format!("/v1/picker/orders/{order}/release"),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{res}");
    assert_eq!(res["status"], "CONFIRMED");
    let (status, list) = post(
        &app,
        &p2,
        &format!("/v1/picker/orders/{order}/start"),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let milk = list["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["name"] == "Milk")
        .unwrap()
        .clone();
    assert_eq!(milk["pickedQuantity"], 1);

    // A picker at another store doesn't see it at all.
    let (_, other) = app
        .request(
            Method::POST,
            "/v1/admin/stores",
            Some(&s.admin),
            Some(store_body("BLR-FAR", 12.9352, 77.6245, 2.0)),
        )
        .await;
    let p3 = picker(
        &app,
        &s.admin,
        Uuid::parse_str(other["id"].as_str().unwrap()).unwrap(),
    )
    .await;
    let (_, queue) = app.get("/v1/picker/orders", Some(&p3)).await;
    assert_eq!(queue, json!([]));
    let (status, _) = app
        .get(&format!("/v1/picker/orders/{order}"), Some(&p3))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Customers aren't pickers.
    let (customer, _) = app.signed_in(&random_phone()).await;
    let (status, _) = app.get("/v1/picker/orders", Some(&customer)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Once picking starts the customer can no longer cancel.
    let (count,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM orders WHERE id = $1::uuid AND status = 'PICKING'")
            .bind(&order)
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(count, 1);
}

#[sqlx::test]
async fn picker_cancel_restocks_only_what_was_found(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = setup(&app).await;
    let p1 = picker(&app, &s.admin, s.store).await;
    let order = order(&app, &s).await;
    post(
        &app,
        &p1,
        &format!("/v1/picker/orders/{order}/start"),
        json!({}),
    )
    .await;
    let milk = s.products[0].1;
    // Found 1 of 2 milk, then gave up on the order.
    app.request(
        Method::PUT,
        &format!("/v1/picker/orders/{order}/items/{milk}"),
        Some(&p1),
        Some(json!({ "pickedQuantity": 1 })),
    )
    .await;
    let (status, res) = post(
        &app,
        &p1,
        &format!("/v1/picker/orders/{order}/pack"),
        json!({ "bagCount": 1 }),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "not all counted");
    assert_eq!(error_code(&res), "NOT_ALL_COUNTED");

    let (status, res) = post(
        &app,
        &p1,
        &format!("/v1/picker/orders/{order}/cancel"),
        json!({ "reason": "freezer broke" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{res}");
    assert_eq!(res["status"], "CANCELLED");
    let stock = |pid: Uuid| {
        let db = db.clone();
        let store = s.store;
        async move {
            sqlx::query_scalar::<_, i32>(
                "SELECT quantity FROM store_inventory WHERE store_id = $1 AND product_id = $2",
            )
            .bind(store)
            .bind(pid)
            .fetch_one(&db)
            .await
            .unwrap()
        }
    };
    // Milk: 10 - 2 sold + 1 found back on the shelf. Bread/Eggs weren't counted,
    // so the full ordered quantity goes back.
    assert_eq!(stock(milk).await, 9);
    assert_eq!(stock(s.products[1].1).await, 10);
    assert_eq!(stock(s.products[2].1).await, 10);
}

#[sqlx::test]
async fn picker_websocket_streams_store_events(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = setup(&app).await;
    let p1 = picker(&app, &s.admin, s.store).await;
    let addr = app.serve().await;
    let url = format!("ws://{addr}/v1/ws/picker");

    // Unauthenticated: rejected.
    let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
    ws.send(Message::text(
        json!({ "type": "auth", "token": "garbage" }).to_string(),
    ))
    .await
    .unwrap();
    let first = next_json(&mut ws).await;
    assert_eq!(first["type"], "error");
    assert_eq!(first["code"], "UNAUTHORIZED");

    // A customer's valid token isn't a picker's.
    let (customer, _) = app.signed_in(&random_phone()).await;
    let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
    ws.send(Message::text(
        json!({ "type": "auth", "token": customer }).to_string(),
    ))
    .await
    .unwrap();
    assert_eq!(next_json(&mut ws).await["code"], "FORBIDDEN");

    let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
    ws.send(Message::text(
        json!({ "type": "auth", "token": p1 }).to_string(),
    ))
    .await
    .unwrap();
    assert_eq!(next_json(&mut ws).await["type"], "ready");

    // A customer order arrives: PLACED then CONFIRMED, live.
    let order = order(&app, &s).await;
    let mut statuses = Vec::new();
    while statuses.len() < 2 {
        let msg = next_json(&mut ws).await;
        if msg["type"] == "order" {
            assert_eq!(msg["event"]["orderId"], order);
            statuses.push(msg["event"]["status"].as_str().unwrap().to_owned());
        }
    }
    assert_eq!(statuses, ["PLACED", "CONFIRMED"]);

    // Expired tokens are refused.
    let now = chrono::Utc::now().timestamp();
    let expired = TokenBuilder::new().set("exp", json!(now - 120)).sign();
    let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
    ws.send(Message::text(
        json!({ "type": "auth", "token": expired }).to_string(),
    ))
    .await
    .unwrap();
    assert_eq!(next_json(&mut ws).await["code"], "UNAUTHORIZED");
}
