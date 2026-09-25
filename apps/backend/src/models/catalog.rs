use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Category {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub slug: String,
    pub image_url: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
    pub product_count: i64,
}

#[derive(Debug, Clone)]
pub struct Product {
    pub id: Uuid,
    pub category_id: Uuid,
    pub category_name: String,
    pub name: String,
    pub slug: String,
    pub brand: Option<String>,
    pub description: Option<String>,
    pub unit_label: String,
    pub barcode: Option<String>,
    pub mrp_paise: i64,
    pub price_paise: i64,
    pub image_urls: Vec<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A product joined with one store's (possibly missing) inventory row.
#[derive(Debug, Clone)]
pub struct InventoryRow {
    pub product_id: Uuid,
    pub product_name: String,
    pub brand: Option<String>,
    pub unit_label: String,
    pub barcode: Option<String>,
    pub image_urls: Vec<String>,
    pub category_name: String,
    pub mrp_paise: i64,
    pub price_paise: i64,
    pub stocked: bool,
    pub quantity: i32,
    pub bin_location: Option<String>,
    pub price_override_paise: Option<i64>,
    pub is_available: bool,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct CatalogCategoryRow {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub image_url: Option<String>,
    pub product_count: i64,
}

/// A product as a customer of one store sees it (effective price, stock).
#[derive(Debug, Clone)]
pub struct CatalogProductRow {
    pub id: Uuid,
    pub category_id: Uuid,
    pub name: String,
    pub brand: Option<String>,
    pub unit_label: String,
    pub image_urls: Vec<String>,
    pub mrp_paise: i64,
    pub price_paise: i64,
    pub quantity: i32,
    pub description: Option<String>,
}
