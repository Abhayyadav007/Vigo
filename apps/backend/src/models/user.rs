use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::auth::Role;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub firebase_uid: String,
    pub phone: String,
    pub name: Option<String>,
    pub role: Role,
    pub store_id: Option<Uuid>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
