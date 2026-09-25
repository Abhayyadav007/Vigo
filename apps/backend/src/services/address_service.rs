use uuid::Uuid;

use crate::{
    dto::{
        geo::LatLng,
        order::{AddressDto, AddressRequest},
    },
    error::{AppError, AppResult},
    models::order::Address,
    repositories::{
        addresses::{self, AddressInput},
        stores,
    },
    state::AppState,
};

/// Customers keep a handful of addresses; this stops abuse.
const MAX_ADDRESSES: usize = 20;

fn opt(s: &Option<String>) -> Option<&str> {
    s.as_deref().map(str::trim).filter(|s| !s.is_empty())
}

fn input(req: &AddressRequest) -> AddressInput<'_> {
    AddressInput {
        label: req.label.trim(),
        line1: req.line1.trim(),
        line2: opt(&req.line2),
        landmark: opt(&req.landmark),
        city: req.city.trim(),
        pincode: &req.pincode,
        lat: req.location.lat,
        lng: req.location.lng,
    }
}

async fn to_dto(state: &AppState, a: Address) -> AppResult<AddressDto> {
    let serving = stores::find_serving(&state.db, a.lat, a.lng).await?;
    Ok(dto(a, serving.map(|s| s.id)))
}

fn dto(a: Address, serving_store_id: Option<Uuid>) -> AddressDto {
    AddressDto {
        id: a.id,
        label: a.label,
        line1: a.line1,
        line2: a.line2,
        landmark: a.landmark,
        city: a.city,
        pincode: a.pincode,
        location: LatLng {
            lat: a.lat,
            lng: a.lng,
        },
        serving_store_id,
    }
}

pub async fn list(state: &AppState, user_id: Uuid) -> AppResult<Vec<AddressDto>> {
    let rows = addresses::list_with_serving_store(&state.db, user_id).await?;
    Ok(rows.into_iter().map(|(a, s)| dto(a, s)).collect())
}

pub async fn create(
    state: &AppState,
    user_id: Uuid,
    req: &AddressRequest,
) -> AppResult<AddressDto> {
    if addresses::list_with_serving_store(&state.db, user_id)
        .await?
        .len()
        >= MAX_ADDRESSES
    {
        return Err(AppError::Validation(format!(
            "you can save up to {MAX_ADDRESSES} addresses"
        )));
    }
    let a = addresses::create(&state.db, user_id, &input(req)).await?;
    to_dto(state, a).await
}

pub async fn update(
    state: &AppState,
    user_id: Uuid,
    id: Uuid,
    req: &AddressRequest,
) -> AppResult<AddressDto> {
    let a = addresses::update(&state.db, user_id, id, &input(req))
        .await?
        .ok_or(AppError::NotFound("address"))?;
    to_dto(state, a).await
}

pub async fn delete(state: &AppState, user_id: Uuid, id: Uuid) -> AppResult<()> {
    if addresses::delete(&state.db, user_id, id).await? {
        Ok(())
    } else {
        Err(AppError::NotFound("address"))
    }
}
