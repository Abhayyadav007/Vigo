//! Redis access. Later phases add `inventory` and `geo` submodules.

pub mod session;

use deadpool_redis::{Pool, redis};

use crate::error::AppError;

pub async fn ping(redis: &Pool) -> Result<(), AppError> {
    let mut conn = redis.get().await?;
    let _: String = redis::cmd("PING").query_async(&mut conn).await?;
    Ok(())
}
