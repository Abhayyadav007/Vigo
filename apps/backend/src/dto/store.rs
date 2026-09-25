use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;
use validator::Validate;

use super::geo::{GeoJsonPolygon, LatLng};
use crate::models::store::DarkStore;

/// `POST /v1/admin/stores`, `PUT /v1/admin/stores/{id}` (full replace).
#[derive(Debug, Deserialize, Validate, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct StoreRequest {
    /// e.g. `BLR-IND-01`
    #[validate(custom(function = "store_code"))]
    pub code: String,
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    #[validate(length(min = 1, max = 300))]
    pub address: String,
    #[validate(nested)]
    pub location: LatLng,
    pub service_area: GeoJsonPolygon,
    pub is_active: bool,
}

fn store_code(code: &str) -> Result<(), validator::ValidationError> {
    let ok = (2..=32).contains(&code.len())
        && code
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'-');
    if ok {
        Ok(())
    } else {
        Err(validator::ValidationError::new("code")
            .with_message("must be 2-32 characters of A-Z, 0-9 and -".into()))
    }
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AdminStore {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub address: String,
    pub location: LatLng,
    pub service_area: GeoJsonPolygon,
    /// Square kilometres covered by `service_area`.
    pub area_sq_km: f64,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<DarkStore> for AdminStore {
    fn from(s: DarkStore) -> Self {
        Self {
            id: s.id,
            code: s.code,
            name: s.name,
            address: s.address,
            location: LatLng {
                lat: s.lat,
                lng: s.lng,
            },
            service_area: s.service_area.0,
            area_sq_km: (s.area_sq_m / 1_000_000.0 * 100.0).round() / 100.0,
            is_active: s.is_active,
            created_at: s.created_at,
            updated_at: s.updated_at,
        }
    }
}

#[derive(Debug, Deserialize, Validate, TS)]
#[ts(export)]
pub struct StoreListQuery {
    /// Matches name, code or address.
    #[validate(length(min = 1, max = 100))]
    #[ts(optional)]
    pub q: Option<String>,
}
