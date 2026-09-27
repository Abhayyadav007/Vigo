use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, TS)]
#[sqlx(type_name = "vehicle_type", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export)]
pub enum VehicleType {
    Bicycle,
    Scooter,
    Motorcycle,
    EvScooter,
}

#[derive(Debug, Clone)]
pub struct RiderProfile {
    pub user_id: Uuid,
    pub vehicle_type: VehicleType,
    pub vehicle_number: Option<String>,
    pub is_online: bool,
}

#[derive(Debug, Clone)]
pub struct Delivery {
    pub id: Uuid,
    pub order_id: Uuid,
    pub rider_id: Uuid,
    pub store_id: Uuid,
    pub assigned_at: DateTime<Utc>,
    pub picked_up_at: Option<DateTime<Utc>>,
    pub otp_attempts: i32,
}

/// Everything a rider needs to decide on / carry out a delivery.
#[derive(Debug, Clone)]
pub struct DeliveryInfo {
    pub order_id: Uuid,
    pub number: i64,
    pub store_id: Uuid,
    pub store_name: String,
    pub store_address: String,
    pub store_lat: f64,
    pub store_lng: f64,
    pub drop_lat: f64,
    pub drop_lng: f64,
    pub address: sqlx::types::Json<crate::models::order::AddressSnapshot>,
    pub customer_phone: String,
    pub item_count: i32,
    pub bag_count: Option<i32>,
    pub staging_slot: Option<String>,
    pub payment_method: crate::models::order::PaymentMethod,
    pub payment_status: crate::models::order::PaymentStatus,
    pub total_paise: i64,
    pub status: crate::models::order::OrderStatus,
}
