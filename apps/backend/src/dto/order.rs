use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;
use validator::{Validate, ValidationError};

use super::geo::LatLng;
use crate::models::order::{AddressSnapshot, OrderStatus, PaymentMethod, PaymentStatus};

fn valid_pincode(p: &str) -> Result<(), ValidationError> {
    let ok = p.len() == 6 && p.bytes().all(|b| b.is_ascii_digit()) && !p.starts_with('0');
    ok.then_some(()).ok_or_else(|| {
        ValidationError::new("pincode").with_message("must be a 6-digit Indian PIN code".into())
    })
}

// ---------- addresses ----------

#[derive(Debug, Deserialize, Validate, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct AddressRequest {
    #[validate(length(min = 1, max = 30))]
    pub label: String,
    /// House / flat / floor.
    #[validate(length(min = 1, max = 200))]
    pub line1: String,
    #[validate(length(max = 200))]
    #[serde(default)]
    #[ts(optional)]
    pub line2: Option<String>,
    #[validate(length(max = 100))]
    #[serde(default)]
    #[ts(optional)]
    pub landmark: Option<String>,
    #[validate(length(min = 1, max = 80))]
    pub city: String,
    #[validate(custom(function = "valid_pincode"))]
    pub pincode: String,
    #[validate(nested)]
    pub location: LatLng,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AddressDto {
    pub id: Uuid,
    pub label: String,
    pub line1: String,
    pub line2: Option<String>,
    pub landmark: Option<String>,
    pub city: String,
    pub pincode: String,
    pub location: LatLng,
    /// Store delivering to this address right now, if any.
    pub serving_store_id: Option<Uuid>,
}

// ---------- cart ----------

#[derive(Debug, Deserialize, Validate, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CartQuery {
    pub store_id: Uuid,
}

/// `PUT /v1/customer/cart/items/{productId}`; quantity 0 removes the line.
#[derive(Debug, Deserialize, Validate, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct SetCartItemRequest {
    pub store_id: Uuid,
    #[validate(range(min = 0, max = 10, message = "must be 0-10"))]
    pub quantity: i32,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CartLine {
    pub product_id: Uuid,
    pub name: String,
    pub brand: Option<String>,
    pub unit_label: String,
    pub image_url: Option<String>,
    pub quantity: i32,
    #[ts(type = "number")]
    pub unit_price_paise: i64,
    #[ts(type = "number")]
    pub unit_mrp_paise: i64,
    #[ts(type = "number")]
    pub line_total_paise: i64,
    /// False when the store stopped selling it or it's out of stock.
    pub available: bool,
    pub max_quantity: i32,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BillSummary {
    #[ts(type = "number")]
    pub item_total_paise: i64,
    #[ts(type = "number")]
    pub mrp_total_paise: i64,
    #[ts(type = "number")]
    pub delivery_fee_paise: i64,
    #[ts(type = "number")]
    pub total_paise: i64,
    /// Orders at or above this item total ship free.
    #[ts(type = "number")]
    pub free_delivery_above_paise: i64,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CartResponse {
    pub store_id: Uuid,
    pub items: Vec<CartLine>,
    pub item_count: i32,
    /// Bill for the available lines only.
    pub bill: BillSummary,
    /// True when every line can be ordered as is.
    pub can_checkout: bool,
}

// ---------- checkout & orders ----------

#[derive(Debug, Deserialize, Validate, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct CheckoutRequest {
    pub store_id: Uuid,
    pub address_id: Uuid,
    pub payment_method: PaymentMethod,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RazorpayCheckout {
    pub key_id: String,
    pub gateway_order_id: String,
    #[ts(type = "number")]
    pub amount_paise: i64,
    pub currency: String,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CheckoutResponse {
    pub order: OrderDetail,
    /// Present for `ONLINE`: open the Razorpay checkout with it.
    pub razorpay: Option<RazorpayCheckout>,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OrderSummary {
    pub id: Uuid,
    /// Display number, e.g. `VG100001`.
    pub number: String,
    pub status: OrderStatus,
    pub payment_method: PaymentMethod,
    pub payment_status: PaymentStatus,
    #[ts(type = "number")]
    pub total_paise: i64,
    pub item_count: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OrderItemDto {
    pub product_id: Uuid,
    pub name: String,
    pub brand: Option<String>,
    pub unit_label: String,
    pub image_url: Option<String>,
    pub quantity: i32,
    #[ts(type = "number")]
    pub unit_price_paise: i64,
    #[ts(type = "number")]
    pub unit_mrp_paise: i64,
    pub picked_quantity: Option<i32>,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OrderEvent {
    pub status: OrderStatus,
    pub note: Option<String>,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OrderDetail {
    #[serde(flatten)]
    #[ts(flatten)]
    pub summary: OrderSummary,
    pub store_id: Uuid,
    pub items: Vec<OrderItemDto>,
    pub bill: BillSummary,
    pub address: AddressSnapshot,
    /// Give this to the rider at the door. Hidden once the order is closed.
    pub delivery_otp: Option<String>,
    pub cancel_reason: Option<String>,
    pub can_cancel: bool,
    pub events: Vec<OrderEvent>,
}

#[derive(Debug, Deserialize, Validate, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct CancelOrderRequest {
    #[validate(length(max = 200))]
    #[serde(default)]
    #[ts(optional)]
    pub reason: Option<String>,
}

/// Pushed on the `orders:{id}` and `stores:{storeId}:orders` Redis channels.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OrderStatusChanged {
    pub order_id: Uuid,
    pub store_id: Uuid,
    pub number: String,
    pub from: Option<OrderStatus>,
    pub status: OrderStatus,
    pub at: DateTime<Utc>,
}

pub fn display_number(number: i64) -> String {
    format!("VG{number}")
}
