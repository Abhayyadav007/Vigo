//! `firebase_uid -> SessionUser` cache so authenticated requests skip Postgres.
//! Postgres stays the source of truth; entries expire after a short TTL and are
//! deleted whenever a user's role, store or status changes.

use std::time::Duration;

use deadpool_redis::{Pool, redis::AsyncCommands};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{auth::Role, error::AppError, models::user::User};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionUser {
    pub user_id: Uuid,
    pub role: Role,
    pub store_id: Option<Uuid>,
    pub is_active: bool,
}

impl From<&User> for SessionUser {
    fn from(u: &User) -> Self {
        Self {
            user_id: u.id,
            role: u.role,
            store_id: u.store_id,
            is_active: u.is_active,
        }
    }
}

fn key(firebase_uid: &str) -> String {
    format!("session:v1:{firebase_uid}")
}

pub async fn get(redis: &Pool, firebase_uid: &str) -> Result<Option<SessionUser>, AppError> {
    let mut conn = redis.get().await?;
    let raw: Option<String> = conn.get(key(firebase_uid)).await?;
    // A value we can't parse (e.g. after a struct change) is treated as a miss.
    Ok(raw.and_then(|s| serde_json::from_str(&s).ok()))
}

pub async fn set(
    redis: &Pool,
    firebase_uid: &str,
    session: &SessionUser,
    ttl: Duration,
) -> Result<(), AppError> {
    let mut conn = redis.get().await?;
    let value = serde_json::to_string(session).map_err(anyhow::Error::from)?;
    let () = conn
        .set_ex(key(firebase_uid), value, ttl.as_secs().max(1))
        .await?;
    Ok(())
}

pub async fn invalidate(redis: &Pool, firebase_uid: &str) -> Result<(), AppError> {
    let mut conn = redis.get().await?;
    let _: i64 = conn.del(key(firebase_uid)).await?;
    Ok(())
}
