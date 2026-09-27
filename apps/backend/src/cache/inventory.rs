//! Redis mirror of store stock plus short-lived reservations.
//!
//! Postgres `store_inventory.quantity` is the source of truth. Per store, Redis
//! holds `stock` (a mirror of that quantity) and `held` (units reserved by
//! checkouts in flight). A checkout may reserve `q` units only while
//! `stock - held >= q`, checked and applied for all lines in one Lua script, so
//! concurrent checkouts can't oversell. All keys of a store share a hash tag,
//! so the scripts also work on Redis Cluster.

use std::{sync::LazyLock, time::Duration};

use deadpool_redis::{
    Pool,
    redis::{self, AsyncCommands, Script},
};
use uuid::Uuid;

use crate::error::AppError;

static RESERVE: LazyLock<Script> =
    LazyLock::new(|| Script::new(include_str!("scripts/reserve.lua")));
static RELEASE: LazyLock<Script> =
    LazyLock::new(|| Script::new(include_str!("scripts/release.lua")));
static COMMIT: LazyLock<Script> = LazyLock::new(|| Script::new(include_str!("scripts/commit.lua")));

/// Reservation keys outlive their expiry by a wide margin so the sweeper can
/// always find what to release, even if it was down for a while.
const RESERVATION_KEY_TTL: Duration = Duration::from_secs(24 * 3600);

struct Keys {
    stock: String,
    held: String,
    exp: String,
}

fn keys(store_id: Uuid) -> Keys {
    Keys {
        stock: format!("inv:{{{store_id}}}:stock"),
        held: format!("inv:{{{store_id}}}:held"),
        exp: format!("inv:{{{store_id}}}:resv_exp"),
    }
}

fn reservation_key(store_id: Uuid, reservation_id: Uuid) -> String {
    format!("inv:{{{store_id}}}:resv:{reservation_id}")
}

#[derive(Debug, PartialEq, Eq)]
pub enum ReserveOutcome {
    Reserved,
    /// Not enough units of this product.
    Insufficient(Uuid),
    /// The stock mirror has no entry for this product; load it and retry.
    NotLoaded(Uuid),
}

pub async fn reserve(
    redis: &Pool,
    store_id: Uuid,
    reservation_id: Uuid,
    lines: &[(Uuid, i32)],
    expires_at_ms: i64,
) -> Result<ReserveOutcome, AppError> {
    let k = keys(store_id);
    let mut conn = redis.get().await?;
    let mut invocation = RESERVE.prepare_invoke();
    invocation
        .key(&k.stock)
        .key(&k.held)
        .key(reservation_key(store_id, reservation_id))
        .key(&k.exp)
        .arg(reservation_id.to_string())
        .arg(expires_at_ms)
        .arg(RESERVATION_KEY_TTL.as_secs())
        .arg(lines.len());
    for (product_id, qty) in lines {
        invocation.arg(product_id.to_string()).arg(*qty);
    }
    let (code, product): (i64, String) = invocation.invoke_async(&mut conn).await?;
    let product = || Uuid::parse_str(&product).map_err(|e| AppError::Internal(e.into()));
    Ok(match code {
        1 => ReserveOutcome::Reserved,
        0 => ReserveOutcome::Insufficient(product()?),
        _ => ReserveOutcome::NotLoaded(product()?),
    })
}

/// Returns the reserved units to the pool. Idempotent.
pub async fn release(redis: &Pool, store_id: Uuid, reservation_id: Uuid) -> Result<(), AppError> {
    let k = keys(store_id);
    let mut conn = redis.get().await?;
    let _: i64 = RELEASE
        .key(&k.held)
        .key(reservation_key(store_id, reservation_id))
        .key(&k.exp)
        .arg(reservation_id.to_string())
        .invoke_async(&mut conn)
        .await?;
    Ok(())
}

/// Converts held units into sold units after Postgres was decremented. Idempotent.
pub async fn commit(redis: &Pool, store_id: Uuid, reservation_id: Uuid) -> Result<(), AppError> {
    let k = keys(store_id);
    let mut conn = redis.get().await?;
    let _: i64 = COMMIT
        .key(&k.stock)
        .key(&k.held)
        .key(reservation_key(store_id, reservation_id))
        .key(&k.exp)
        .arg(reservation_id.to_string())
        .invoke_async(&mut conn)
        .await?;
    Ok(())
}

/// Product ids of `candidates` whose stock isn't mirrored yet.
pub async fn missing_stock(
    redis: &Pool,
    store_id: Uuid,
    candidates: &[Uuid],
) -> Result<Vec<Uuid>, AppError> {
    if candidates.is_empty() {
        return Ok(vec![]);
    }
    let mut conn = redis.get().await?;
    let fields: Vec<String> = candidates.iter().map(Uuid::to_string).collect();
    let values: Vec<Option<i64>> = redis::cmd("HMGET")
        .arg(keys(store_id).stock)
        .arg(&fields)
        .query_async(&mut conn)
        .await?;
    Ok(candidates
        .iter()
        .zip(values)
        .filter_map(|(id, v)| v.is_none().then_some(*id))
        .collect())
}

/// Loads Postgres quantities into the mirror without clobbering entries a
/// concurrent request already loaded (HSETNX).
pub async fn load_stock(
    redis: &Pool,
    store_id: Uuid,
    quantities: &[(Uuid, i32)],
) -> Result<(), AppError> {
    let mut conn = redis.get().await?;
    let key = keys(store_id).stock;
    for (product_id, qty) in quantities {
        let _: bool = conn.hset_nx(&key, product_id.to_string(), *qty).await?;
    }
    Ok(())
}

/// Overwrites the mirror after an admin changed stock in Postgres.
pub async fn set_stock(
    redis: &Pool,
    store_id: Uuid,
    product_id: Uuid,
    qty: i32,
) -> Result<(), AppError> {
    let mut conn = redis.get().await?;
    let _: i64 = conn
        .hset(keys(store_id).stock, product_id.to_string(), qty)
        .await?;
    Ok(())
}

/// Adds units back to the mirror (restock after a confirmed order is cancelled).
pub async fn add_stock(
    redis: &Pool,
    store_id: Uuid,
    product_id: Uuid,
    qty: i32,
) -> Result<(), AppError> {
    let mut conn = redis.get().await?;
    let key = keys(store_id).stock;
    let field = product_id.to_string();
    // Only a loaded entry is adjusted; a missing one reloads from Postgres.
    if conn.hexists(&key, &field).await? {
        let _: i64 = conn.hincr(&key, &field, qty).await?;
    }
    Ok(())
}

/// Reservation ids in this store whose expiry has passed.
pub async fn expired_reservations(
    redis: &Pool,
    store_id: Uuid,
    now_ms: i64,
) -> Result<Vec<Uuid>, AppError> {
    let mut conn = redis.get().await?;
    let ids: Vec<String> = conn
        .zrangebyscore_limit(keys(store_id).exp, "-inf", now_ms, 0, 100)
        .await?;
    Ok(ids.iter().filter_map(|s| Uuid::parse_str(s).ok()).collect())
}

/// Units currently held for a product (for tests and diagnostics).
pub async fn held(redis: &Pool, store_id: Uuid, product_id: Uuid) -> Result<i64, AppError> {
    let mut conn = redis.get().await?;
    let v: Option<i64> = conn
        .hget(keys(store_id).held, product_id.to_string())
        .await?;
    Ok(v.unwrap_or(0))
}

/// Mirrored stock for a product, if loaded (for tests and diagnostics).
pub async fn stock(
    redis: &Pool,
    store_id: Uuid,
    product_id: Uuid,
) -> Result<Option<i64>, AppError> {
    let mut conn = redis.get().await?;
    Ok(conn
        .hget(keys(store_id).stock, product_id.to_string())
        .await?)
}

/// Overwrites a store's stock mirror with Postgres quantities (reconciliation).
/// `held` is untouched: expired reservations are released by the sweeper.
/// ponytail: a confirm/cancel racing this can leave one product off by that
/// order's units until the next run; Postgres' conditional UPDATE still
/// prevents overselling. Lock per store if that window ever matters.
pub async fn resync_stock(
    redis: &Pool,
    store_id: Uuid,
    levels: &[(Uuid, i32)],
) -> Result<(), AppError> {
    if levels.is_empty() {
        return Ok(());
    }
    let mut conn = redis.get().await?;
    let fields: Vec<(String, i32)> = levels.iter().map(|(id, q)| (id.to_string(), *q)).collect();
    let () = conn.hset_multiple(keys(store_id).stock, &fields).await?;
    Ok(())
}
