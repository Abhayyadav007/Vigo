use axum::{Json, extract::State, http::StatusCode};

use crate::{
    cache,
    dto::health::{ComponentStatus, HealthResponse},
    repositories,
    state::AppState,
};

/// `GET /healthz`: 200 when Postgres and Redis are both reachable, 503 otherwise.
pub async fn healthz(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    let (db, redis) = tokio::join!(
        repositories::health::ping(&state.db),
        cache::ping(&state.redis)
    );

    let database = component("database", db.map_err(|e| e.to_string()));
    let redis = component("redis", redis.map_err(|e| e.to_string()));
    let healthy = database == ComponentStatus::Ok && redis == ComponentStatus::Ok;

    let body = HealthResponse {
        status: if healthy {
            ComponentStatus::Ok
        } else {
            ComponentStatus::Down
        },
        database,
        redis,
        version: env!("CARGO_PKG_VERSION").to_owned(),
    };
    let code = if healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (code, Json(body))
}

fn component(name: &str, result: Result<(), String>) -> ComponentStatus {
    match result {
        Ok(()) => ComponentStatus::Ok,
        Err(error) => {
            tracing::warn!(component = name, %error, "health check failed");
            ComponentStatus::Down
        }
    }
}
