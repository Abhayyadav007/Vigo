//! Customer-facing catalog DTOs. Always scoped to one dark store.

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;
use validator::Validate;

use crate::models::catalog::{CatalogCategoryRow, CatalogProductRow};

/// Most units of one product a single order may contain.
pub const MAX_PER_ITEM: i32 = 10;

#[derive(Debug, Deserialize, Validate, TS)]
#[ts(export)]
pub struct ServiceabilityQuery {
    #[validate(range(min = -90.0, max = 90.0))]
    pub lat: f64,
    #[validate(range(min = -180.0, max = 180.0))]
    pub lng: f64,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StoreSummary {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    /// Rough delivery estimate from store distance.
    pub eta_minutes: i32,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ServiceabilityResponse {
    pub serviceable: bool,
    pub store: Option<StoreSummary>,
}

#[derive(Debug, Deserialize, Validate, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StoreScope {
    /// From `GET /v1/customer/serviceability`.
    pub store_id: Uuid,
}

#[derive(Debug, Deserialize, Validate, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CatalogProductQuery {
    pub store_id: Uuid,
    #[ts(optional)]
    pub category_id: Option<Uuid>,
    #[validate(length(min = 1, max = 100))]
    #[ts(optional)]
    pub q: Option<String>,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CatalogCategory {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub image_url: Option<String>,
    #[ts(type = "number")]
    pub product_count: i64,
}

impl From<CatalogCategoryRow> for CatalogCategory {
    fn from(r: CatalogCategoryRow) -> Self {
        Self {
            id: r.id,
            name: r.name,
            slug: r.slug,
            image_url: r.image_url,
            product_count: r.product_count,
        }
    }
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CatalogProduct {
    pub id: Uuid,
    pub category_id: Uuid,
    pub name: String,
    pub brand: Option<String>,
    pub unit_label: String,
    pub image_urls: Vec<String>,
    #[ts(type = "number")]
    pub mrp_paise: i64,
    /// What the customer pays at this store (store override or base price).
    #[ts(type = "number")]
    pub price_paise: i64,
    pub in_stock: bool,
    /// Cap for the quantity stepper: min(stock, per-item limit).
    pub max_quantity: i32,
    /// Only on the product detail endpoint.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub description: Option<String>,
}

impl From<CatalogProductRow> for CatalogProduct {
    fn from(r: CatalogProductRow) -> Self {
        Self {
            id: r.id,
            category_id: r.category_id,
            name: r.name,
            brand: r.brand,
            unit_label: r.unit_label,
            image_urls: r.image_urls,
            mrp_paise: r.mrp_paise,
            price_paise: r.price_paise,
            in_stock: r.quantity > 0,
            max_quantity: r.quantity.clamp(0, MAX_PER_ITEM),
            description: r.description,
        }
    }
}
