use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::errors::DbError;

pub async fn get_pool(database_url: &str) -> Result<PgPool, DbError> {
    let url = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;
    Ok(url)
}
pub async fn migrrate_pool(pool: &PgPool) -> Result<(), DbError> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}
