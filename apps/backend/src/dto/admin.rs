use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;
use validator::Validate;

use crate::{auth::Role, models::user::User};

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AdminUser {
    pub id: Uuid,
    pub phone: String,
    pub name: Option<String>,
    pub role: Role,
    pub store_id: Option<Uuid>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<User> for AdminUser {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            phone: u.phone,
            name: u.name,
            role: u.role,
            store_id: u.store_id,
            is_active: u.is_active,
            created_at: u.created_at,
            updated_at: u.updated_at,
        }
    }
}

/// `GET /v1/admin/users` filters (plus `limit`/`offset`).
#[derive(Debug, Deserialize, Validate, TS)]
#[ts(export)]
pub struct UserListQuery {
    /// Substring of the phone number, e.g. `98765`.
    #[validate(length(min = 1, max = 13), custom(function = "phone_fragment"))]
    #[ts(optional)]
    pub phone: Option<String>,
    #[ts(optional)]
    pub role: Option<Role>,
}

fn phone_fragment(s: &str) -> Result<(), validator::ValidationError> {
    if s.bytes().all(|b| b.is_ascii_digit() || b == b'+') {
        Ok(())
    } else {
        Err(validator::ValidationError::new("phone")
            .with_message("may only contain digits and +".into()))
    }
}

/// `PATCH /v1/admin/users/{id}/role`
#[derive(Debug, Deserialize, Validate, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct UpdateRoleRequest {
    pub role: Role,
    #[serde(default)]
    #[ts(optional)]
    pub store_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, Validate, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StoreFilter {
    #[ts(optional)]
    pub store_id: Option<Uuid>,
}

/// One card on the admin live board.
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BoardOrder {
    pub id: Uuid,
    pub number: String,
    pub status: crate::models::order::OrderStatus,
    pub store_code: String,
    pub payment_method: crate::models::order::PaymentMethod,
    #[ts(type = "number")]
    pub total_paise: i64,
    pub item_count: i32,
    pub rider_phone: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Metrics {
    #[ts(type = "number")]
    pub orders_today: i64,
    #[ts(type = "number")]
    pub delivered_today: i64,
    #[ts(type = "number")]
    pub cancelled_today: i64,
    /// Delivered order value today.
    #[ts(type = "number")]
    pub gmv_today_paise: i64,
    /// Order placed -> delivered, averaged over today's deliveries.
    pub avg_delivery_minutes: Option<f64>,
    #[ts(type = "number")]
    pub riders_online: i64,
}
