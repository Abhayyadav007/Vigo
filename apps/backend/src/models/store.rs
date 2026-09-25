use chrono::{DateTime, Utc};
use sqlx::types::Json;
use uuid::Uuid;

use crate::dto::geo::GeoJsonPolygon;

/// `dark_stores` row with PostGIS columns converted in SQL.
#[derive(Debug, Clone)]
pub struct DarkStore {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub address: String,
    pub lat: f64,
    pub lng: f64,
    pub service_area: Json<GeoJsonPolygon>,
    pub area_sq_m: f64,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// The store serving a customer location.
#[derive(Debug, Clone)]
pub struct ServingStore {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub distance_m: f64,
}
