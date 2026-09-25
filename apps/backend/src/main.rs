use std::{sync::Arc, time::Duration};

use anyhow::Context;
use sqlx::postgres::PgPoolOptions;
use tokio::{net::TcpListener, signal};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::{config::Config, state::AppState};

mod app;
mod cache;
mod config;
mod dto;
mod error;
mod handlers;
mod repositories;
mod routes;
mod state;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // A missing .env is fine; real deployments set env vars directly.
    let _ = dotenvy::dotenv();
    let config = Config::from_env()?;
    init_tracing(&config);

    let db = PgPoolOptions::new()
        .max_connections(config.db_max_connections)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&config.database_url)
        .await
        .context("connecting to Postgres")?;
    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .context("running migrations")?;

    let redis = deadpool_redis::Config::from_url(&config.redis_url)
        .create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .context("creating Redis pool")?;
    cache::ping(&redis)
        .await
        .map_err(|e| anyhow::anyhow!("connecting to Redis: {e}"))?;

    let addr = config.addr;
    let state = AppState {
        config: Arc::new(config),
        db,
        redis,
    };
    let app = app::build_router(state);

    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("binding {addr}"))?;
    tracing::info!(%addr, "backend listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("serving HTTP")?;
    Ok(())
}

fn init_tracing(config: &Config) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,backend=debug,tower_http=info,sqlx=warn"));
    let registry = tracing_subscriber::registry().with(filter);
    if config.is_production() {
        registry
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        registry.with(tracing_subscriber::fmt::layer()).init();
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(error) = signal::ctrl_c().await {
            tracing::error!(%error, "failed to listen for ctrl-c");
        }
    };
    #[cfg(unix)]
    let terminate = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(error) => tracing::error!(%error, "failed to listen for SIGTERM"),
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    tracing::info!("shutdown signal received");
}
