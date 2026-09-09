use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use db::PgPool;
use domain::NearbyShop;
use serde::Deserialize;
use serde_json::json;
use shared::{AppError, Config};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
struct AppState {
    pool: PgPool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    shared::telemetry::init();

    let config = Config::from_env()?;
    let pool = db::connect(&config.database_url).await?;
    tracing::info!("connected to database");

    let app = Router::new()
        .route("/health", get(health))
        .route("/shops/nearby", get(nearby))
        .layer(TraceLayer::new_for_http())
        .with_state(AppState { pool });

    let listener = TcpListener::bind(("0.0.0.0", config.port)).await?;
    tracing::info!(port = config.port, "listening");
    axum::serve(listener, app).await?;

    Ok(())
}

/// Real liveness check - actually queries the database rather than returning
/// a hardcoded "ok", so a dead pool surfaces as 503.
async fn health(State(state): State<AppState>) -> impl IntoResponse {
    match db::ping(&state.pool).await {
        Ok(()) => (
            StatusCode::OK,
            Json(json!({ "status": "ok", "database": "up" })),
        ),
        Err(err) => {
            tracing::error!(error = ?err, "health check failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "status": "degraded", "database": "down" })),
            )
        }
    }
}

#[derive(Debug, Deserialize)]
struct NearbyParams {
    lat: f64,
    lng: f64,
}

async fn nearby(
    State(state): State<AppState>,
    Query(params): Query<NearbyParams>,
) -> Result<Json<Vec<NearbyShop>>, AppError> {
    if !(-90.0..=90.0).contains(&params.lat) || !(-180.0..=180.0).contains(&params.lng) {
        return Err(AppError::BadRequest("lat/lng out of range".into()));
    }

    let shops = db::shops::nearby(&state.pool, params.lat, params.lng).await?;
    Ok(Json(shops))
}
