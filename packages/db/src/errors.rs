use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("Dababase error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Migration error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
}

