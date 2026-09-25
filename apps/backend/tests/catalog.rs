// Tests may panic freely; the no-unwrap/expect rule is for production code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use axum::http::{Method, StatusCode};
use common::{TestApp, admin_token, create_store, error_code, hexagon, random_phone, store_body};
use serde_json::{Value, json};
use sqlx::PgPool;

// Points in Bengaluru relative to the test store at Indiranagar (12.9719, 77.6412).
const INSIDE: (f64, f64) = (12.9730, 77.6420);
const OUTSIDE: (f64, f64) = (13.0500, 77.5000);

async fn create(app: &TestApp, token: &str, uri: &str, body: Value) -> Value {
    let (status, res) = app
        .request(Method::POST, uri, Some(token), Some(body))
        .await;
    assert_eq!(status, StatusCode::CREATED, "POST {uri}: {res}");
    res
}

async fn category(app: &TestApp, token: &str, name: &str) -> Value {
    create(
        app,
        token,
        "/v1/admin/categories",
        json!({ "name": name, "isActive": true }),
    )
    .await
}

fn product_body(category_id: &Value, name: &str, mrp: i64, price: i64) -> Value {
    json!({
        "categoryId": category_id, "name": name, "brand": "Amul", "unitLabel": "500 ml",
        "mrpPaise": mrp, "pricePaise": price, "isActive": true,
    })
}

async fn stock(
    app: &TestApp,
    token: &str,
    store: &Value,
    product: &Value,
    body: Value,
) -> (StatusCode, Value) {
    let uri = format!(
        "/v1/admin/stores/{}/inventory/{}",
        store.as_str().unwrap(),
        product.as_str().unwrap()
    );
    app.request(Method::PUT, &uri, Some(token), Some(body))
        .await
}

#[sqlx::test]
async fn store_crud_and_validation(db: PgPool) {
    let app = TestApp::new(db);
    let token = admin_token(&app).await;

    let id = create_store(&app, &token, "BLR-IND-01").await;
    let (status, store) = app
        .get(
            &format!("/v1/admin/stores/{}", id.as_str().unwrap()),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(store["code"], "BLR-IND-01");
    let ring = store["serviceArea"]["coordinates"][0].as_array().unwrap();
    assert_eq!(ring.len(), 7, "hexagon round-trips through PostGIS");
    let area = store["areaSqKm"].as_f64().unwrap();
    assert!(
        (9.0..12.0).contains(&area),
        "2 km hexagon ≈ 10.4 km², got {area}"
    );

    // Duplicate code -> 409.
    let (status, body) = app
        .request(
            Method::POST,
            "/v1/admin/stores",
            Some(&token),
            Some(store_body("BLR-IND-01", 12.97, 77.64, 2.0)),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");

    let mut bowtie = store_body("BLR-BAD-01", 12.97, 77.64, 2.0);
    bowtie["serviceArea"] = json!({ "type": "Polygon", "coordinates": [[
        [77.60, 12.90], [77.70, 13.00], [77.70, 12.90], [77.60, 13.00], [77.60, 12.90]
    ]] });
    let mut london = store_body("LON-01", 51.5, -0.12, 2.0);
    london["location"] = json!({ "lat": 12.97, "lng": 77.64 });
    let cases = [
        ("self-intersecting", bowtie),
        ("outside India", london),
        ("too large", store_body("BLR-BIG-01", 12.97, 77.64, 20.0)),
        ("too small", store_body("BLR-TINY-01", 12.97, 77.64, 0.05)),
        ("bad code", store_body("blr ind", 12.97, 77.64, 2.0)),
    ];
    for (name, body) in cases {
        let (status, res) = app
            .request(Method::POST, "/v1/admin/stores", Some(&token), Some(body))
            .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{name}: {res}");
    }

    // PUT replaces the area.
    let mut moved = store_body("BLR-IND-01", 12.9719, 77.6412, 1.0);
    moved["name"] = json!("Renamed");
    let (status, updated) = app
        .request(
            Method::PUT,
            &format!("/v1/admin/stores/{}", id.as_str().unwrap()),
            Some(&token),
            Some(moved),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(updated["name"], "Renamed");
    assert!(updated["areaSqKm"].as_f64().unwrap() < 3.0);
}

#[sqlx::test]
async fn serviceability_picks_the_covering_store(db: PgPool) {
    let app = TestApp::new(db);
    let token = admin_token(&app).await;
    let near = create_store(&app, &token, "BLR-A").await;
    // A second, overlapping store 1.5 km north: the nearer one must win.
    create(
        &app,
        &token,
        "/v1/admin/stores",
        store_body("BLR-B", 12.9855, 77.6412, 2.0),
    )
    .await;

    let check = |lat: f64, lng: f64| {
        let app = &app;
        async move {
            app.get(
                &format!("/v1/customer/serviceability?lat={lat}&lng={lng}"),
                None,
            )
            .await
        }
    };

    let (status, res) = check(INSIDE.0, INSIDE.1).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res["serviceable"], true);
    assert_eq!(res["store"]["id"], near, "nearest overlapping store");
    assert_eq!(res["store"]["etaMinutes"], 8);

    let (_, res) = check(12.9840, 77.6412).await;
    assert_eq!(res["store"]["code"], "BLR-B");

    let (_, res) = check(OUTSIDE.0, OUTSIDE.1).await;
    assert_eq!(res, json!({ "serviceable": false, "store": null }));

    let (status, _) = app
        .get("/v1/customer/serviceability?lat=abc&lng=1", None)
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn inactive_stores_do_not_serve(db: PgPool) {
    let app = TestApp::new(db);
    let token = admin_token(&app).await;
    let id = create_store(&app, &token, "BLR-OFF").await;
    let mut body = store_body("BLR-OFF", 12.9719, 77.6412, 2.0);
    body["isActive"] = json!(false);
    app.request(
        Method::PUT,
        &format!("/v1/admin/stores/{}", id.as_str().unwrap()),
        Some(&token),
        Some(body),
    )
    .await;
    let (_, res) = app
        .get(
            &format!(
                "/v1/customer/serviceability?lat={}&lng={}",
                INSIDE.0, INSIDE.1
            ),
            None,
        )
        .await;
    assert_eq!(res["serviceable"], false);
}

#[sqlx::test]
async fn categories_and_products(db: PgPool) {
    let app = TestApp::new(db);
    let token = admin_token(&app).await;

    let dairy = category(&app, &token, "Dairy, Bread & Eggs").await;
    assert_eq!(dairy["slug"], "dairy-bread-eggs", "slug derived from name");
    let (status, _) = app
        .request(
            Method::POST,
            "/v1/admin/categories",
            Some(&token),
            Some(json!({ "name": "Dairy Bread Eggs", "isActive": true })),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "same derived slug");

    // A category can't become its own ancestor.
    let milk = create(
        &app,
        &token,
        "/v1/admin/categories",
        json!({ "name": "Milk", "parentId": dairy["id"], "isActive": true }),
    )
    .await;
    let (status, res) = app
        .request(
            Method::PUT,
            &format!("/v1/admin/categories/{}", dairy["id"].as_str().unwrap()),
            Some(&token),
            Some(json!({ "name": "Dairy", "parentId": milk["id"], "isActive": true })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{res}");

    let p = create(
        &app,
        &token,
        "/v1/admin/products",
        product_body(&dairy["id"], "Taaza Milk", 2800, 2700),
    )
    .await;
    assert_eq!(p["slug"], "amul-taaza-milk-500-ml");
    assert_eq!(p["categoryName"], "Dairy, Bread & Eggs");

    let bad = [
        (
            "price above MRP",
            product_body(&dairy["id"], "X", 1000, 1100),
        ),
        ("zero price", product_body(&dairy["id"], "Y", 1000, 0)),
        (
            "unknown category",
            product_body(&json!(uuid::Uuid::new_v4()), "Z", 1000, 900),
        ),
        ("bad barcode", {
            let mut b = product_body(&dairy["id"], "W", 1000, 900);
            b["barcode"] = json!("12ab");
            b
        }),
        ("foreign image host", {
            let mut b = product_body(&dairy["id"], "V", 1000, 900);
            b["imageUrls"] = json!(["http://evil.test/x.png"]);
            b
        }),
    ];
    for (name, body) in bad {
        let (status, res) = app
            .request(Method::POST, "/v1/admin/products", Some(&token), Some(body))
            .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{name}: {res}");
    }
    let (status, _) = app
        .request(
            Method::POST,
            "/v1/admin/products",
            Some(&token),
            Some(product_body(&dairy["id"], "Taaza Milk", 2800, 2700)),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "duplicate slug");

    // Search: fuzzy on name/brand, exact on barcode.
    let mut with_barcode = product_body(&dairy["id"], "Salted Butter", 6200, 6000);
    with_barcode["barcode"] = json!("8901262010016");
    create(&app, &token, "/v1/admin/products", with_barcode).await;
    for (q, expect) in [
        ("taaza", "Taaza Milk"),
        ("butter", "Salted Butter"),
        ("8901262010016", "Salted Butter"),
    ] {
        let (_, page) = app
            .get(&format!("/v1/admin/products?q={q}"), Some(&token))
            .await;
        assert_eq!(page["items"][0]["name"], expect, "search {q}: {page}");
    }
}

#[sqlx::test]
async fn customer_catalog_is_scoped_to_the_store(db: PgPool) {
    let app = TestApp::new(db);
    let token = admin_token(&app).await;
    let here = create_store(&app, &token, "BLR-HERE").await;
    let there = create(
        &app,
        &token,
        "/v1/admin/stores",
        store_body("BLR-THERE", 12.9352, 77.6245, 2.0),
    )
    .await["id"]
        .clone();
    let dairy = category(&app, &token, "Dairy").await;
    let snacks = category(&app, &token, "Snacks").await;

    let milk = create(
        &app,
        &token,
        "/v1/admin/products",
        product_body(&dairy["id"], "Milk", 3000, 2800),
    )
    .await["id"]
        .clone();
    let curd = create(
        &app,
        &token,
        "/v1/admin/products",
        product_body(&dairy["id"], "Fresh Paneer", 5000, 4500),
    )
    .await["id"]
        .clone();
    let chips = create(
        &app,
        &token,
        "/v1/admin/products",
        product_body(&snacks["id"], "Chips", 2000, 2000),
    )
    .await["id"]
        .clone();
    let hidden = create(
        &app,
        &token,
        "/v1/admin/products",
        product_body(&snacks["id"], "Hidden", 2000, 2000),
    )
    .await["id"]
        .clone();

    let ok = |r: (StatusCode, Value)| assert_eq!(r.0, StatusCode::OK, "{}", r.1);
    ok(stock(&app, &token, &here, &milk, json!({ "quantity": 30, "binLocation": "B-01-1", "priceOverridePaise": 2600, "isAvailable": true })).await);
    ok(stock(
        &app,
        &token,
        &here,
        &curd,
        json!({ "quantity": 0, "isAvailable": true }),
    )
    .await);
    ok(stock(
        &app,
        &token,
        &here,
        &hidden,
        json!({ "quantity": 9, "isAvailable": false }),
    )
    .await);
    ok(stock(
        &app,
        &token,
        &there,
        &chips,
        json!({ "quantity": 5, "isAvailable": true }),
    )
    .await);

    // Override above MRP is rejected.
    let (status, _) = stock(
        &app,
        &token,
        &here,
        &milk,
        json!({ "quantity": 1, "priceOverridePaise": 3100, "isAvailable": true }),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    let store = here.as_str().unwrap();
    let (_, cats) = app
        .get(
            &format!("/v1/customer/catalog/categories?storeId={store}"),
            None,
        )
        .await;
    let names: Vec<&str> = cats
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        ["Dairy"],
        "snacks only stocked elsewhere / unavailable"
    );
    assert_eq!(cats[0]["productCount"], 2);

    let (_, page) = app
        .get(
            &format!("/v1/customer/catalog/products?storeId={store}"),
            None,
        )
        .await;
    let items = page["items"].as_array().unwrap();
    assert_eq!(items.len(), 2, "{page}");
    assert_eq!(items[0]["name"], "Milk", "in-stock first");
    assert_eq!(items[0]["pricePaise"], 2600, "store override applies");
    assert_eq!(items[0]["mrpPaise"], 3000);
    assert_eq!(items[0]["maxQuantity"], 10, "capped per item");
    assert!(
        items[0].get("description").is_none(),
        "no description in lists"
    );
    assert_eq!(items[1]["name"], "Fresh Paneer");
    assert_eq!(items[1]["inStock"], false);

    let (_, other) = app
        .get(
            &format!(
                "/v1/customer/catalog/products?storeId={}",
                there.as_str().unwrap()
            ),
            None,
        )
        .await;
    assert_eq!(other["items"][0]["name"], "Chips");
    assert_eq!(other["total"], 1);

    let (status, _) = app
        .get(
            &format!(
                "/v1/customer/catalog/products/{}?storeId={store}",
                chips.as_str().unwrap()
            ),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "not stocked here");
    let (status, _) = app
        .get(
            &format!(
                "/v1/customer/catalog/products/{}?storeId={store}",
                hidden.as_str().unwrap()
            ),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "marked unavailable");

    let (_, found) = app
        .get(
            &format!("/v1/customer/catalog/products?storeId={store}&q=panner"),
            None,
        )
        .await;
    assert_eq!(
        found["items"][0]["name"], "Fresh Paneer",
        "typo-tolerant search: {found}"
    );
    assert_eq!(found["total"], 1, "milk doesn't match 'panner'");

    // Deactivating a product hides it from customers.
    let mut body = product_body(&dairy["id"], "Milk", 3000, 2800);
    body["isActive"] = json!(false);
    app.request(
        Method::PUT,
        &format!("/v1/admin/products/{}", milk.as_str().unwrap()),
        Some(&token),
        Some(body),
    )
    .await;
    let (_, page) = app
        .get(
            &format!("/v1/customer/catalog/products?storeId={store}"),
            None,
        )
        .await;
    assert_eq!(page["total"], 1);

    let (status, _) = app
        .get(
            &format!(
                "/v1/customer/catalog/products?storeId={}",
                uuid::Uuid::new_v4()
            ),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn mrp_cut_below_a_store_price_is_blocked(db: PgPool) {
    let app = TestApp::new(db);
    let token = admin_token(&app).await;
    let store = create_store(&app, &token, "BLR-M").await;
    let dairy = category(&app, &token, "Dairy").await;
    let milk = create(
        &app,
        &token,
        "/v1/admin/products",
        product_body(&dairy["id"], "Milk", 3000, 2800),
    )
    .await["id"]
        .clone();
    stock(
        &app,
        &token,
        &store,
        &milk,
        json!({ "quantity": 5, "priceOverridePaise": 2900, "isAvailable": true }),
    )
    .await;

    let (status, res) = app
        .request(
            Method::PUT,
            &format!("/v1/admin/products/{}", milk.as_str().unwrap()),
            Some(&token),
            Some(product_body(&dairy["id"], "Milk", 2850, 2800)),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{res}");
    assert!(
        res["error"]["message"].as_str().unwrap().contains("₹29"),
        "{res}"
    );
}

#[sqlx::test]
async fn admin_inventory_lists_stocked_and_unstocked(db: PgPool) {
    let app = TestApp::new(db);
    let token = admin_token(&app).await;
    let store = create_store(&app, &token, "BLR-I").await;
    let dairy = category(&app, &token, "Dairy").await;
    let milk = create(
        &app,
        &token,
        "/v1/admin/products",
        product_body(&dairy["id"], "Milk", 3000, 2800),
    )
    .await["id"]
        .clone();
    create(
        &app,
        &token,
        "/v1/admin/products",
        product_body(&dairy["id"], "Curd", 5000, 4500),
    )
    .await;
    stock(
        &app,
        &token,
        &store,
        &milk,
        json!({ "quantity": 7, "binLocation": "B-02-1", "isAvailable": true }),
    )
    .await;

    let base = format!("/v1/admin/stores/{}/inventory", store.as_str().unwrap());
    let (_, all) = app.get(&base, Some(&token)).await;
    assert_eq!(all["total"], 2);
    assert_eq!(all["items"][0]["productName"], "Milk", "stocked first");
    assert_eq!(all["items"][0]["quantity"], 7);
    assert_eq!(all["items"][1]["stocked"], false);

    let (_, stocked) = app.get(&format!("{base}?stocked=true"), Some(&token)).await;
    assert_eq!(stocked["total"], 1);
    let (_, by_bin) = app.get(&format!("{base}?q=B-02"), Some(&token)).await;
    assert_eq!(by_bin["items"][0]["productName"], "Milk");

    let (status, _) = stock(
        &app,
        &token,
        &store,
        &milk,
        json!({ "quantity": -1, "isAvailable": true }),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, _) = stock(
        &app,
        &token,
        &json!(uuid::Uuid::new_v4()),
        &milk,
        json!({ "quantity": 1, "isAvailable": true }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn staff_roles_require_an_active_store(db: PgPool) {
    let app = TestApp::new(db);
    let token = admin_token(&app).await;
    let store = create_store(&app, &token, "BLR-S").await;
    let (_, picker) = app.signed_in(&random_phone()).await;
    let uri = format!("/v1/admin/users/{}/role", picker["id"].as_str().unwrap());

    for body in [
        json!({ "role": "PICKER" }),
        json!({ "role": "PICKER", "storeId": uuid::Uuid::new_v4() }),
    ] {
        let (status, res) = app
            .request(Method::PATCH, &uri, Some(&token), Some(body))
            .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{res}");
    }
    let (status, res) = app
        .request(
            Method::PATCH,
            &uri,
            Some(&token),
            Some(json!({ "role": "PICKER", "storeId": store })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{res}");
    assert_eq!(res["storeId"], store);

    // Back to CUSTOMER clears the store.
    let (_, res) = app
        .request(
            Method::PATCH,
            &uri,
            Some(&token),
            Some(json!({ "role": "CUSTOMER", "storeId": store })),
        )
        .await;
    assert!(res["storeId"].is_null());
}

#[sqlx::test]
async fn catalog_admin_endpoints_are_admin_only(db: PgPool) {
    let app = TestApp::new(db);
    let (customer, _) = app.signed_in(&random_phone()).await;
    for uri in [
        "/v1/admin/stores",
        "/v1/admin/categories",
        "/v1/admin/products",
    ] {
        let (status, body) = app.get(uri, Some(&customer)).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{uri}");
        assert_eq!(error_code(&body), "FORBIDDEN");
        let (status, _) = app.get(uri, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{uri}");
    }
    let _ = hexagon(0.0, 0.0, 1.0);
}

#[sqlx::test]
async fn image_uploads(db: PgPool) {
    let app = TestApp::new(db);
    let token = admin_token(&app).await;

    let png: &[u8] =
        b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR\0\0\0\x01\0\0\0\x01\x08\x06\0\0\0\x1f\x15\xc4\x89";
    let (status, res) = app
        .upload("/v1/admin/uploads", &token, "file", "photo.png", png)
        .await;
    assert_eq!(status, StatusCode::CREATED, "{res}");
    let url = res["url"].as_str().unwrap();
    assert!(url.starts_with("/media/") && url.ends_with(".png"), "{url}");

    let (status, bytes, headers) = app.raw_get(url).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(bytes, png);
    assert_eq!(headers.get("content-type").unwrap(), "image/png");
    assert_eq!(headers.get("x-content-type-options").unwrap(), "nosniff");

    // The file extension/name is not trusted: SVG renamed to .png is rejected.
    let (status, res) = app
        .upload(
            "/v1/admin/uploads",
            &token,
            "file",
            "x.png",
            b"<svg xmlns='http://www.w3.org/2000/svg'/>",
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{res}");

    let big = vec![0xFFu8; 6 * 1024 * 1024];
    let (status, _) = app
        .upload("/v1/admin/uploads", &token, "file", "big.jpg", &big)
        .await;
    assert!(
        status == StatusCode::UNPROCESSABLE_ENTITY || status == StatusCode::PAYLOAD_TOO_LARGE,
        "{status}"
    );

    let (status, _) = app
        .upload("/v1/admin/uploads", &token, "other", "x.png", png)
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (customer, _) = app.signed_in(&random_phone()).await;
    let (status, _) = app
        .upload("/v1/admin/uploads", &customer, "file", "x.png", png)
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
