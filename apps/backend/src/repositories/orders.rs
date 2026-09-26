use sqlx::{PgConnection, PgExecutor, types::Json};
use uuid::Uuid;

use crate::models::order::{
    AddressSnapshot, Order, OrderEventRow, OrderItem, OrderStatus, PaymentMethod, PaymentStatus,
};

pub struct NewOrder<'a> {
    pub id: Uuid,
    pub user_id: Uuid,
    pub store_id: Uuid,
    pub payment_method: PaymentMethod,
    pub item_total_paise: i64,
    pub mrp_total_paise: i64,
    pub delivery_fee_paise: i64,
    pub address: &'a AddressSnapshot,
    pub delivery_otp: &'a str,
    pub idempotency_key: &'a str,
}

pub struct NewOrderItem<'a> {
    pub product_id: Uuid,
    pub name: &'a str,
    pub brand: Option<&'a str>,
    pub unit_label: &'a str,
    pub image_url: Option<&'a str>,
    pub quantity: i32,
    pub unit_price_paise: i64,
    pub unit_mrp_paise: i64,
}

/// Inserts a PLACED order with its items and first status event.
pub async fn insert(
    conn: &mut PgConnection,
    o: &NewOrder<'_>,
    items: &[NewOrderItem<'_>],
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO orders (id, user_id, store_id, payment_method, item_total_paise, mrp_total_paise,
                            delivery_fee_paise, total_paise, address, delivery_location,
                            delivery_otp, idempotency_key)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $5::bigint + $7::bigint, $8,
                ST_SetSRID(ST_MakePoint($9, $10), 4326)::geography, $11, $12)
        "#,
        o.id,
        o.user_id,
        o.store_id,
        o.payment_method as PaymentMethod,
        o.item_total_paise,
        o.mrp_total_paise,
        o.delivery_fee_paise,
        Json(o.address) as _,
        o.address.lng,
        o.address.lat,
        o.delivery_otp,
        o.idempotency_key,
    )
    .execute(&mut *conn)
    .await?;

    for i in items {
        sqlx::query!(
            r#"
            INSERT INTO order_items (order_id, product_id, name, brand, unit_label, image_url,
                                     quantity, unit_price_paise, unit_mrp_paise)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
            o.id,
            i.product_id,
            i.name,
            i.brand,
            i.unit_label,
            i.image_url,
            i.quantity,
            i.unit_price_paise,
            i.unit_mrp_paise,
        )
        .execute(&mut *conn)
        .await?;
    }

    sqlx::query!(
        "INSERT INTO order_status_events (order_id, to_status, actor_user_id) VALUES ($1, 'PLACED', $2)",
        o.id,
        o.user_id,
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn find<'e>(db: impl PgExecutor<'e>, id: Uuid) -> Result<Option<Order>, sqlx::Error> {
    sqlx::query_as!(
        Order,
        r#"
        SELECT id, number, user_id, store_id, status AS "status: OrderStatus",
               payment_method AS "payment_method: PaymentMethod",
               payment_status AS "payment_status: PaymentStatus",
               item_total_paise, mrp_total_paise, delivery_fee_paise, total_paise,
               address AS "address: Json<AddressSnapshot>", delivery_otp, gateway_order_id,
               cancel_reason, created_at, updated_at
        FROM orders WHERE id = $1
        "#,
        id,
    )
    .fetch_optional(db)
    .await
}

pub async fn find_by_idempotency_key<'e>(
    db: impl PgExecutor<'e>,
    user_id: Uuid,
    key: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT id FROM orders WHERE user_id = $1 AND idempotency_key = $2",
        user_id,
        key,
    )
    .fetch_optional(db)
    .await
}

pub async fn find_by_gateway_order_id<'e>(
    db: impl PgExecutor<'e>,
    gateway_order_id: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT id FROM orders WHERE gateway_order_id = $1",
        gateway_order_id
    )
    .fetch_optional(db)
    .await
}

pub async fn items<'e>(
    db: impl PgExecutor<'e>,
    order_id: Uuid,
) -> Result<Vec<OrderItem>, sqlx::Error> {
    sqlx::query_as!(
        OrderItem,
        r#"
        SELECT product_id, name, brand, unit_label, image_url, quantity, unit_price_paise,
               unit_mrp_paise, picked_quantity
        FROM order_items WHERE order_id = $1 ORDER BY name
        "#,
        order_id,
    )
    .fetch_all(db)
    .await
}

pub async fn events<'e>(
    db: impl PgExecutor<'e>,
    order_id: Uuid,
) -> Result<Vec<OrderEventRow>, sqlx::Error> {
    sqlx::query_as!(
        OrderEventRow,
        r#"
        SELECT to_status AS "to_status: OrderStatus", note, created_at
        FROM order_status_events WHERE order_id = $1 ORDER BY id
        "#,
        order_id,
    )
    .fetch_all(db)
    .await
}

pub struct OrderListRow {
    pub order: Order,
    pub item_count: i32,
}

pub async fn list_for_user<'e>(
    db: impl PgExecutor<'e>,
    user_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<(Vec<OrderListRow>, i64), sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT id, number, user_id, store_id, status AS "status: OrderStatus",
               payment_method AS "payment_method: PaymentMethod",
               payment_status AS "payment_status: PaymentStatus",
               item_total_paise, mrp_total_paise, delivery_fee_paise, total_paise,
               address AS "address: Json<AddressSnapshot>", delivery_otp, gateway_order_id,
               cancel_reason, created_at, updated_at,
               (SELECT sum(quantity) FROM order_items oi WHERE oi.order_id = o.id)::int AS "item_count!",
               count(*) OVER () AS "total!"
        FROM orders o
        WHERE user_id = $1
        ORDER BY created_at DESC, id
        LIMIT $2 OFFSET $3
        "#,
        user_id,
        limit,
        offset,
    )
    .fetch_all(db)
    .await?;
    let total = rows.first().map_or(0, |r| r.total);
    let list = rows
        .into_iter()
        .map(|r| OrderListRow {
            item_count: r.item_count,
            order: Order {
                id: r.id,
                number: r.number,
                user_id: r.user_id,
                store_id: r.store_id,
                status: r.status,
                payment_method: r.payment_method,
                payment_status: r.payment_status,
                item_total_paise: r.item_total_paise,
                mrp_total_paise: r.mrp_total_paise,
                delivery_fee_paise: r.delivery_fee_paise,
                total_paise: r.total_paise,
                address: r.address,
                delivery_otp: r.delivery_otp,
                gateway_order_id: r.gateway_order_id,
                cancel_reason: r.cancel_reason,
                created_at: r.created_at,
                updated_at: r.updated_at,
            },
        })
        .collect();
    Ok((list, total))
}

/// Compare-and-set status change: succeeds only if the current status is one
/// of `from`. Returns the previous status, or None if nothing matched.
pub async fn set_status(
    conn: &mut PgConnection,
    id: Uuid,
    from: &[OrderStatus],
    to: OrderStatus,
    actor: Option<Uuid>,
    note: Option<&str>,
    cancel_reason: Option<&str>,
) -> Result<Option<OrderStatus>, sqlx::Error> {
    let previous = sqlx::query_scalar!(
        r#"
        UPDATE orders o SET status = $3, cancel_reason = coalesce($4, o.cancel_reason)
        FROM (SELECT id, status FROM orders WHERE id = $1 FOR UPDATE) prev
        WHERE o.id = prev.id AND prev.status = ANY($2)
        RETURNING prev.status AS "previous: OrderStatus"
        "#,
        id,
        from as &[OrderStatus],
        to as OrderStatus,
        cancel_reason,
    )
    .fetch_optional(&mut *conn)
    .await?;

    if let Some(prev) = previous {
        sqlx::query!(
            r#"
            INSERT INTO order_status_events (order_id, from_status, to_status, actor_user_id, note)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            id,
            prev as OrderStatus,
            to as OrderStatus,
            actor,
            note,
        )
        .execute(&mut *conn)
        .await?;
    }
    Ok(previous)
}

pub async fn set_payment_status<'e>(
    db: impl PgExecutor<'e>,
    id: Uuid,
    status: PaymentStatus,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE orders SET payment_status = $2 WHERE id = $1",
        id,
        status as PaymentStatus
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn set_gateway_order_id<'e>(
    db: impl PgExecutor<'e>,
    id: Uuid,
    gateway_order_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE orders SET gateway_order_id = $2 WHERE id = $1",
        id,
        gateway_order_id
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Conditional decrement: false (and nothing changed) if stock is short.
pub async fn decrement_stock(
    conn: &mut PgConnection,
    store_id: Uuid,
    product_id: Uuid,
    qty: i32,
) -> Result<bool, sqlx::Error> {
    let res = sqlx::query!(
        r#"
        UPDATE store_inventory SET quantity = quantity - $3
        WHERE store_id = $1 AND product_id = $2 AND quantity >= $3
        "#,
        store_id,
        product_id,
        qty,
    )
    .execute(&mut *conn)
    .await?;
    Ok(res.rows_affected() == 1)
}

pub async fn increment_stock(
    conn: &mut PgConnection,
    store_id: Uuid,
    product_id: Uuid,
    qty: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE store_inventory SET quantity = quantity + $3 WHERE store_id = $1 AND product_id = $2",
        store_id,
        product_id,
        qty,
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Records a webhook delivery; false if this event id was seen before.
pub async fn record_payment_event<'e>(
    db: impl PgExecutor<'e>,
    provider: &str,
    event_id: &str,
    event_type: &str,
    order_id: Option<Uuid>,
    payload: &serde_json::Value,
) -> Result<bool, sqlx::Error> {
    let res = sqlx::query!(
        r#"
        INSERT INTO payment_events (provider, event_id, event_type, order_id, payload)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (provider, event_id) DO NOTHING
        "#,
        provider,
        event_id,
        event_type,
        order_id,
        payload,
    )
    .execute(db)
    .await?;
    Ok(res.rows_affected() == 1)
}

/// Stores with pending reservations to sweep (all active stores).
pub async fn active_store_ids<'e>(db: impl PgExecutor<'e>) -> Result<Vec<Uuid>, sqlx::Error> {
    sqlx::query_scalar!("SELECT id FROM dark_stores WHERE is_active")
        .fetch_all(db)
        .await
}

pub struct BoardRow {
    pub id: Uuid,
    pub number: i64,
    pub status: OrderStatus,
    pub store_code: String,
    pub payment_method: PaymentMethod,
    pub total_paise: i64,
    pub item_count: i32,
    pub rider_phone: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Orders still in flight, oldest first (admin live board).
pub async fn board<'e>(
    db: impl PgExecutor<'e>,
    store: Option<Uuid>,
) -> Result<Vec<BoardRow>, sqlx::Error> {
    sqlx::query_as!(
        BoardRow,
        r#"
        SELECT o.id, o.number, o.status AS "status: OrderStatus", s.code AS store_code,
               o.payment_method AS "payment_method: PaymentMethod", o.total_paise,
               (SELECT sum(quantity) FROM order_items i WHERE i.order_id = o.id)::int AS "item_count!",
               (SELECT u.phone FROM deliveries d JOIN users u ON u.id = d.rider_id
                WHERE d.order_id = o.id AND d.delivered_at IS NULL AND d.ended_at IS NULL) AS rider_phone,
               o.created_at, o.updated_at
        FROM orders o JOIN dark_stores s ON s.id = o.store_id
        WHERE o.status NOT IN ('DELIVERED', 'CANCELLED', 'PARTIALLY_FULFILLED')
          AND ($1::uuid IS NULL OR o.store_id = $1)
        ORDER BY o.created_at, o.id
        LIMIT 500
        "#,
        store,
    )
    .fetch_all(db)
    .await
}

pub struct MetricsRow {
    pub orders: i64,
    pub delivered: i64,
    pub cancelled: i64,
    pub gmv_paise: i64,
    pub avg_delivery_minutes: Option<f64>,
    pub riders_online: i64,
}

/// Today's numbers (India time), optionally for one store.
pub async fn metrics_today<'e>(
    db: impl PgExecutor<'e>,
    store: Option<Uuid>,
) -> Result<MetricsRow, sqlx::Error> {
    sqlx::query_as!(
        MetricsRow,
        r#"
        WITH today AS (
            SELECT * FROM orders
            WHERE created_at >= date_trunc('day', now() AT TIME ZONE 'Asia/Kolkata') AT TIME ZONE 'Asia/Kolkata'
              AND ($1::uuid IS NULL OR store_id = $1)
        )
        SELECT
            (SELECT count(*) FROM today) AS "orders!",
            (SELECT count(*) FROM today WHERE status IN ('DELIVERED', 'PARTIALLY_FULFILLED')) AS "delivered!",
            (SELECT count(*) FROM today WHERE status = 'CANCELLED') AS "cancelled!",
            (SELECT coalesce(sum(total_paise), 0) FROM today
             WHERE status IN ('DELIVERED', 'PARTIALLY_FULFILLED'))::bigint AS "gmv_paise!",
            (SELECT avg(extract(epoch FROM d.delivered_at - t.created_at) / 60)
             FROM today t JOIN deliveries d ON d.order_id = t.id AND d.delivered_at IS NOT NULL)::float8 AS avg_delivery_minutes,
            (SELECT count(*) FROM rider_profiles p JOIN users u ON u.id = p.user_id
             WHERE p.is_online AND u.role = 'RIDER' AND ($1::uuid IS NULL OR u.store_id = $1)) AS "riders_online!"
        "#,
        store,
    )
    .fetch_one(db)
    .await
}
