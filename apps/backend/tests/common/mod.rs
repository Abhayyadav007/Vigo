//! Shared harness: builds the real router over a per-test database (from
//! `#[sqlx::test]`) and the dev Redis, with a verifier that trusts the RSA
//! key in `tests/fixtures`, so tokens go through the real RS256 checks.

#![allow(dead_code)]

use std::{collections::HashMap, sync::Arc, time::Duration};

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode, header},
};
use backend::{
    app,
    auth::FirebaseVerifier,
    config::{AppEnv, Config},
    services::{
        media_service::MediaStore,
        payments::{CashOnDelivery, Payments, Razorpay},
    },
    state::AppState,
    ws::hub::PubSubHub,
};
use futures::StreamExt;
use http_body_util::BodyExt;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header};
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio_tungstenite::tungstenite::Message;
use tower::ServiceExt;

pub const PROJECT_ID: &str = "vigo-test";
pub const RAZORPAY_KEY: &str = "rzp_test_vigo";
pub const RAZORPAY_WEBHOOK_SECRET: &str = "whsec_test_vigo";
pub const KID: &str = "test-kid";
const PRIVATE_KEY: &[u8] = include_bytes!("../fixtures/test_rsa_private.pem");
const PUBLIC_KEY: &[u8] = include_bytes!("../fixtures/test_rsa_public.pem");
const OTHER_PRIVATE_KEY: &[u8] = include_bytes!("../fixtures/other_rsa_private.pem");

pub struct TestApp {
    pub state: AppState,
    router: Router,
}

impl TestApp {
    pub fn new(db: PgPool) -> Self {
        Self::with_config(db, |_| {})
    }

    /// Like `new`, with a hook to tweak config (e.g. a short reservation TTL).
    pub fn with_config(db: PgPool, tweak: impl FnOnce(&mut Config)) -> Self {
        let _ = dotenvy::dotenv();
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".into());
        let redis = deadpool_redis::Config::from_url(&redis_url)
            .create_pool(Some(deadpool_redis::Runtime::Tokio1))
            .expect("redis pool");
        let key = DecodingKey::from_rsa_pem(PUBLIC_KEY).expect("public key");
        let verifier =
            FirebaseVerifier::with_static_keys(PROJECT_ID, HashMap::from([(KID.into(), key)]));

        let mut config = Config {
            app_env: AppEnv::Development,
            addr: "127.0.0.1:0".parse().expect("addr"),
            database_url: String::new(),
            db_max_connections: 5,
            redis_url,
            firebase_project_id: PROJECT_ID.into(),
            firebase_auth_emulator_host: None,
            session_cache_ttl: Duration::from_secs(300),
            cors_allowed_origins: vec![],
            request_timeout: Duration::from_secs(10),
            media_dir: std::env::temp_dir()
                .join(format!("vigo-test-media-{}", uuid::Uuid::new_v4())),
            reservation_ttl: Duration::from_secs(600),
            razorpay: Some((RAZORPAY_KEY.into(), RAZORPAY_WEBHOOK_SECRET.into())),
            dispatch: backend::config::DispatchConfig::default(),
        };
        tweak(&mut config);
        let payments = Payments {
            cod: CashOnDelivery,
            razorpay: config.razorpay.clone().map(|(k, s)| Razorpay::new(k, s)),
        };
        let media_dir = config.media_dir.clone();
        let redis_url_for_hub = config.redis_url.clone();
        let state = AppState {
            config: Arc::new(config),
            db,
            redis,
            verifier: Arc::new(verifier),
            media: Arc::new(MediaStore::Local { dir: media_dir }),
            payments: Arc::new(payments),
            hub: PubSubHub::start(&redis_url_for_hub).expect("pub/sub hub"),
        };
        Self {
            router: app::build_router(state.clone()),
            state,
        }
    }

    pub async fn request(
        &self,
        method: Method,
        uri: &str,
        token: Option<&str>,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut req = Request::builder().method(method).uri(uri);
        if let Some(t) = token {
            req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
        }
        let req = match body {
            Some(b) => req
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(b.to_string())),
            None => req.body(Body::empty()),
        }
        .expect("request");

        let res = self.router.clone().oneshot(req).await.expect("response");
        let status = res.status();
        let bytes = res.into_body().collect().await.expect("body").to_bytes();
        let json = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes)
                .unwrap_or_else(|_| panic!("non-JSON body: {}", String::from_utf8_lossy(&bytes)))
        };
        (status, json)
    }

    /// Multipart upload of a single file field.
    pub async fn upload(
        &self,
        uri: &str,
        token: &str,
        field: &str,
        filename: &str,
        bytes: &[u8],
    ) -> (StatusCode, Value) {
        let boundary = "vigo-test-boundary";
        let mut body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"{field}\"; filename=\"{filename}\"\r\nContent-Type: application/octet-stream\r\n\r\n"
        )
        .into_bytes();
        body.extend_from_slice(bytes);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        let req = Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(Body::from(body))
            .expect("request");
        let res = self.router.clone().oneshot(req).await.expect("response");
        let status = res.status();
        let bytes = res.into_body().collect().await.expect("body").to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    /// Serves the real router on a random local port (for WebSocket tests).
    pub async fn serve(&self) -> std::net::SocketAddr {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        let router = self.router.clone();
        tokio::spawn(async move {
            axum::serve(listener, router).await.expect("serve");
        });
        addr
    }

    /// GET returning raw bytes and headers (static files).
    pub async fn raw_get(&self, uri: &str) -> (StatusCode, Vec<u8>, axum::http::HeaderMap) {
        let req = Request::builder()
            .uri(uri)
            .body(Body::empty())
            .expect("request");
        let res = self.router.clone().oneshot(req).await.expect("response");
        let status = res.status();
        let headers = res.headers().clone();
        let bytes = res.into_body().collect().await.expect("body").to_bytes();
        (status, bytes.to_vec(), headers)
    }

    /// POST with extra headers (Idempotency-Key, webhook signatures).
    pub async fn post_with_headers(
        &self,
        uri: &str,
        token: Option<&str>,
        headers: &[(&str, &str)],
        body: &[u8],
    ) -> (StatusCode, Value) {
        let mut req = Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header(header::CONTENT_TYPE, "application/json");
        if let Some(t) = token {
            req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
        }
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        let res = self
            .router
            .clone()
            .oneshot(req.body(Body::from(body.to_vec())).expect("request"))
            .await
            .expect("response");
        let status = res.status();
        let bytes = res.into_body().collect().await.expect("body").to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    pub async fn get(&self, uri: &str, token: Option<&str>) -> (StatusCode, Value) {
        self.request(Method::GET, uri, token, None).await
    }

    /// Signs in (sync) as a new user and returns their token.
    pub async fn signed_in(&self, phone: &str) -> (String, Value) {
        let token = TokenBuilder::new().phone(phone).sign();
        let (status, body) = self
            .request(Method::POST, "/v1/auth/sync", Some(&token), None)
            .await;
        assert_eq!(status, StatusCode::OK, "sync failed: {body}");
        (token, body)
    }
}

/// Builds Firebase-shaped ID tokens; defaults are valid.
#[derive(Clone)]
pub struct TokenBuilder {
    pub claims: serde_json::Map<String, Value>,
    kid: Option<String>,
    key: &'static [u8],
    alg: Algorithm,
}

impl TokenBuilder {
    pub fn new() -> Self {
        let now = chrono::Utc::now().timestamp();
        let uid = format!("uid-{}", uuid::Uuid::new_v4().simple());
        let claims = json!({
            "sub": uid,
            "aud": PROJECT_ID,
            "iss": format!("https://securetoken.google.com/{PROJECT_ID}"),
            "iat": now,
            "exp": now + 3600,
            "auth_time": now,
            "phone_number": random_phone(),
            "firebase": { "sign_in_provider": "phone" },
        });
        let Value::Object(claims) = claims else {
            unreachable!()
        };
        Self {
            claims,
            kid: Some(KID.into()),
            key: PRIVATE_KEY,
            alg: Algorithm::RS256,
        }
    }

    pub fn set(mut self, claim: &str, value: Value) -> Self {
        self.claims.insert(claim.into(), value);
        self
    }

    pub fn remove(mut self, claim: &str) -> Self {
        self.claims.remove(claim);
        self
    }

    pub fn uid(self, uid: &str) -> Self {
        self.set("sub", json!(uid))
    }

    pub fn phone(self, phone: &str) -> Self {
        self.set("phone_number", json!(phone))
    }

    pub fn kid(mut self, kid: Option<&str>) -> Self {
        self.kid = kid.map(str::to_owned);
        self
    }

    pub fn signed_by_other_key(mut self) -> Self {
        self.key = OTHER_PRIVATE_KEY;
        self
    }

    pub fn sub(&self) -> String {
        self.claims["sub"].as_str().unwrap_or_default().to_owned()
    }

    pub fn sign(&self) -> String {
        let mut header = Header::new(self.alg);
        header.kid = self.kid.clone();
        let key = EncodingKey::from_rsa_pem(self.key).expect("private key");
        jsonwebtoken::encode(&header, &self.claims, &key).expect("sign")
    }
}

/// A random valid Indian mobile number, so parallel tests never collide.
pub fn random_phone() -> String {
    let n = uuid::Uuid::new_v4().as_u128() % 1_000_000_000;
    format!("+919{n:09}")
}

pub fn error_code(body: &Value) -> &str {
    body["error"]["code"].as_str().unwrap_or("<none>")
}

/// A ~2 km hexagon around a point (GeoJSON `[lng, lat]` ring).
pub fn hexagon(lat: f64, lng: f64, km: f64) -> Value {
    let ring: Vec<[f64; 2]> = (0..=6)
        .map(|i| {
            let a = f64::from(i % 6 * 60 + 30).to_radians();
            [
                lng + km * a.cos() / (111.32 * lat.to_radians().cos()),
                lat + km * a.sin() / 110.57,
            ]
        })
        .collect();
    json!({ "type": "Polygon", "coordinates": [ring] })
}

pub fn store_body(code: &str, lat: f64, lng: f64, km: f64) -> Value {
    json!({
        "code": code,
        "name": format!("Store {code}"),
        "address": "Test address, Bengaluru",
        "location": { "lat": lat, "lng": lng },
        "serviceArea": hexagon(lat, lng, km),
        "isActive": true,
    })
}

/// Creates a store around Indiranagar, Bengaluru; returns its id.
pub async fn create_store(app: &TestApp, admin_token: &str, code: &str) -> Value {
    let (status, body) = app
        .request(
            Method::POST,
            "/v1/admin/stores",
            Some(admin_token),
            Some(store_body(code, 12.9719, 77.6412, 2.0)),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "create store: {body}");
    body["id"].clone()
}

/// Signs in a fresh user and promotes them to ADMIN; returns their token.
pub async fn admin_token(app: &TestApp) -> String {
    let phone = random_phone();
    let (token, _) = app.signed_in(&phone).await;
    backend::services::user_service::promote_admin_by_phone(&app.state, &phone)
        .await
        .expect("promote");
    token
}

/// Next WebSocket text message as JSON (5 s timeout).
pub async fn next_json<S>(ws: &mut S) -> Value
where
    S: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    loop {
        let msg = tokio::time::timeout(Duration::from_secs(5), ws.next())
            .await
            .expect("timed out waiting for a message")
            .expect("socket closed")
            .expect("socket error");
        if let Message::Text(text) = msg {
            return serde_json::from_str(&text).unwrap();
        }
    }
}

/// Next message of the given `type`, skipping others.
pub async fn next_of<S>(ws: &mut S, kind: &str) -> Value
where
    S: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    loop {
        let msg = next_json(ws).await;
        if msg["type"] == kind {
            return msg;
        }
    }
}
