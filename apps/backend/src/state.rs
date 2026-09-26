use std::{sync::Arc, time::Duration};

use anyhow::Context;
use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::{
    auth::FirebaseVerifier,
    cache,
    config::Config,
    services::{
        media_service::MediaStore,
        payments::{CashOnDelivery, Payments, Razorpay},
    },
    ws::hub::PubSubHub,
};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: PgPool,
    pub redis: deadpool_redis::Pool,
    pub verifier: Arc<FirebaseVerifier>,
    pub media: Arc<MediaStore>,
    pub payments: Arc<Payments>,
    /// Redis Pub/Sub fan-out for WebSocket sessions.
    pub hub: PubSubHub,
}

impl AppState {
    /// Connects to Postgres (running pending migrations) and Redis, and picks
    /// the Firebase token verifier for this environment.
    pub async fn connect(config: Config) -> anyhow::Result<Self> {
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

        let verifier = match &config.firebase_auth_emulator_host {
            Some(host) => {
                tracing::warn!(
                    %host,
                    "FIREBASE_AUTH_EMULATOR_HOST set: accepting UNSIGNED emulator tokens (dev only)"
                );
                FirebaseVerifier::emulator(&config.firebase_project_id)
            }
            None => {
                let http = reqwest::Client::builder()
                    .timeout(Duration::from_secs(10))
                    .build()
                    .context("building HTTP client")?;
                FirebaseVerifier::google(&config.firebase_project_id, http)
            }
        };

        tracing::info!(dir = %config.media_dir.display(), "local media store");
        let media = MediaStore::Local {
            dir: config.media_dir.clone(),
        };

        let payments = Payments {
            cod: CashOnDelivery,
            razorpay: config
                .razorpay
                .clone()
                .map(|(key, secret)| Razorpay::new(key, secret)),
        };
        if payments.razorpay.is_none() {
            tracing::info!("Razorpay not configured: online payments disabled (COD only)");
        }

        let hub = PubSubHub::start(&config.redis_url).context("starting pub/sub hub")?;

        Ok(Self {
            hub,
            payments: Arc::new(payments),
            config: Arc::new(config),
            db,
            redis,
            verifier: Arc::new(verifier),
            media: Arc::new(media),
        })
    }
}
