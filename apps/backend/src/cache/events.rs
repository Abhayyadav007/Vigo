//! Order events over Redis Pub/Sub, so every backend instance (and the
//! WebSocket fan-out in later phases) sees status changes.

use deadpool_redis::{Pool, redis::AsyncCommands};
use serde::Serialize;
use uuid::Uuid;

use crate::error::AppError;

pub fn order_channel(order_id: Uuid) -> String {
    format!("orders:{order_id}")
}

pub fn store_channel(store_id: Uuid) -> String {
    format!("stores:{store_id}:orders")
}

/// Publishes `event` as JSON to each channel.
pub async fn publish<T: Serialize>(
    redis: &Pool,
    channels: &[String],
    event: &T,
) -> Result<(), AppError> {
    let payload = serde_json::to_string(event).map_err(anyhow::Error::from)?;
    let mut conn = redis.get().await?;
    for channel in channels {
        let _: i64 = conn.publish(channel, &payload).await?;
    }
    Ok(())
}
