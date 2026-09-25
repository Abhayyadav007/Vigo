use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, TS)]
#[sqlx(type_name = "order_status", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export)]
pub enum OrderStatus {
    Placed,
    Confirmed,
    Picking,
    Packed,
    RiderAssigned,
    PickedUp,
    OutForDelivery,
    Delivered,
    Cancelled,
    PartiallyFulfilled,
}

impl OrderStatus {
    pub const ALL: [Self; 10] = [
        Self::Placed,
        Self::Confirmed,
        Self::Picking,
        Self::Packed,
        Self::RiderAssigned,
        Self::PickedUp,
        Self::OutForDelivery,
        Self::Delivered,
        Self::Cancelled,
        Self::PartiallyFulfilled,
    ];

    /// The order lifecycle. Every status change goes through
    /// `order_service::transition`, which enforces this table.
    pub fn can_transition_to(self, to: Self) -> bool {
        use OrderStatus::*;
        matches!(
            (self, to),
            (Placed, Confirmed | Cancelled)
                | (Confirmed, Picking | Cancelled)
                | (Picking, Packed | Cancelled)
                | (Packed, RiderAssigned | Cancelled)
                // A rider can drop an assignment before pickup.
                | (RiderAssigned, PickedUp | Packed | Cancelled)
                | (PickedUp, OutForDelivery)
                // Delivered with some items missing (picker shortages) ends partial.
                | (OutForDelivery, Delivered | PartiallyFulfilled)
        )
    }

    /// Statuses from which `to` is reachable in one step.
    pub fn predecessors(to: Self) -> Vec<Self> {
        Self::ALL
            .into_iter()
            .filter(|from| from.can_transition_to(to))
            .collect()
    }

    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Delivered | Self::Cancelled | Self::PartiallyFulfilled
        )
    }

    /// The customer may cancel until picking starts.
    pub fn customer_cancellable(self) -> bool {
        matches!(self, Self::Placed | Self::Confirmed)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Placed => "PLACED",
            Self::Confirmed => "CONFIRMED",
            Self::Picking => "PICKING",
            Self::Packed => "PACKED",
            Self::RiderAssigned => "RIDER_ASSIGNED",
            Self::PickedUp => "PICKED_UP",
            Self::OutForDelivery => "OUT_FOR_DELIVERY",
            Self::Delivered => "DELIVERED",
            Self::Cancelled => "CANCELLED",
            Self::PartiallyFulfilled => "PARTIALLY_FULFILLED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, TS)]
#[sqlx(type_name = "payment_method", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export)]
pub enum PaymentMethod {
    /// Cash (or UPI to the rider) on delivery.
    Cod,
    /// Prepaid via the payment gateway (UPI, cards, netbanking).
    Online,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, TS)]
#[sqlx(type_name = "payment_status", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export)]
pub enum PaymentStatus {
    Pending,
    Paid,
    Failed,
    Refunded,
}

/// Address as captured on the order (JSONB snapshot).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AddressSnapshot {
    pub label: String,
    pub line1: String,
    pub line2: Option<String>,
    pub landmark: Option<String>,
    pub city: String,
    pub pincode: String,
    pub lat: f64,
    pub lng: f64,
}

#[derive(Debug, Clone)]
pub struct Order {
    pub id: Uuid,
    pub number: i64,
    pub user_id: Uuid,
    pub store_id: Uuid,
    pub status: OrderStatus,
    pub payment_method: PaymentMethod,
    pub payment_status: PaymentStatus,
    pub item_total_paise: i64,
    pub mrp_total_paise: i64,
    pub delivery_fee_paise: i64,
    pub total_paise: i64,
    pub address: Json<AddressSnapshot>,
    pub delivery_otp: String,
    pub gateway_order_id: Option<String>,
    pub cancel_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct OrderItem {
    pub product_id: Uuid,
    pub name: String,
    pub brand: Option<String>,
    pub unit_label: String,
    pub image_url: Option<String>,
    pub quantity: i32,
    pub unit_price_paise: i64,
    pub unit_mrp_paise: i64,
    pub picked_quantity: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct OrderEventRow {
    pub to_status: OrderStatus,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Address {
    pub id: Uuid,
    pub user_id: Uuid,
    pub label: String,
    pub line1: String,
    pub line2: Option<String>,
    pub landmark: Option<String>,
    pub city: String,
    pub pincode: String,
    pub lat: f64,
    pub lng: f64,
}

/// A cart line joined with the store's current catalog state.
#[derive(Debug, Clone)]
pub struct CartLineRow {
    pub product_id: Uuid,
    pub name: String,
    pub brand: Option<String>,
    pub unit_label: String,
    pub image_urls: Vec<String>,
    pub quantity: i32,
    pub price_paise: i64,
    pub mrp_paise: i64,
    /// Store stock; 0 when the product is no longer sold here.
    pub stock: i32,
    pub sellable: bool,
}

#[cfg(test)]
mod tests {
    use super::OrderStatus::{self, *};

    #[test]
    fn happy_path_is_allowed() {
        let path = [
            Placed,
            Confirmed,
            Picking,
            Packed,
            RiderAssigned,
            PickedUp,
            OutForDelivery,
            Delivered,
        ];
        for pair in path.windows(2) {
            assert!(
                pair[0].can_transition_to(pair[1]),
                "{:?} -> {:?}",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn terminal_states_are_final_and_no_skipping() {
        for from in [Delivered, Cancelled, PartiallyFulfilled] {
            assert!(from.is_terminal());
            for to in OrderStatus::ALL {
                assert!(!from.can_transition_to(to), "{from:?} -> {to:?}");
            }
        }
        assert!(!Placed.can_transition_to(Delivered));
        assert!(!Confirmed.can_transition_to(Packed));
        assert!(
            !PickedUp.can_transition_to(Cancelled),
            "goods are with the rider"
        );
        assert!(!Placed.can_transition_to(Placed));
    }

    #[test]
    fn predecessors_match_the_table() {
        assert_eq!(OrderStatus::predecessors(Confirmed), vec![Placed]);
        assert_eq!(
            OrderStatus::predecessors(Packed),
            vec![Picking, RiderAssigned]
        );
        let cancel_from = OrderStatus::predecessors(Cancelled);
        assert_eq!(
            cancel_from,
            vec![Placed, Confirmed, Picking, Packed, RiderAssigned]
        );
    }
}
