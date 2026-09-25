//! Admin catalog management DTOs (categories, products, inventory).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;
use validator::{Validate, ValidationError};

use crate::models::catalog::{Category, InventoryRow, Product};

fn valid_slug(s: &str) -> Result<(), ValidationError> {
    let ok = !s.is_empty()
        && s.len() <= 120
        && !s.starts_with('-')
        && !s.ends_with('-')
        && !s.contains("--")
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
    ok.then_some(()).ok_or_else(|| {
        ValidationError::new("slug").with_message("must be lowercase words joined by -".into())
    })
}

fn valid_barcode(s: &str) -> Result<(), ValidationError> {
    let ok = (8..=14).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit());
    ok.then_some(()).ok_or_else(|| {
        ValidationError::new("barcode").with_message("must be 8-14 digits (EAN/UPC)".into())
    })
}

fn valid_media_urls(urls: &[String]) -> Result<(), ValidationError> {
    let ok = urls.len() <= 8 && urls.iter().all(|u| is_media_url(u));
    ok.then_some(()).ok_or_else(|| {
        ValidationError::new("image_urls")
            .with_message("up to 8 URLs from /v1/admin/uploads or https://".into())
    })
}

fn valid_media_url(u: &str) -> Result<(), ValidationError> {
    valid_media_urls(std::slice::from_ref(&u.to_owned()))
}

fn is_media_url(u: &str) -> bool {
    u.len() <= 500 && (u.starts_with("/media/") || u.starts_with("https://"))
}

// ---------- categories ----------

/// `POST /v1/admin/categories`, `PUT /v1/admin/categories/{id}`.
#[derive(Debug, Deserialize, Validate, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct CategoryRequest {
    #[validate(length(min = 1, max = 80))]
    pub name: String,
    /// Derived from `name` when omitted.
    #[validate(custom(function = "valid_slug"))]
    #[serde(default)]
    #[ts(optional)]
    pub slug: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub parent_id: Option<Uuid>,
    #[validate(custom(function = "valid_media_url"))]
    #[serde(default)]
    #[ts(optional)]
    pub image_url: Option<String>,
    #[serde(default)]
    #[validate(range(min = -10000, max = 10000))]
    pub sort_order: i32,
    pub is_active: bool,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AdminCategory {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub slug: String,
    pub image_url: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
    #[ts(type = "number")]
    pub product_count: i64,
}

impl From<Category> for AdminCategory {
    fn from(c: Category) -> Self {
        Self {
            id: c.id,
            parent_id: c.parent_id,
            name: c.name,
            slug: c.slug,
            image_url: c.image_url,
            sort_order: c.sort_order,
            is_active: c.is_active,
            product_count: c.product_count,
        }
    }
}

// ---------- products ----------

/// `POST /v1/admin/products`, `PUT /v1/admin/products/{id}`. Prices in paise.
#[derive(Debug, Deserialize, Validate, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct ProductRequest {
    pub category_id: Uuid,
    #[validate(length(min = 1, max = 150))]
    pub name: String,
    /// Derived from brand + name + unit when omitted.
    #[validate(custom(function = "valid_slug"))]
    #[serde(default)]
    #[ts(optional)]
    pub slug: Option<String>,
    #[validate(length(min = 1, max = 80))]
    #[serde(default)]
    #[ts(optional)]
    pub brand: Option<String>,
    #[validate(length(max = 2000))]
    #[serde(default)]
    #[ts(optional)]
    pub description: Option<String>,
    /// Pack size, e.g. "500 g", "1 L".
    #[validate(length(min = 1, max = 40))]
    pub unit_label: String,
    #[validate(custom(function = "valid_barcode"))]
    #[serde(default)]
    #[ts(optional)]
    pub barcode: Option<String>,
    #[validate(range(min = 1, max = 100_000_000, message = "must be ₹0.01 to ₹10,00,000"))]
    #[ts(type = "number")]
    pub mrp_paise: i64,
    #[validate(range(min = 1, max = 100_000_000, message = "must be ₹0.01 to ₹10,00,000"))]
    #[ts(type = "number")]
    pub price_paise: i64,
    #[validate(custom(function = "valid_media_urls"))]
    #[serde(default)]
    pub image_urls: Vec<String>,
    pub is_active: bool,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AdminProduct {
    pub id: Uuid,
    pub category_id: Uuid,
    pub category_name: String,
    pub name: String,
    pub slug: String,
    pub brand: Option<String>,
    pub description: Option<String>,
    pub unit_label: String,
    pub barcode: Option<String>,
    #[ts(type = "number")]
    pub mrp_paise: i64,
    #[ts(type = "number")]
    pub price_paise: i64,
    pub image_urls: Vec<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Product> for AdminProduct {
    fn from(p: Product) -> Self {
        Self {
            id: p.id,
            category_id: p.category_id,
            category_name: p.category_name,
            name: p.name,
            slug: p.slug,
            brand: p.brand,
            description: p.description,
            unit_label: p.unit_label,
            barcode: p.barcode,
            mrp_paise: p.mrp_paise,
            price_paise: p.price_paise,
            image_urls: p.image_urls,
            is_active: p.is_active,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

#[derive(Debug, Deserialize, Validate, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProductListQuery {
    /// Fuzzy match on name/brand, or exact barcode.
    #[validate(length(min = 1, max = 100))]
    #[ts(optional)]
    pub q: Option<String>,
    #[ts(optional)]
    pub category_id: Option<Uuid>,
    #[ts(optional)]
    pub is_active: Option<bool>,
}

// ---------- inventory ----------

/// `PUT /v1/admin/stores/{storeId}/inventory/{productId}` (upsert).
#[derive(Debug, Deserialize, Validate, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct InventoryRequest {
    #[validate(range(min = 0, max = 1_000_000))]
    pub quantity: i32,
    /// Shelf address, e.g. "A-03-2".
    #[validate(length(min = 1, max = 32))]
    #[serde(default)]
    #[ts(optional)]
    pub bin_location: Option<String>,
    /// Store-specific price; omit to use the product's base price.
    #[validate(range(min = 1, max = 100_000_000))]
    #[serde(default)]
    #[ts(optional, type = "number")]
    pub price_override_paise: Option<i64>,
    pub is_available: bool,
}

/// A product as seen from one store; `stocked` is false when the store has no
/// inventory row for it yet (then the inventory fields are defaults).
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct InventoryItem {
    pub product_id: Uuid,
    pub product_name: String,
    pub brand: Option<String>,
    pub unit_label: String,
    pub barcode: Option<String>,
    pub image_url: Option<String>,
    pub category_name: String,
    #[ts(type = "number")]
    pub mrp_paise: i64,
    #[ts(type = "number")]
    pub base_price_paise: i64,
    pub stocked: bool,
    pub quantity: i32,
    pub bin_location: Option<String>,
    #[ts(type = "number | null")]
    pub price_override_paise: Option<i64>,
    pub is_available: bool,
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<InventoryRow> for InventoryItem {
    fn from(r: InventoryRow) -> Self {
        Self {
            product_id: r.product_id,
            product_name: r.product_name,
            brand: r.brand,
            unit_label: r.unit_label,
            barcode: r.barcode,
            image_url: r.image_urls.into_iter().next(),
            category_name: r.category_name,
            mrp_paise: r.mrp_paise,
            base_price_paise: r.price_paise,
            stocked: r.stocked,
            quantity: r.quantity,
            bin_location: r.bin_location,
            price_override_paise: r.price_override_paise,
            is_available: r.is_available,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Debug, Deserialize, Validate, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct InventoryListQuery {
    #[validate(length(min = 1, max = 100))]
    #[ts(optional)]
    pub q: Option<String>,
    /// `true`: only products this store carries. `false`: only ones it doesn't.
    #[ts(optional)]
    pub stocked: Option<bool>,
}

// ---------- uploads ----------

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct UploadResponse {
    /// Relative (`/media/...`) for local storage; resolve against the API origin.
    pub url: String,
}
