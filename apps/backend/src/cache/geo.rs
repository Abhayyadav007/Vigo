//! Rider positions and dispatch offers in Redis.
//!
//! Per store: `riders:{store}:geo` (GEO set of online riders), and
//! `riders:{store}:seen` (zset of last-fix time, to skip stale riders).
//! Per rider: `rider:{id}:loc` (latest fix) and `rider:{id}:offers` (zset of
//! offered order ids by expiry). Per order: `dsp:{order}:offered` (current
//! offer wave), `dsp:{order}:tried` and `dsp:{order}:winner`.

use std::{sync::LazyLock, time::Duration};

use deadpool_redis::{
    Pool,
    redis::{self, AsyncCommands, Script},
};
use uuid::Uuid;

use crate::error::AppError;

static CLAIM: LazyLock<Script> =
    LazyLock::new(|| Script::new(include_str!("scripts/claim_offer.lua")));

const WINNER_TTL: Duration = Duration::from_secs(6 * 3600);
const TRIED_TTL: Duration = Duration::from_secs(6 * 3600);

fn geo_key(store: Uuid) -> String {
    format!("riders:{{{store}}}:geo")
}
fn seen_key(store: Uuid) -> String {
    format!("riders:{{{store}}}:seen")
}
fn loc_key(rider: Uuid) -> String {
    format!("rider:{rider}:loc")
}
fn offers_key(rider: Uuid) -> String {
    format!("rider:{rider}:offers")
}
fn offered_key(order: Uuid) -> String {
    format!("dsp:{{{order}}}:offered")
}
fn tried_key(order: Uuid) -> String {
    format!("dsp:{{{order}}}:tried")
}
fn winner_key(order: Uuid) -> String {
    format!("dsp:{{{order}}}:winner")
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fix {
    pub lat: f64,
    pub lng: f64,
    /// Unix ms when the device recorded it.
    pub at_ms: i64,
}

/// Records a rider's position and marks them seen now.
pub async fn update_position(
    redis: &Pool,
    store: Uuid,
    rider: Uuid,
    fix: Fix,
    now_ms: i64,
) -> Result<(), AppError> {
    let mut conn = redis.get().await?;
    let id = rider.to_string();
    let _: () = redis::pipe()
        .cmd("GEOADD")
        .arg(geo_key(store))
        .arg(fix.lng)
        .arg(fix.lat)
        .arg(&id)
        .ignore()
        .zadd(seen_key(store), &id, now_ms)
        .ignore()
        .hset_multiple(
            loc_key(rider),
            &[
                ("lat", fix.lat.to_string()),
                ("lng", fix.lng.to_string()),
                ("at", fix.at_ms.to_string()),
            ],
        )
        .ignore()
        .expire(loc_key(rider), 24 * 3600)
        .ignore()
        .query_async(&mut conn)
        .await?;
    Ok(())
}

/// Takes the rider out of the dispatch index (going offline).
pub async fn remove_rider(redis: &Pool, store: Uuid, rider: Uuid) -> Result<(), AppError> {
    let mut conn = redis.get().await?;
    let id = rider.to_string();
    let _: () = redis::pipe()
        .zrem(geo_key(store), &id)
        .ignore()
        .zrem(seen_key(store), &id)
        .ignore()
        .query_async(&mut conn)
        .await?;
    Ok(())
}

pub async fn last_fix(redis: &Pool, rider: Uuid) -> Result<Option<Fix>, AppError> {
    let mut conn = redis.get().await?;
    let (lat, lng, at): (Option<f64>, Option<f64>, Option<i64>) = redis::cmd("HMGET")
        .arg(loc_key(rider))
        .arg("lat")
        .arg("lng")
        .arg("at")
        .query_async(&mut conn)
        .await?;
    Ok(match (lat, lng, at) {
        (Some(lat), Some(lng), Some(at_ms)) => Some(Fix { lat, lng, at_ms }),
        _ => None,
    })
}

/// Riders within `radius_m` of a point, nearest first, seen since `fresh_after_ms`.
pub async fn nearby_riders(
    redis: &Pool,
    store: Uuid,
    lat: f64,
    lng: f64,
    radius_m: f64,
    fresh_after_ms: i64,
) -> Result<Vec<(Uuid, f64)>, AppError> {
    let mut conn = redis.get().await?;
    let hits: Vec<(String, f64)> = redis::cmd("GEOSEARCH")
        .arg(geo_key(store))
        .arg("FROMLONLAT")
        .arg(lng)
        .arg(lat)
        .arg("BYRADIUS")
        .arg(radius_m)
        .arg("m")
        .arg("ASC")
        .arg("COUNT")
        .arg(50)
        .arg("WITHDIST")
        .query_async(&mut conn)
        .await?;
    if hits.is_empty() {
        return Ok(vec![]);
    }
    let ids: Vec<&str> = hits.iter().map(|(id, _)| id.as_str()).collect();
    let seen: Vec<Option<i64>> = redis::cmd("ZMSCORE")
        .arg(seen_key(store))
        .arg(&ids)
        .query_async(&mut conn)
        .await?;
    Ok(hits
        .into_iter()
        .zip(seen)
        .filter(|(_, seen)| seen.is_some_and(|t| t >= fresh_after_ms))
        .filter_map(|((id, dist), _)| Uuid::parse_str(&id).ok().map(|u| (u, dist)))
        .collect())
}

// ---------- offers ----------

pub async fn has_winner(redis: &Pool, order: Uuid) -> Result<bool, AppError> {
    let mut conn = redis.get().await?;
    Ok(conn.exists(winner_key(order)).await?)
}

/// Whether an offer wave is still open for this order.
pub async fn wave_open(redis: &Pool, order: Uuid) -> Result<bool, AppError> {
    let mut conn = redis.get().await?;
    Ok(conn.exists(offered_key(order)).await?)
}

pub async fn tried(redis: &Pool, order: Uuid) -> Result<Vec<Uuid>, AppError> {
    let mut conn = redis.get().await?;
    let ids: Vec<String> = conn.smembers(tried_key(order)).await?;
    Ok(ids.iter().filter_map(|s| Uuid::parse_str(s).ok()).collect())
}

pub async fn reset_tried(redis: &Pool, order: Uuid) -> Result<(), AppError> {
    let mut conn = redis.get().await?;
    let _: i64 = conn.del(tried_key(order)).await?;
    Ok(())
}

/// Opens a new wave: these riders may accept until it expires.
pub async fn open_wave(
    redis: &Pool,
    order: Uuid,
    riders: &[Uuid],
    ttl: Duration,
    expires_at_ms: i64,
) -> Result<(), AppError> {
    let mut conn = redis.get().await?;
    let ids: Vec<String> = riders.iter().map(Uuid::to_string).collect();
    let mut pipe = redis::pipe();
    pipe.del(offered_key(order))
        .ignore()
        .sadd(offered_key(order), &ids)
        .ignore()
        .pexpire(
            offered_key(order),
            i64::try_from(ttl.as_millis()).unwrap_or(i64::MAX),
        )
        .ignore()
        .sadd(tried_key(order), &ids)
        .ignore()
        .expire(tried_key(order), TRIED_TTL.as_secs() as i64)
        .ignore();
    for id in riders {
        pipe.zadd(offers_key(*id), order.to_string(), expires_at_ms)
            .ignore();
    }
    let _: () = pipe.query_async(&mut conn).await?;
    Ok(())
}

pub enum Claim {
    Won { others: Vec<Uuid> },
    NotOffered,
    Taken,
}

pub async fn claim(redis: &Pool, order: Uuid, rider: Uuid) -> Result<Claim, AppError> {
    let mut conn = redis.get().await?;
    let res: Vec<String> = CLAIM
        .key(offered_key(order))
        .key(winner_key(order))
        .arg(rider.to_string())
        .arg(WINNER_TTL.as_secs())
        .invoke_async(&mut conn)
        .await?;
    Ok(match res.first().map(String::as_str) {
        Some("1") => Claim::Won {
            others: res[1..]
                .iter()
                .filter_map(|s| Uuid::parse_str(s).ok())
                .collect(),
        },
        Some("-1") => Claim::Taken,
        _ => Claim::NotOffered,
    })
}

/// The assignment fell through (order cancelled, rider dropped it): allow a
/// new winner.
pub async fn clear_winner(redis: &Pool, order: Uuid) -> Result<(), AppError> {
    let mut conn = redis.get().await?;
    let _: i64 = conn.del(winner_key(order)).await?;
    Ok(())
}

/// Removes one rider from the open wave; returns how many remain.
pub async fn decline(redis: &Pool, order: Uuid, rider: Uuid) -> Result<i64, AppError> {
    let mut conn = redis.get().await?;
    let (_, _, left): (i64, i64, i64) = redis::pipe()
        .srem(offered_key(order), rider.to_string())
        .zrem(offers_key(rider), order.to_string())
        .scard(offered_key(order))
        .query_async(&mut conn)
        .await?;
    Ok(left)
}

pub async fn drop_offer(redis: &Pool, rider: Uuid, order: Uuid) -> Result<(), AppError> {
    let mut conn = redis.get().await?;
    let _: i64 = conn.zrem(offers_key(rider), order.to_string()).await?;
    Ok(())
}

/// Order ids currently offered to the rider (unexpired).
pub async fn pending_offers(
    redis: &Pool,
    rider: Uuid,
    now_ms: i64,
) -> Result<Vec<(Uuid, i64)>, AppError> {
    let mut conn = redis.get().await?;
    let key = offers_key(rider);
    let _: i64 = conn.zrembyscore(&key, "-inf", now_ms).await?;
    let rows: Vec<(String, i64)> = conn.zrange_withscores(&key, 0, -1).await?;
    Ok(rows
        .into_iter()
        .filter_map(|(id, exp)| Uuid::parse_str(&id).ok().map(|u| (u, exp)))
        .collect())
}

/// Great-circle distance in metres.
pub fn haversine_m(a: (f64, f64), b: (f64, f64)) -> f64 {
    const R: f64 = 6_371_000.0;
    let (lat1, lng1) = (a.0.to_radians(), a.1.to_radians());
    let (lat2, lng2) = (b.0.to_radians(), b.1.to_radians());
    let h = ((lat2 - lat1) / 2.0).sin().powi(2)
        + lat1.cos() * lat2.cos() * ((lng2 - lng1) / 2.0).sin().powi(2);
    2.0 * R * h.sqrt().asin()
}

#[cfg(test)]
mod tests {
    use super::haversine_m;

    #[test]
    fn haversine_is_sane() {
        // Indiranagar -> Koramangala, ~4.4 km.
        let d = haversine_m((12.9719, 77.6412), (12.9352, 77.6245));
        assert!((4_300.0..4_600.0).contains(&d), "{d}");
        assert!(haversine_m((12.97, 77.64), (12.97, 77.64)) < 0.001);
    }
}
