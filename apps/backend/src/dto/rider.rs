use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;
use validator::{Validate, ValidationError};

use super::geo::LatLng;
use crate::models::{
    order::{AddressSnapshot, OrderStatus, PaymentMethod},
    rider::VehicleType,
};

fn valid_vehicle_number(s: &str) -> Result<(), ValidationError> {
    // KA01AB1234, DL3CAB1234, MH12AB123 ...
    let b = s.as_bytes();
    let ok = (6..=11).contains(&b.len())
        && b[..2].iter().all(u8::is_ascii_uppercase)
        && b[2].is_ascii_digit()
        && b.iter()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        && b.last().is_some_and(u8::is_ascii_digit);
    ok.then_some(()).ok_or_else(|| {
        ValidationError::new("vehicle_number").with_message("must look like KA01AB1234".into())
    })
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RiderMe {
    pub is_online: bool,
    pub vehicle_type: VehicleType,
    pub vehicle_number: Option<String>,
    pub store_id: Option<Uuid>,
    pub store_name: Option<String>,
    /// The order the rider is currently delivering, if any.
    pub active_order_id: Option<Uuid>,
    pub delivered_today: i32,
}

#[derive(Debug, Deserialize, Validate, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct RiderProfileRequest {
    pub vehicle_type: VehicleType,
    #[validate(custom(function = "valid_vehicle_number"))]
    #[serde(default)]
    #[ts(optional)]
    pub vehicle_number: Option<String>,
}

#[derive(Debug, Deserialize, Validate, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct RiderStatusRequest {
    pub online: bool,
}

#[derive(Debug, Serialize, Deserialize, Validate, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct LocationPoint {
    #[validate(range(min = -90.0, max = 90.0))]
    pub lat: f64,
    #[validate(range(min = -180.0, max = 180.0))]
    pub lng: f64,
    #[serde(default)]
    #[ts(optional)]
    pub accuracy_m: Option<f64>,
    /// Unix milliseconds when the device took the fix.
    #[ts(type = "number")]
    pub recorded_at: i64,
}

/// Background location uploads come in batches; the newest point wins.
#[derive(Debug, Deserialize, Validate, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct LocationBatch {
    #[validate(length(min = 1, max = 100), nested)]
    pub points: Vec<LocationPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DeliveryOffer {
    pub order_id: Uuid,
    pub number: String,
    pub store_name: String,
    pub store_address: String,
    pub store_location: LatLng,
    /// Drop area only (full address after accepting).
    pub drop_area: String,
    pub drop_location: LatLng,
    /// Rider -> store, metres (from the rider's last fix).
    pub to_store_m: Option<f64>,
    /// Store -> customer, metres (straight line).
    pub trip_m: f64,
    pub item_count: i32,
    pub bag_count: Option<i32>,
    /// Cash to collect at the door (0 if prepaid).
    #[ts(type = "number")]
    pub collect_paise: i64,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ActiveDelivery {
    pub order_id: Uuid,
    pub number: String,
    /// RIDER_ASSIGNED (go to store), PICKED_UP / OUT_FOR_DELIVERY (go to customer).
    pub status: OrderStatus,
    pub store_name: String,
    pub store_address: String,
    pub store_location: LatLng,
    pub drop: AddressSnapshot,
    // TODO(prod): number masking via a calling provider instead of the raw phone.
    pub customer_phone: String,
    pub item_count: i32,
    pub bag_count: Option<i32>,
    pub staging_slot: Option<String>,
    pub payment_method: PaymentMethod,
    #[ts(type = "number")]
    pub collect_paise: i64,
    pub otp_attempts_left: i32,
}

#[derive(Debug, Deserialize, Validate, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct PickupRequest {
    /// Bags the rider collected; must match what the picker packed.
    #[validate(range(min = 1, max = 20))]
    pub bag_count: i32,
}

#[derive(Debug, Deserialize, Validate, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct DeliverRequest {
    #[validate(length(equal = 4))]
    pub otp: String,
    /// For cash on delivery: amount taken from the customer.
    #[serde(default)]
    #[ts(optional, type = "number")]
    pub cod_collected_paise: Option<i64>,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DeliveryHistoryItem {
    pub order_id: Uuid,
    pub number: String,
    pub status: OrderStatus,
    pub assigned_at: DateTime<Utc>,
    pub delivered_at: Option<DateTime<Utc>>,
    #[ts(type = "number | null")]
    pub cod_collected_paise: Option<i64>,
}

/// Shown to the customer once a rider is assigned.
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AssignedRider {
    pub name: Option<String>,
    pub phone: String,
    pub vehicle_type: VehicleType,
    pub vehicle_number: Option<String>,
    /// Last known position; live updates follow on `/v1/ws/orders/{id}`.
    pub location: Option<LatLng>,
}

/// Pushed on `orders:{id}` (as `riderLocation`) while the order is on its way.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RiderLocation {
    pub order_id: Uuid,
    pub lat: f64,
    pub lng: f64,
    pub at: DateTime<Utc>,
}
