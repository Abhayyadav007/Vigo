//! Assigning packed orders to riders.
//!
//! An order that reaches PACKED is offered to the nearest few available
//! riders of its store at once (a "wave"). The first to accept wins, decided
//! atomically in Redis (`claim_offer.lua`), then made durable with a
//! compare-and-set status change plus a unique active-delivery index.
//! Declined or expired waves move on to riders not yet tried; when everyone
//! has been tried the list resets. A background tick re-dispatches anything
//! still waiting.

use std::collections::HashSet;

use axum::http::StatusCode;
use chrono::{TimeZone, Utc};
use uuid::Uuid;

use crate::{
    cache::{events, geo},
    dto::{
        geo::LatLng,
        order::display_number,
        rider::{ActiveDelivery, DeliveryOffer},
        ws::WsServerMessage,
    },
    error::{AppError, AppResult},
    extractors::AuthUser,
    models::{
        order::{OrderStatus, PaymentMethod, PaymentStatus},
        rider::DeliveryInfo,
    },
    repositories::{orders, riders},
    services::{order_service, rider_service},
    state::AppState,
};

fn coded(status: StatusCode, code: &'static str, message: impl Into<String>) -> AppError {
    AppError::Coded {
        status,
        code,
        message: message.into(),
    }
}

/// Cash the rider collects at the door.
pub fn collect_paise(info: &DeliveryInfo) -> i64 {
    if info.payment_method == PaymentMethod::Cod && info.payment_status != PaymentStatus::Paid {
        info.total_paise
    } else {
        0
    }
}

pub fn offer_for(
    info: &DeliveryInfo,
    to_store_m: Option<f64>,
    expires_at_ms: i64,
) -> DeliveryOffer {
    let a = &info.address.0;
    DeliveryOffer {
        order_id: info.order_id,
        number: display_number(info.number),
        store_name: info.store_name.clone(),
        store_address: info.store_address.clone(),
        store_location: LatLng {
            lat: info.store_lat,
            lng: info.store_lng,
        },
        drop_area: format!("{}, {}", a.city, a.pincode),
        drop_location: LatLng {
            lat: info.drop_lat,
            lng: info.drop_lng,
        },
        to_store_m: to_store_m.map(f64::round),
        trip_m: geo::haversine_m(
            (info.store_lat, info.store_lng),
            (info.drop_lat, info.drop_lng),
        )
        .round(),
        item_count: info.item_count,
        bag_count: info.bag_count,
        collect_paise: collect_paise(info),
        expires_at: Utc
            .timestamp_millis_opt(expires_at_ms)
            .single()
            .unwrap_or_else(Utc::now),
    }
}

async fn push(state: &AppState, rider: Uuid, msg: &WsServerMessage) {
    if let Err(error) = events::publish(&state.redis, &[events::rider_channel(rider)], msg).await {
        tracing::warn!(%error, %rider, "pushing to rider failed");
    }
}

/// Offers a PACKED order to the next wave of riders. Returns how many riders
/// were offered (0 if it's already assigned, a wave is open, or nobody is
/// available).
pub async fn dispatch(state: &AppState, order_id: Uuid) -> AppResult<usize> {
    let Some(info) = riders::info(&state.db, order_id).await? else {
        return Ok(0);
    };
    if info.status != OrderStatus::Packed
        || geo::has_winner(&state.redis, order_id).await?
        || geo::wave_open(&state.redis, order_id).await?
        || riders::active_for_order(&state.db, order_id)
            .await?
            .is_some()
    {
        return Ok(0);
    }

    let cfg = &state.config.dispatch;
    let now = Utc::now().timestamp_millis();
    let stale_ms = i64::try_from(cfg.stale_after.as_millis()).unwrap_or(i64::MAX);
    let nearby = geo::nearby_riders(
        &state.redis,
        info.store_id,
        info.store_lat,
        info.store_lng,
        cfg.radius_m,
        now - stale_ms,
    )
    .await?;
    let ids: Vec<Uuid> = nearby.iter().map(|(id, _)| *id).collect();
    let online: HashSet<Uuid> = riders::online_rider_ids(&state.db, &ids)
        .await?
        .into_iter()
        .collect();
    let busy: HashSet<Uuid> = riders::busy(&state.db, &ids).await?.into_iter().collect();
    let available: Vec<(Uuid, f64)> = nearby
        .into_iter()
        .filter(|(id, _)| online.contains(id) && !busy.contains(id))
        .collect();
    let tried: HashSet<Uuid> = geo::tried(&state.redis, order_id)
        .await?
        .into_iter()
        .collect();
    let wave: Vec<(Uuid, f64)> = available
        .iter()
        .filter(|(id, _)| !tried.contains(id))
        .take(cfg.wave_size.max(1))
        .copied()
        .collect();

    if wave.is_empty() {
        if !available.is_empty() {
            // Everyone available has passed on it once; start over next tick.
            geo::reset_tried(&state.redis, order_id).await?;
        }
        return Ok(0);
    }

    let expires_at = now + i64::try_from(cfg.offer_ttl.as_millis()).unwrap_or(i64::MAX);
    let wave_ids: Vec<Uuid> = wave.iter().map(|(id, _)| *id).collect();
    geo::open_wave(&state.redis, order_id, &wave_ids, cfg.offer_ttl, expires_at).await?;
    for (rider, dist) in &wave {
        let offer = offer_for(&info, Some(*dist), expires_at);
        push(state, *rider, &WsServerMessage::Offer { offer }).await;
    }
    tracing::info!(%order_id, riders = wave.len(), "offered delivery");
    Ok(wave.len())
}

/// Dispatch without failing the caller (e.g. right after packing).
pub async fn dispatch_best_effort(state: &AppState, order_id: Uuid) {
    if let Err(error) = dispatch(state, order_id).await {
        tracing::warn!(%error, %order_id, "dispatch failed; the dispatcher tick retries");
    }
}

/// Re-dispatches packed orders whose wave expired or never happened.
pub async fn tick(state: &AppState) -> AppResult<usize> {
    let mut offered = 0;
    for order_id in riders::awaiting_rider(&state.db).await? {
        offered += dispatch(state, order_id).await?;
    }
    Ok(offered)
}

pub async fn pending_offers(state: &AppState, rider: &AuthUser) -> AppResult<Vec<DeliveryOffer>> {
    let now = Utc::now().timestamp_millis();
    let fix = geo::last_fix(&state.redis, rider.user_id).await?;
    let mut out = Vec::new();
    for (order_id, expires_at) in geo::pending_offers(&state.redis, rider.user_id, now).await? {
        if let Some(info) = riders::info(&state.db, order_id).await?
            && info.status == OrderStatus::Packed
        {
            let to_store =
                fix.map(|f| geo::haversine_m((f.lat, f.lng), (info.store_lat, info.store_lng)));
            out.push(offer_for(&info, to_store, expires_at));
        }
    }
    Ok(out)
}

/// First accept wins.
pub async fn accept(
    state: &AppState,
    rider: &AuthUser,
    order_id: Uuid,
) -> AppResult<ActiveDelivery> {
    let store = rider_service::store_of(rider)?;
    let info = riders::info(&state.db, order_id)
        .await?
        .filter(|i| i.store_id == store)
        .ok_or(AppError::NotFound("order"))?;
    if !riders::profile(&state.db, rider.user_id).await?.is_online {
        return Err(coded(
            StatusCode::CONFLICT,
            "OFFLINE",
            "go online to accept deliveries",
        ));
    }
    if riders::active_for_rider(&state.db, rider.user_id)
        .await?
        .is_some()
    {
        return Err(coded(
            StatusCode::CONFLICT,
            "BUSY",
            "finish your current delivery first",
        ));
    }

    let others = match geo::claim(&state.redis, order_id, rider.user_id).await? {
        geo::Claim::Won { others } => others,
        geo::Claim::Taken => {
            geo::drop_offer(&state.redis, rider.user_id, order_id).await?;
            return Err(coded(
                StatusCode::CONFLICT,
                "OFFER_TAKEN",
                "another rider accepted this order",
            ));
        }
        geo::Claim::NotOffered => {
            geo::drop_offer(&state.redis, rider.user_id, order_id).await?;
            return Err(coded(
                StatusCode::GONE,
                "OFFER_EXPIRED",
                "this offer has expired",
            ));
        }
    };

    let assigned = async {
        let mut tx = state.db.begin().await?;
        let prev = orders::set_status(
            &mut tx,
            order_id,
            &[OrderStatus::Packed],
            OrderStatus::RiderAssigned,
            Some(rider.user_id),
            Some("rider accepted"),
            None,
        )
        .await?;
        if prev.is_none() {
            tx.rollback().await?;
            return Ok::<_, AppError>(false);
        }
        riders::insert(&mut tx, order_id, rider.user_id, info.store_id).await?;
        tx.commit().await?;
        Ok(true)
    }
    .await;

    match assigned {
        Ok(true) => {}
        other => {
            // Let someone else have it (or nobody, if it was cancelled).
            geo::clear_winner(&state.redis, order_id).await?;
            geo::drop_offer(&state.redis, rider.user_id, order_id).await?;
            return match other {
                Err(AppError::Database(sqlx::Error::Database(db)))
                    if db.constraint() == Some("uq_deliveries_one_active_per_rider") =>
                {
                    Err(coded(
                        StatusCode::CONFLICT,
                        "BUSY",
                        "finish your current delivery first",
                    ))
                }
                Err(e) => Err(e),
                Ok(_) => Err(order_service::invalid_transition(
                    state,
                    order_id,
                    OrderStatus::RiderAssigned,
                )
                .await),
            };
        }
    }

    geo::drop_offer(&state.redis, rider.user_id, order_id).await?;
    for other in others {
        geo::drop_offer(&state.redis, other, order_id).await?;
        push(state, other, &WsServerMessage::OfferRevoked { order_id }).await;
    }
    order_service::publish_status(
        state,
        order_id,
        Some(OrderStatus::Packed),
        OrderStatus::RiderAssigned,
    )
    .await;
    rider_service::active(state, rider)
        .await?
        .ok_or(AppError::NotFound("delivery"))
}

pub async fn decline(state: &AppState, rider: &AuthUser, order_id: Uuid) -> AppResult<()> {
    let left = geo::decline(&state.redis, order_id, rider.user_id).await?;
    if left == 0 {
        dispatch_best_effort(state, order_id).await;
    }
    Ok(())
}
