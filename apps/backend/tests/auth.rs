// Tests may panic freely; the no-unwrap/expect rule is for production code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use axum::http::{Method, StatusCode};
use backend::services::user_service;
use common::{TestApp, TokenBuilder, error_code, random_phone};
use serde_json::json;
use sqlx::PgPool;

#[sqlx::test]
async fn sync_creates_customer_and_is_idempotent(db: PgPool) {
    let app = TestApp::new(db);
    let phone = random_phone();
    let token = TokenBuilder::new().phone(&phone).sign();

    let (status, first) = app
        .request(Method::POST, "/v1/auth/sync", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{first}");
    assert_eq!(first["phone"], phone);
    assert_eq!(first["role"], "CUSTOMER");
    assert!(first["storeId"].is_null());

    let (status, second) = app
        .request(Method::POST, "/v1/auth/sync", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(second["id"], first["id"], "same user on repeat sync");

    let (status, me) = app.get("/v1/auth/me", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["id"], first["id"]);
}

#[sqlx::test]
async fn missing_or_malformed_auth_header_is_401(db: PgPool) {
    let app = TestApp::new(db);
    for token in [None, Some(""), Some("not-a-jwt"), Some("a.b.c")] {
        let (status, body) = app.get("/v1/auth/me", token).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "token {token:?}");
        assert_eq!(error_code(&body), "UNAUTHORIZED");
    }
}

#[sqlx::test]
async fn rejects_tokens_that_fail_verification(db: PgPool) {
    let app = TestApp::new(db);
    let now = chrono::Utc::now().timestamp();
    let cases = [
        (
            "wrong audience",
            TokenBuilder::new().set("aud", json!("other-project")),
        ),
        (
            "wrong issuer",
            TokenBuilder::new().set("iss", json!("https://securetoken.google.com/other")),
        ),
        ("expired", TokenBuilder::new().set("exp", json!(now - 3600))),
        (
            "issued in the future",
            TokenBuilder::new().set("iat", json!(now + 3600)),
        ),
        (
            "auth_time in the future",
            TokenBuilder::new().set("auth_time", json!(now + 3600)),
        ),
        ("empty subject", TokenBuilder::new().uid("")),
        ("missing subject", TokenBuilder::new().remove("sub")),
        (
            "unknown key id",
            TokenBuilder::new().kid(Some("rotated-away")),
        ),
        ("no key id", TokenBuilder::new().kid(None)),
        ("bad signature", TokenBuilder::new().signed_by_other_key()),
    ];
    for (name, builder) in cases {
        let (status, body) = app
            .request(Method::POST, "/v1/auth/sync", Some(&builder.sign()), None)
            .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{name}: {body}");
        assert_eq!(error_code(&body), "UNAUTHORIZED", "{name}");
    }
}

#[sqlx::test]
async fn sync_is_india_only_and_requires_phone(db: PgPool) {
    let app = TestApp::new(db);
    for (name, builder) in [
        ("US number", TokenBuilder::new().phone("+14155550100")),
        ("landline-like", TokenBuilder::new().phone("+914012345678")),
        ("no phone", TokenBuilder::new().remove("phone_number")),
    ] {
        let (status, body) = app
            .request(Method::POST, "/v1/auth/sync", Some(&builder.sign()), None)
            .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{name}: {body}");
        assert_eq!(error_code(&body), "VALIDATION_FAILED");
    }
}

#[sqlx::test]
async fn me_before_sync_is_user_not_registered(db: PgPool) {
    let app = TestApp::new(db);
    let (status, body) = app
        .get("/v1/auth/me", Some(&TokenBuilder::new().sign()))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(error_code(&body), "USER_NOT_REGISTERED");
}

#[sqlx::test]
async fn same_phone_under_new_firebase_uid_keeps_the_account(db: PgPool) {
    let app = TestApp::new(db);
    let phone = random_phone();
    let (_, original) = app.signed_in(&phone).await;

    let (_, relinked) = app.signed_in(&phone).await;
    assert_eq!(relinked["id"], original["id"]);
}

#[sqlx::test]
async fn disabled_accounts_are_rejected(db: PgPool) {
    let app = TestApp::new(db.clone());
    let phone = random_phone();
    let (token, _) = app.signed_in(&phone).await;
    sqlx::query("UPDATE users SET is_active = false WHERE phone = $1")
        .bind(&phone)
        .execute(&db)
        .await
        .unwrap();

    // Re-sync reads Postgres directly and refuses.
    let (status, body) = app
        .request(Method::POST, "/v1/auth/sync", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(error_code(&body), "ACCOUNT_DISABLED");
}

#[sqlx::test]
async fn customers_cannot_use_admin_endpoints(db: PgPool) {
    let app = TestApp::new(db);
    let (token, _) = app.signed_in(&random_phone()).await;
    let (status, body) = app.get("/v1/admin/users", Some(&token)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(error_code(&body), "FORBIDDEN");
}

pub async fn admin(app: &TestApp) -> (String, serde_json::Value) {
    let phone = random_phone();
    let (token, _) = app.signed_in(&phone).await;
    let user = user_service::promote_admin_by_phone(&app.state, &phone)
        .await
        .unwrap();
    assert_eq!(user.role.as_str(), "ADMIN");
    let (_, me) = app.get("/v1/auth/me", Some(&token)).await;
    (token, me)
}

#[sqlx::test]
async fn role_change_takes_effect_immediately(db: PgPool) {
    let app = TestApp::new(db);
    let (admin_token, _) = admin(&app).await;
    let (rider_token, rider) = app.signed_in(&random_phone()).await;

    // The rider's CUSTOMER session is now cached in Redis (TTL 300s).
    let (_, me) = app.get("/v1/auth/me", Some(&rider_token)).await;
    assert_eq!(me["role"], "CUSTOMER");

    let store_id = common::create_store(&app, &admin_token, "BLR-TEST-01").await;
    let uri = format!("/v1/admin/users/{}/role", rider["id"].as_str().unwrap());
    let (status, updated) = app
        .request(
            Method::PATCH,
            &uri,
            Some(&admin_token),
            Some(json!({ "role": "RIDER", "storeId": store_id })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(updated["role"], "RIDER");
    assert_eq!(updated["storeId"], store_id);

    // Cache was invalidated, so the new role is visible without waiting.
    let (_, me) = app.get("/v1/auth/me", Some(&rider_token)).await;
    assert_eq!(me["role"], "RIDER");

    // And the old role lost its access: a rider is not an admin.
    let (status, _) = app.get("/v1/admin/users", Some(&rider_token)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn admins_cannot_change_their_own_role(db: PgPool) {
    let app = TestApp::new(db);
    let (token, me) = admin(&app).await;
    let uri = format!("/v1/admin/users/{}/role", me["id"].as_str().unwrap());
    let (status, body) = app
        .request(
            Method::PATCH,
            &uri,
            Some(&token),
            Some(json!({ "role": "CUSTOMER" })),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
}

#[sqlx::test]
async fn role_update_input_errors_use_the_json_error_shape(db: PgPool) {
    let app = TestApp::new(db);
    let (token, _) = admin(&app).await;
    let (_, target) = app.signed_in(&random_phone()).await;
    let uri = format!("/v1/admin/users/{}/role", target["id"].as_str().unwrap());

    let cases = [
        ("unknown role", uri.as_str(), json!({ "role": "SUPERUSER" })),
        (
            "unknown field",
            uri.as_str(),
            json!({ "role": "RIDER", "isAdmin": true }),
        ),
        (
            "bad uuid in path",
            "/v1/admin/users/not-a-uuid/role",
            json!({ "role": "RIDER" }),
        ),
    ];
    for (name, uri, body) in cases {
        let (status, res) = app
            .request(Method::PATCH, uri, Some(&token), Some(body))
            .await;
        assert!(status.is_client_error(), "{name}: {status}");
        assert!(res["error"]["code"].is_string(), "{name}: {res}");
    }

    let missing = format!("/v1/admin/users/{}/role", uuid::Uuid::new_v4());
    let (status, res) = app
        .request(
            Method::PATCH,
            &missing,
            Some(&token),
            Some(json!({ "role": "ADMIN" })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{res}");
}

#[sqlx::test]
async fn admin_lists_and_filters_users(db: PgPool) {
    let app = TestApp::new(db);
    let (token, _) = admin(&app).await;
    let phones: Vec<String> = (0..3).map(|_| random_phone()).collect();
    for p in &phones {
        app.signed_in(p).await;
    }

    let (status, page) = app.get("/v1/admin/users?limit=2", Some(&token)).await;
    assert_eq!(status, StatusCode::OK, "{page}");
    assert_eq!(page["items"].as_array().unwrap().len(), 2);
    assert_eq!(page["total"], 4, "3 customers + the admin");
    assert_eq!(page["limit"], 2);

    let (_, admins) = app.get("/v1/admin/users?role=ADMIN", Some(&token)).await;
    assert_eq!(admins["total"], 1);

    let fragment = &phones[0][3..10];
    let (_, by_phone) = app
        .get(&format!("/v1/admin/users?phone={fragment}"), Some(&token))
        .await;
    assert!(
        by_phone["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|u| u["phone"] == phones[0].as_str())
    );

    for bad in [
        "limit=0",
        "limit=101",
        "offset=-1",
        "phone=abc",
        "role=KING",
    ] {
        let (status, body) = app
            .get(&format!("/v1/admin/users?{bad}"), Some(&token))
            .await;
        assert!(status.is_client_error(), "{bad}: {status} {body}");
        assert!(body["error"]["code"].is_string(), "{bad}: {body}");
    }
}

#[sqlx::test]
async fn sync_is_rate_limited_per_firebase_user(db: PgPool) {
    let app = TestApp::new(db);
    let token = TokenBuilder::new().phone(&random_phone()).sign();
    for i in 1..=10 {
        let (status, _) = app
            .request(Method::POST, "/v1/auth/sync", Some(&token), None)
            .await;
        assert_eq!(status, StatusCode::OK, "attempt {i}");
    }
    let (status, body) = app
        .request(Method::POST, "/v1/auth/sync", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(error_code(&body), "RATE_LIMITED");
}
