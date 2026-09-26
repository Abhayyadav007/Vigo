use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;
use validator::Validate;

use crate::models::order::OrderStatus;

/// One order in the store's picking queue.
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PickerQueueItem {
    pub id: Uuid,
    pub number: String,
    /// CONFIRMED (waiting), PICKING, or PACKED (waiting for a rider).
    pub status: OrderStatus,
    pub item_count: i32,
    pub line_count: i32,
    /// Lines the picker has counted (found or marked missing).
    pub lines_done: i32,
    pub picker_id: Option<Uuid>,
    pub is_mine: bool,
    pub staging_slot: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PickLine {
    pub product_id: Uuid,
    pub name: String,
    pub brand: Option<String>,
    pub unit_label: String,
    pub image_url: Option<String>,
    pub barcode: Option<String>,
    /// Shelf address, e.g. "A-03-2"; the list is sorted by it.
    pub bin_location: Option<String>,
    pub quantity: i32,
    /// None until counted; less than `quantity` means units are missing.
    pub picked_quantity: Option<i32>,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PickList {
    pub id: Uuid,
    pub number: String,
    pub status: OrderStatus,
    pub picker_id: Option<Uuid>,
    pub is_mine: bool,
    pub lines: Vec<PickLine>,
    /// Every line counted and at least one unit found.
    pub can_pack: bool,
    pub bag_count: Option<i32>,
    pub staging_slot: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct ScanRequest {
    #[validate(length(min = 1, max = 64))]
    pub barcode: String,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ScanResult {
    /// The line the scan counted towards.
    pub line: PickLine,
    /// True when this scan completed the line.
    pub line_complete: bool,
    pub pick_list: PickList,
}

/// Manually set a line's count: for unlabelled items, or to mark units missing.
#[derive(Debug, Deserialize, Validate, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct SetPickedRequest {
    #[validate(range(min = 0, max = 10))]
    pub picked_quantity: i32,
}

#[derive(Debug, Deserialize, Validate, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct PackRequest {
    #[validate(range(min = 1, max = 20))]
    pub bag_count: i32,
    /// Where the bags wait for the rider, e.g. "S-03".
    #[validate(length(min = 1, max = 16))]
    #[serde(default)]
    #[ts(optional)]
    pub staging_slot: Option<String>,
}

#[derive(Debug, Deserialize, Validate, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct PickerCancelRequest {
    #[validate(length(min = 1, max = 200))]
    pub reason: String,
}
