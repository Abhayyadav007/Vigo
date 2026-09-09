pub mod shops;

use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

// Re-exported so `api` can hold a pool without depending on sqlx itself.
pub use sqlx::PgPool;

pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await
}

/// Cheap liveness probe for `/health`.
pub async fn ping(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!("SELECT 1").fetch_one(pool).await?;
    Ok(())
}
