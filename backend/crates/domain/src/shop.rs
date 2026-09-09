use serde::Serialize;
use uuid::Uuid;

/// A shop that can serve a given customer location right now.
#[derive(Debug, Clone, Serialize)]
pub struct NearbyShop {
    pub id: Uuid,
    pub name: String,
    /// Straight-line distance from the customer, in metres.
    pub metres: f64,
    /// Measured preparation time, not a declared one.
    pub prep_seconds: i32,
    pub acceptance_rate: f64,
    /// Prep + travel. The number the customer actually sees.
    pub eta_minutes: f64,
    /// Distinct products currently in stock and available.
    pub item_count: i64,
}
