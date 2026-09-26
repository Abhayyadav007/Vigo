//! Redis access.

pub mod events;
pub mod geo;
pub mod inventory;
pub mod session;

use deadpool_redis::{Pool, redis};

use crate::error::AppError;

pub async fn ping(redis: &Pool) -> Result<(), AppError> {
    let mut conn = redis.get().await?;
    let _: String = redis::cmd("PING").query_async(&mut conn).await?;
    Ok(())
}
