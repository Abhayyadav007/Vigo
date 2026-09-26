use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgExecutor, types::Json};
use uuid::Uuid;

use crate::models::{
    order::{AddressSnapshot, OrderStatus, PaymentMethod, PaymentStatus},
    rider::{Delivery, DeliveryInfo, RiderProfile, VehicleType},
};

/// The rider's profile, created on first use.
pub async fn profile<'e>(
    db: impl PgExecutor<'e>,
    user_id: Uuid,
) -> Result<RiderProfile, sqlx::Error> {
    sqlx::query_as!(
        RiderProfile,
        r#"
        WITH ins AS (
            INSERT INTO rider_profiles (user_id) VALUES ($1)
            ON CONFLICT (user_id) DO NOTHING
            RETURNING user_id, vehicle_type, vehicle_number, is_online
        )
        SELECT user_id AS "user_id!", vehicle_type AS "vehicle_type!: VehicleType",
               vehicle_number, is_online AS "is_online!" FROM ins
        UNION ALL
        SELECT user_id, vehicle_type, vehicle_number, is_online
        FROM rider_profiles WHERE user_id = $1
        LIMIT 1
        "#,
        user_id,
    )
    .fetch_one(db)
    .await
}

pub async fn set_online<'e>(
    db: impl PgExecutor<'e>,
    user_id: Uuid,
    online: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE rider_profiles SET is_online = $2, last_seen_at = now() WHERE user_id = $1",
        user_id,
        online
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn touch<'e>(db: impl PgExecutor<'e>, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE rider_profiles SET last_seen_at = now() WHERE user_id = $1",
        user_id
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn update_vehicle<'e>(
    db: impl PgExecutor<'e>,
    user_id: Uuid,
    vehicle_type: VehicleType,
    vehicle_number: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE rider_profiles SET vehicle_type = $2, vehicle_number = $3 WHERE user_id = $1",
        user_id,
        vehicle_type as VehicleType,
        vehicle_number,
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn online_rider_ids<'e>(
    db: impl PgExecutor<'e>,
    ids: &[Uuid],
) -> Result<Vec<Uuid>, sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT user_id FROM rider_profiles WHERE user_id = ANY($1) AND is_online",
        ids
    )
    .fetch_all(db)
    .await
}

// ---------- deliveries ----------

pub async fn active_for_rider<'e>(
    db: impl PgExecutor<'e>,
    rider: Uuid,
) -> Result<Option<Delivery>, sqlx::Error> {
    sqlx::query_as!(
        Delivery,
        r#"
        SELECT id, order_id, rider_id, store_id, assigned_at, picked_up_at, otp_attempts
        FROM deliveries WHERE rider_id = $1 AND delivered_at IS NULL AND ended_at IS NULL
        "#,
        rider,
    )
    .fetch_optional(db)
    .await
}

pub async fn active_for_order<'e>(
    db: impl PgExecutor<'e>,
    order: Uuid,
) -> Result<Option<Delivery>, sqlx::Error> {
    sqlx::query_as!(
        Delivery,
        r#"
        SELECT id, order_id, rider_id, store_id, assigned_at, picked_up_at, otp_attempts
        FROM deliveries WHERE order_id = $1 AND delivered_at IS NULL AND ended_at IS NULL
        "#,
        order,
    )
    .fetch_optional(db)
    .await
}

/// The rider on the most recent delivery of this order (active or not).
pub async fn latest_rider_for_order<'e>(
    db: impl PgExecutor<'e>,
    order: Uuid,
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT rider_id FROM deliveries WHERE order_id = $1 ORDER BY assigned_at DESC LIMIT 1",
        order
    )
    .fetch_optional(db)
    .await
}

/// Of `ids`, the riders currently on a delivery.
pub async fn busy<'e>(db: impl PgExecutor<'e>, ids: &[Uuid]) -> Result<Vec<Uuid>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        SELECT rider_id FROM deliveries
        WHERE rider_id = ANY($1) AND delivered_at IS NULL AND ended_at IS NULL
        "#,
        ids
    )
    .fetch_all(db)
    .await
}

pub async fn insert(
    conn: &mut PgConnection,
    order: Uuid,
    rider: Uuid,
    store: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO deliveries (order_id, rider_id, store_id) VALUES ($1, $2, $3)",
        order,
        rider,
        store
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn mark_picked_up(conn: &mut PgConnection, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE deliveries SET picked_up_at = now() WHERE id = $1",
        id
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn mark_departed(conn: &mut PgConnection, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE deliveries SET departed_at = now() WHERE id = $1",
        id
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn mark_delivered(
    conn: &mut PgConnection,
    id: Uuid,
    cod_collected: Option<i64>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE deliveries SET delivered_at = now(), cod_collected_paise = $2 WHERE id = $1",
        id,
        cod_collected
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Ends the active delivery for an order (rider dropped it, order cancelled).
pub async fn end_active_for_order(
    conn: &mut PgConnection,
    order: Uuid,
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        UPDATE deliveries SET ended_at = now()
        WHERE order_id = $1 AND delivered_at IS NULL AND ended_at IS NULL
        RETURNING rider_id
        "#,
        order
    )
    .fetch_optional(&mut *conn)
    .await
}

/// Counts a wrong OTP; returns the new total.
pub async fn bump_otp_attempts<'e>(db: impl PgExecutor<'e>, id: Uuid) -> Result<i32, sqlx::Error> {
    sqlx::query_scalar!(
        "UPDATE deliveries SET otp_attempts = otp_attempts + 1 WHERE id = $1 RETURNING otp_attempts",
        id
    )
    .fetch_one(db)
    .await
}

pub async fn delivered_today<'e>(db: impl PgExecutor<'e>, rider: Uuid) -> Result<i32, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        SELECT count(*)::int AS "n!" FROM deliveries
        WHERE rider_id = $1 AND delivered_at >= date_trunc('day', now() AT TIME ZONE 'Asia/Kolkata') AT TIME ZONE 'Asia/Kolkata'
        "#,
        rider
    )
    .fetch_one(db)
    .await
}

pub async fn info<'e>(
    db: impl PgExecutor<'e>,
    order: Uuid,
) -> Result<Option<DeliveryInfo>, sqlx::Error> {
    sqlx::query_as!(
        DeliveryInfo,
        r#"
        SELECT o.id AS order_id, o.number, o.store_id, s.name AS store_name, s.address AS store_address,
               ST_Y(s.location::geometry) AS "store_lat!", ST_X(s.location::geometry) AS "store_lng!",
               ST_Y(o.delivery_location::geometry) AS "drop_lat!", ST_X(o.delivery_location::geometry) AS "drop_lng!",
               o.address AS "address: Json<AddressSnapshot>", u.phone AS customer_phone,
               (SELECT sum(coalesce(picked_quantity, quantity)) FROM order_items i WHERE i.order_id = o.id)::int AS "item_count!",
               o.bag_count, o.staging_slot,
               o.payment_method AS "payment_method: PaymentMethod",
               o.payment_status AS "payment_status: PaymentStatus",
               o.total_paise, o.status AS "status: OrderStatus"
        FROM orders o
        JOIN dark_stores s ON s.id = o.store_id
        JOIN users u ON u.id = o.user_id
        WHERE o.id = $1
        "#,
        order,
    )
    .fetch_optional(db)
    .await
}

/// Packed orders with no active delivery, oldest first (dispatch backlog).
pub async fn awaiting_rider<'e>(db: impl PgExecutor<'e>) -> Result<Vec<Uuid>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        SELECT o.id FROM orders o
        WHERE o.status = 'PACKED'
          AND NOT EXISTS (SELECT 1 FROM deliveries d WHERE d.order_id = o.id
                          AND d.delivered_at IS NULL AND d.ended_at IS NULL)
        ORDER BY o.packed_at NULLS FIRST, o.id
        LIMIT 200
        "#,
    )
    .fetch_all(db)
    .await
}

pub struct HistoryRow {
    pub order_id: Uuid,
    pub number: i64,
    pub status: OrderStatus,
    pub assigned_at: DateTime<Utc>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub cod_collected_paise: Option<i64>,
}

pub async fn history<'e>(
    db: impl PgExecutor<'e>,
    rider: Uuid,
    limit: i64,
    offset: i64,
) -> Result<(Vec<HistoryRow>, i64), sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT d.order_id, o.number, o.status AS "status: OrderStatus", d.assigned_at,
               d.delivered_at, d.cod_collected_paise, count(*) OVER () AS "total!"
        FROM deliveries d JOIN orders o ON o.id = d.order_id
        WHERE d.rider_id = $1
        ORDER BY d.assigned_at DESC
        LIMIT $2 OFFSET $3
        "#,
        rider,
        limit,
        offset
    )
    .fetch_all(db)
    .await?;
    let total = rows.first().map_or(0, |r| r.total);
    Ok((
        rows.into_iter()
            .map(|r| HistoryRow {
                order_id: r.order_id,
                number: r.number,
                status: r.status,
                assigned_at: r.assigned_at,
                delivered_at: r.delivered_at,
                cod_collected_paise: r.cod_collected_paise,
            })
            .collect(),
        total,
    ))
}

pub struct RiderContact {
    pub name: Option<String>,
    pub phone: String,
    pub vehicle_type: VehicleType,
    pub vehicle_number: Option<String>,
}

pub async fn contact<'e>(
    db: impl PgExecutor<'e>,
    rider: Uuid,
) -> Result<Option<RiderContact>, sqlx::Error> {
    sqlx::query_as!(
        RiderContact,
        r#"
        SELECT u.name, u.phone, coalesce(p.vehicle_type, 'SCOOTER') AS "vehicle_type!: VehicleType",
               p.vehicle_number
        FROM users u LEFT JOIN rider_profiles p ON p.user_id = u.id
        WHERE u.id = $1
        "#,
        rider,
    )
    .fetch_optional(db)
    .await
}

/// True if any line was picked short (the delivery ends PARTIALLY_FULFILLED).
pub async fn has_shortage<'e>(db: impl PgExecutor<'e>, order: Uuid) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM order_items WHERE order_id = $1 AND picked_quantity < quantity) AS "x!""#,
        order
    )
    .fetch_one(db)
    .await
}
