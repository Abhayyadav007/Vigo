use std::sync::Arc;

use sqlx::PgPool;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: PgPool,
    pub redis: deadpool_redis::Pool,
    // TODO(phase-2): `jwks: JwksCache` for Firebase ID token verification.
}
