//! The rider's side: availability, location, pickup and delivery.

use axum::http::StatusCode;
use chrono::Utc;
use uuid::Uuid;

use crate::{
    cache::{events, geo},
    dto::{
        geo::LatLng,
        order::display_number,
        page::Page,
        rider::{
            ActiveDelivery, DeliverRequest, DeliveryHistoryItem, LocationBatch, RiderLocation,
            RiderMe, RiderProfileRequest,
        },
        ws::WsServerMessage,
    },
    error::{AppError, AppResult},
    extractors::AuthUser,
    models::order::{OrderStatus, PaymentMethod, PaymentStatus},
    repositories::{orders, riders, stores},
    services::{dispatch_service, order_service},
    state::AppState,
};

/// Wrong OTPs allowed before the delivery locks (support must step in).
pub const MAX_OTP_ATTEMPTS: i32 = 5;
/// Rider this far from the store with the goods counts as departed.
const DEPART_RADIUS_M: f64 = 200.0;
/// Rider must be this close to the customer to complete delivery...
const DELIVERY_RADIUS_M: f64 = 500.0;
/// ...judged by a fix no older than this.
const FIX_MAX_AGE_MS: i64 = 2 * 60 * 1000;

fn coded(status: StatusCode, code: &'static str, message: impl Into<String>) -> AppError {
    AppError::Coded {
        status,
        code,
        message: message.into(),
    }
}

pub fn store_of(rider: &AuthUser) -> AppResult<Uuid> {
    rider.store_id.ok_or(AppError::Forbidden {
        code: "NO_STORE",
        message: "you're not assigned to a store yet",
    })
}

pub async fn me(state: &AppState, rider: &AuthUser) -> AppResult<RiderMe> {
    let profile = riders::profile(&state.db, rider.user_id).await?;
    let store = match rider.store_id {
        Some(id) => stores::find(&state.db, id).await?,
        None => None,
    };
    Ok(RiderMe {
        is_online: profile.is_online,
        vehicle_type: profile.vehicle_type,
        vehicle_number: profile.vehicle_number,
        store_id: rider.store_id,
        store_name: store.map(|s| s.name),
        active_order_id: riders::active_for_rider(&state.db, rider.user_id)
            .await?
            .map(|d| d.order_id),
        delivered_today: riders::delivered_today(&state.db, rider.user_id).await?,
    })
}

pub async fn update_profile(
    state: &AppState,
    rider: &AuthUser,
    req: &RiderProfileRequest,
) -> AppResult<RiderMe> {
    riders::profile(&state.db, rider.user_id).await?;
    riders::update_vehicle(
        &state.db,
        rider.user_id,
        req.vehicle_type,
        req.vehicle_number.as_deref(),
    )
    .await?;
    me(state, rider).await
}

pub async fn set_online(state: &AppState, rider: &AuthUser, online: bool) -> AppResult<RiderMe> {
    let store = store_of(rider)?;
    riders::profile(&state.db, rider.user_id).await?;
    if !online {
        if riders::active_for_rider(&state.db, rider.user_id)
            .await?
            .is_some()
        {
            return Err(coded(
                StatusCode::CONFLICT,
                "ON_DELIVERY",
                "finish your delivery before going offline",
            ));
        }
        geo::remove_rider(&state.redis, store, rider.user_id).await?;
        let now = Utc::now().timestamp_millis();
        for (order_id, _) in geo::pending_offers(&state.redis, rider.user_id, now).await? {
            dispatch_service::decline(state, rider, order_id).await?;
        }
    }
    riders::set_online(&state.db, rider.user_id, online).await?;
    me(state, rider).await
}

/// Background GPS upload. Offline riders' points are ignored.
pub async fn record_location(
    state: &AppState,
    rider: &AuthUser,
    batch: &LocationBatch,
) -> AppResult<()> {
    let store = store_of(rider)?;
    let Some(latest) = batch.points.iter().max_by_key(|p| p.recorded_at) else {
        return Ok(());
    };
    if !riders::profile(&state.db, rider.user_id).await?.is_online {
        return Ok(());
    }
    let now = Utc::now().timestamp_millis();
    // A fix from the future (bad device clock) counts as now.
    let at_ms = latest.recorded_at.min(now);
    let fix = geo::Fix {
        lat: latest.lat,
        lng: latest.lng,
        at_ms,
    };
    geo::update_position(&state.redis, store, rider.user_id, fix, at_ms).await?;

    if let Some(delivery) = riders::active_for_rider(&state.db, rider.user_id).await?
        && let Some(info) = riders::info(&state.db, delivery.order_id).await?
    {
        if info.status == OrderStatus::PickedUp
            && geo::haversine_m((fix.lat, fix.lng), (info.store_lat, info.store_lng))
                > DEPART_RADIUS_M
        {
            depart_inner(
                state,
                rider,
                delivery.id,
                delivery.order_id,
                "left the store",
            )
            .await?;
        }
        let msg = WsServerMessage::RiderLocation {
            location: RiderLocation {
                order_id: delivery.order_id,
                lat: fix.lat,
                lng: fix.lng,
                at: Utc::now(),
            },
        };
        // Same channel as the order's status events: the customer's tracking socket.
        let _ = events::publish(
            &state.redis,
            &[events::order_channel(delivery.order_id)],
            &msg,
        )
        .await;
    }
    Ok(())
}

async fn delivery_for(
    state: &AppState,
    rider: &AuthUser,
    order_id: Uuid,
) -> AppResult<crate::models::rider::Delivery> {
    riders::active_for_rider(&state.db, rider.user_id)
        .await?
        .filter(|d| d.order_id == order_id)
        .ok_or(AppError::NotFound("delivery"))
}

pub async fn active(state: &AppState, rider: &AuthUser) -> AppResult<Option<ActiveDelivery>> {
    let Some(d) = riders::active_for_rider(&state.db, rider.user_id).await? else {
        return Ok(None);
    };
    let Some(info) = riders::info(&state.db, d.order_id).await? else {
        return Ok(None);
    };
    Ok(Some(ActiveDelivery {
        order_id: info.order_id,
        number: display_number(info.number),
        status: info.status,
        store_name: info.store_name.clone(),
        store_address: info.store_address.clone(),
        store_location: LatLng {
            lat: info.store_lat,
            lng: info.store_lng,
        },
        drop: info.address.0.clone(),
        customer_phone: info.customer_phone.clone(),
        item_count: info.item_count,
        bag_count: info.bag_count,
        staging_slot: info.staging_slot.clone(),
        payment_method: info.payment_method,
        collect_paise: dispatch_service::collect_paise(&info),
        otp_attempts_left: (MAX_OTP_ATTEMPTS - d.otp_attempts).max(0),
    }))
}

async fn active_or_404(state: &AppState, rider: &AuthUser) -> AppResult<ActiveDelivery> {
    active(state, rider)
        .await?
        .ok_or(AppError::NotFound("delivery"))
}

/// RIDER_ASSIGNED -> PICKED_UP, confirming the bag count.
pub async fn pickup(
    state: &AppState,
    rider: &AuthUser,
    order_id: Uuid,
    bags: i32,
) -> AppResult<ActiveDelivery> {
    let d = delivery_for(state, rider, order_id).await?;
    let info = riders::info(&state.db, order_id)
        .await?
        .ok_or(AppError::NotFound("order"))?;
    if let Some(packed) = info.bag_count
        && packed != bags
    {
        return Err(coded(
            StatusCode::UNPROCESSABLE_ENTITY,
            "BAG_MISMATCH",
            format!("this order was packed in {packed} bag(s); check the staging area"),
        ));
    }
    let mut tx = state.db.begin().await?;
    let prev = orders::set_status(
        &mut tx,
        order_id,
        &[OrderStatus::RiderAssigned],
        OrderStatus::PickedUp,
        Some(rider.user_id),
        Some("picked up"),
        None,
    )
    .await?;
    if prev.is_none() {
        tx.rollback().await?;
        return Err(
            order_service::invalid_transition(state, order_id, OrderStatus::PickedUp).await,
        );
    }
    riders::mark_picked_up(&mut tx, d.id).await?;
    tx.commit().await?;
    order_service::publish_status(state, order_id, prev, OrderStatus::PickedUp).await;
    active_or_404(state, rider).await
}

async fn depart_inner(
    state: &AppState,
    rider: &AuthUser,
    delivery_id: Uuid,
    order_id: Uuid,
    note: &str,
) -> AppResult<()> {
    let mut tx = state.db.begin().await?;
    let prev = orders::set_status(
        &mut tx,
        order_id,
        &[OrderStatus::PickedUp],
        OrderStatus::OutForDelivery,
        Some(rider.user_id),
        Some(note),
        None,
    )
    .await?;
    if prev.is_none() {
        tx.rollback().await?;
        return Ok(()); // Already on the way.
    }
    riders::mark_departed(&mut tx, delivery_id).await?;
    tx.commit().await?;
    order_service::publish_status(state, order_id, prev, OrderStatus::OutForDelivery).await;
    Ok(())
}

/// PICKED_UP -> OUT_FOR_DELIVERY (also happens automatically on leaving the store).
pub async fn depart(
    state: &AppState,
    rider: &AuthUser,
    order_id: Uuid,
) -> AppResult<ActiveDelivery> {
    let d = delivery_for(state, rider, order_id).await?;
    depart_inner(state, rider, d.id, order_id, "on the way").await?;
    active_or_404(state, rider).await
}

fn otp_matches(expected: &str, given: &str) -> bool {
    let (a, b) = (expected.as_bytes(), given.as_bytes());
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Completes the delivery with the customer's OTP (and cash, for COD).
pub async fn deliver(
    state: &AppState,
    rider: &AuthUser,
    order_id: Uuid,
    req: &DeliverRequest,
) -> AppResult<DeliveryHistoryItem> {
    let d = delivery_for(state, rider, order_id).await?;
    let order = orders::find(&state.db, order_id)
        .await?
        .ok_or(AppError::NotFound("order"))?;
    if !matches!(
        order.status,
        OrderStatus::PickedUp | OrderStatus::OutForDelivery
    ) {
        return Err(coded(
            StatusCode::CONFLICT,
            "NOT_PICKED_UP",
            "pick the order up from the store first",
        ));
    }
    if d.otp_attempts >= MAX_OTP_ATTEMPTS {
        return Err(coded(
            StatusCode::LOCKED,
            "OTP_LOCKED",
            "too many wrong OTPs; contact support",
        ));
    }
    let now = Utc::now().timestamp_millis();
    if let Some(fix) = geo::last_fix(&state.redis, rider.user_id).await?
        && now - fix.at_ms <= FIX_MAX_AGE_MS
        && geo::haversine_m((fix.lat, fix.lng), (order.address.lat, order.address.lng))
            > DELIVERY_RADIUS_M
    {
        return Err(coded(
            StatusCode::UNPROCESSABLE_ENTITY,
            "TOO_FAR",
            "you're not at the delivery address yet",
        ));
    }
    if !otp_matches(&order.delivery_otp, req.otp.trim()) {
        let attempts = riders::bump_otp_attempts(&state.db, d.id).await?;
        let left = (MAX_OTP_ATTEMPTS - attempts).max(0);
        return Err(if left == 0 {
            coded(
                StatusCode::LOCKED,
                "OTP_LOCKED",
                "too many wrong OTPs; contact support",
            )
        } else {
            coded(
                StatusCode::UNPROCESSABLE_ENTITY,
                "WRONG_OTP",
                format!("wrong OTP, {left} attempt(s) left"),
            )
        });
    }
    let cod =
        order.payment_method == PaymentMethod::Cod && order.payment_status != PaymentStatus::Paid;
    if cod && req.cod_collected_paise != Some(order.total_paise) {
        return Err(coded(
            StatusCode::UNPROCESSABLE_ENTITY,
            "COD_AMOUNT_MISMATCH",
            format!(
                "collect exactly {} from the customer",
                crate::services::catalog_service::rupees(order.total_paise)
            ),
        ));
    }

    if order.status == OrderStatus::PickedUp {
        depart_inner(state, rider, d.id, order_id, "on the way").await?;
    }
    let done = if riders::has_shortage(&state.db, order_id).await? {
        OrderStatus::PartiallyFulfilled
    } else {
        OrderStatus::Delivered
    };
    let mut tx = state.db.begin().await?;
    let prev = orders::set_status(
        &mut tx,
        order_id,
        &[OrderStatus::OutForDelivery],
        done,
        Some(rider.user_id),
        Some("delivered"),
        None,
    )
    .await?;
    if prev.is_none() {
        tx.rollback().await?;
        return Err(order_service::invalid_transition(state, order_id, done).await);
    }
    riders::mark_delivered(&mut tx, d.id, cod.then_some(order.total_paise)).await?;
    if cod {
        orders::set_payment_status(&mut *tx, order_id, PaymentStatus::Paid).await?;
    }
    tx.commit().await?;
    order_service::publish_status(state, order_id, prev, done).await;
    Ok(DeliveryHistoryItem {
        order_id,
        number: display_number(order.number),
        status: done,
        assigned_at: d.assigned_at,
        delivered_at: Some(Utc::now()),
        cod_collected_paise: cod.then_some(order.total_paise),
    })
}

/// The rider drops the job before pickup: back to PACKED and re-dispatched
/// to someone else.
pub async fn unassign(state: &AppState, rider: &AuthUser, order_id: Uuid) -> AppResult<()> {
    delivery_for(state, rider, order_id).await?;
    let mut tx = state.db.begin().await?;
    let prev = orders::set_status(
        &mut tx,
        order_id,
        &[OrderStatus::RiderAssigned],
        OrderStatus::Packed,
        Some(rider.user_id),
        Some("rider dropped the order"),
        None,
    )
    .await?;
    if prev.is_none() {
        tx.rollback().await?;
        return Err(coded(
            StatusCode::CONFLICT,
            "ALREADY_PICKED_UP",
            "you've already picked this order up",
        ));
    }
    riders::end_active_for_order(&mut tx, order_id).await?;
    tx.commit().await?;
    geo::clear_winner(&state.redis, order_id).await?;
    order_service::publish_status(state, order_id, prev, OrderStatus::Packed).await;
    dispatch_service::dispatch_best_effort(state, order_id).await;
    Ok(())
}

pub async fn history(
    state: &AppState,
    rider: &AuthUser,
    limit: i64,
    offset: i64,
) -> AppResult<Page<DeliveryHistoryItem>> {
    let (rows, total) = riders::history(&state.db, rider.user_id, limit, offset).await?;
    Ok(Page {
        items: rows
            .into_iter()
            .map(|r| DeliveryHistoryItem {
                order_id: r.order_id,
                number: display_number(r.number),
                status: r.status,
                assigned_at: r.assigned_at,
                delivered_at: r.delivered_at,
                cod_collected_paise: r.cod_collected_paise,
            })
            .collect(),
        total,
        limit,
        offset,
    })
}

#[cfg(test)]
mod tests {
    use super::otp_matches;

    #[test]
    fn otp_compare() {
        assert!(otp_matches("0421", "0421"));
        assert!(!otp_matches("0421", "0422"));
        assert!(!otp_matches("0421", "042"));
        assert!(!otp_matches("0421", ""));
    }
}
