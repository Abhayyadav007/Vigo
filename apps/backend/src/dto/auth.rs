use chrono::{DateTime, Utc};
use serde::Serialize;
use ts_rs::TS;
use uuid::Uuid;

use crate::{auth::Role, models::user::User};

/// The signed-in user, as returned by `/v1/auth/sync` and `/v1/auth/me`.
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MeResponse {
    pub id: Uuid,
    pub phone: String,
    pub name: Option<String>,
    pub role: Role,
    pub store_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

impl From<User> for MeResponse {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            phone: u.phone,
            name: u.name,
            role: u.role,
            store_id: u.store_id,
            created_at: u.created_at,
        }
    }
}
