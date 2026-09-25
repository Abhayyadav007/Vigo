use uuid::Uuid;

use crate::{
    dto::store::StoreRequest,
    error::{AppError, AppResult, On, map_constraint},
    models::store::DarkStore,
    repositories::stores::{self, StoreInput},
    state::AppState,
};

/// Quick commerce serves a few km around a store; anything bigger is a typo.
const MAX_AREA_SQ_KM: f64 = 150.0;
const MIN_AREA_SQ_KM: f64 = 0.05;

const CONSTRAINTS: &[(&str, On, &str)] = &[(
    "dark_stores_code_key",
    On::Conflict,
    "a store with this code already exists",
)];

pub async fn create(state: &AppState, req: StoreRequest) -> AppResult<DarkStore> {
    let input = validated(state, &req).await?;
    stores::create(&state.db, &input.as_input(&req))
        .await
        .map_err(|e| map_constraint(e, CONSTRAINTS))
}

pub async fn update(state: &AppState, id: Uuid, req: StoreRequest) -> AppResult<DarkStore> {
    let input = validated(state, &req).await?;
    stores::update(&state.db, id, &input.as_input(&req))
        .await
        .map_err(|e| map_constraint(e, CONSTRAINTS))?
        .ok_or(AppError::NotFound("store"))
}

struct Validated {
    polygon: crate::dto::geo::GeoJsonPolygon,
}

impl Validated {
    fn as_input<'a>(&'a self, req: &'a StoreRequest) -> StoreInput<'a> {
        StoreInput {
            code: &req.code,
            name: req.name.trim(),
            address: req.address.trim(),
            lat: req.location.lat,
            lng: req.location.lng,
            service_area: &self.polygon,
            is_active: req.is_active,
        }
    }
}

async fn validated(state: &AppState, req: &StoreRequest) -> AppResult<Validated> {
    let polygon = req
        .service_area
        .clone()
        .normalized()
        .map_err(AppError::Validation)?;
    let check = stores::check_polygon(&state.db, &polygon).await?;
    if !check.valid {
        return Err(AppError::Validation(format!(
            "service area is not a valid polygon: {}",
            check.reason
        )));
    }
    let sq_km = check.area_sq_m / 1_000_000.0;
    if !(MIN_AREA_SQ_KM..=MAX_AREA_SQ_KM).contains(&sq_km) {
        return Err(AppError::Validation(format!(
            "service area is {sq_km:.2} km²; expected {MIN_AREA_SQ_KM}–{MAX_AREA_SQ_KM} km²"
        )));
    }
    Ok(Validated { polygon })
}

/// Delivery estimate: fixed pick/pack time plus riding at ~18 km/h.
pub fn eta_minutes(distance_m: f64) -> i32 {
    const PREP_MINUTES: f64 = 4.0;
    const METRES_PER_MINUTE: f64 = 300.0;
    let eta = (PREP_MINUTES + distance_m / METRES_PER_MINUTE).ceil();
    // Bounded input (service areas are ≤150 km²), so the cast can't truncate.
    eta.clamp(8.0, 60.0) as i32
}

#[cfg(test)]
mod tests {
    use super::eta_minutes;

    #[test]
    fn eta_is_bounded() {
        assert_eq!(eta_minutes(0.0), 8);
        assert_eq!(eta_minutes(1_800.0), 10);
        assert_eq!(eta_minutes(1_000_000.0), 60);
    }
}
