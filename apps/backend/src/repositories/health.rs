use sqlx::PgPool;

/// Round-trips a trivial query to confirm the pool can reach Postgres.
pub async fn ping(db: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(r#"SELECT 1 AS "one!""#)
        .fetch_one(db)
        .await?;
    Ok(())
}
