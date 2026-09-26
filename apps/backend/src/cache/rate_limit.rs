//! Fixed-window rate limits in Redis (INCR + EXPIRE).

use std::time::Duration;

use axum::http::StatusCode;
use deadpool_redis::{Pool, redis};

use crate::error::{AppError, AppResult};

/// Counts one hit on `key`; errors with 429 once `limit` hits land in `window`.
/// Needs Redis 7+ (`EXPIRE ... NX`).
/// ponytail: fixed window allows up to 2x limit across a window edge; sliding
/// window (sorted set) if that burst ever matters.
pub async fn check(redis: &Pool, key: &str, limit: u32, window: Duration) -> AppResult<()> {
    let result: Result<u32, AppError> = async {
        let mut conn = redis.get().await?;
        let (count,): (u32,) = redis::pipe()
            .incr(format!("rl:{key}"), 1)
            .cmd("EXPIRE")
            .arg(format!("rl:{key}"))
            .arg(window.as_secs().max(1))
            .arg("NX")
            .ignore()
            .query_async(&mut conn)
            .await?;
        Ok(count)
    }
    .await;
    match result {
        Ok(count) if count > limit => Err(AppError::Coded {
            status: StatusCode::TOO_MANY_REQUESTS,
            code: "RATE_LIMITED",
            message: "too many requests; wait a minute and try again".into(),
        }),
        Ok(_) => Ok(()),
        // Fail open: a Redis blip shouldn't block sign-in or checkout.
        Err(error) => {
            tracing::warn!(%error, key, "rate limiter unavailable; allowing request");
            Ok(())
        }
    }
}
