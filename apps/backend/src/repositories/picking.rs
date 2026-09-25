use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgExecutor};
use uuid::Uuid;

use crate::models::order::OrderStatus;

pub struct QueueRow {
    pub id: Uuid,
    pub number: i64,
    pub status: OrderStatus,
    pub item_count: i32,
    pub line_count: i32,
    pub lines_done: i32,
    pub picker_id: Option<Uuid>,
    pub staging_slot: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Orders a store's pickers work on, oldest first.
pub async fn queue<'e>(
    db: impl PgExecutor<'e>,
    store_id: Uuid,
) -> Result<Vec<QueueRow>, sqlx::Error> {
    sqlx::query_as!(
        QueueRow,
        r#"
        SELECT o.id, o.number, o.status AS "status: OrderStatus",
               (SELECT sum(quantity) FROM order_items i WHERE i.order_id = o.id)::int AS "item_count!",
               (SELECT count(*) FROM order_items i WHERE i.order_id = o.id)::int AS "line_count!",
               (SELECT count(*) FROM order_items i
                WHERE i.order_id = o.id AND i.picked_quantity IS NOT NULL)::int AS "lines_done!",
               o.picker_id, o.staging_slot, o.created_at
        FROM orders o
        WHERE o.store_id = $1 AND o.status IN ('CONFIRMED', 'PICKING', 'PACKED')
        ORDER BY o.created_at, o.id
        "#,
        store_id,
    )
    .fetch_all(db)
    .await
}

pub struct PickHeader {
    pub id: Uuid,
    pub number: i64,
    pub store_id: Uuid,
    pub status: OrderStatus,
    pub picker_id: Option<Uuid>,
    pub bag_count: Option<i32>,
    pub staging_slot: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub async fn header<'e>(
    db: impl PgExecutor<'e>,
    id: Uuid,
) -> Result<Option<PickHeader>, sqlx::Error> {
    sqlx::query_as!(
        PickHeader,
        r#"
        SELECT id, number, store_id, status AS "status: OrderStatus", picker_id, bag_count,
               staging_slot, created_at
        FROM orders WHERE id = $1
        "#,
        id,
    )
    .fetch_optional(db)
    .await
}

pub struct PickLineRow {
    pub product_id: Uuid,
    pub name: String,
    pub brand: Option<String>,
    pub unit_label: String,
    pub image_url: Option<String>,
    pub barcode: Option<String>,
    pub bin_location: Option<String>,
    pub quantity: i32,
    pub picked_quantity: Option<i32>,
    pub unit_price_paise: i64,
    pub unit_mrp_paise: i64,
}

/// Lines with the store's current bin locations (unsorted; sorted in Rust).
pub async fn lines<'e>(
    db: impl PgExecutor<'e>,
    order_id: Uuid,
) -> Result<Vec<PickLineRow>, sqlx::Error> {
    sqlx::query_as!(
        PickLineRow,
        r#"
        SELECT oi.product_id, oi.name, oi.brand, oi.unit_label, oi.image_url, p.barcode,
               si.bin_location AS "bin_location?", oi.quantity, oi.picked_quantity,
               oi.unit_price_paise, oi.unit_mrp_paise
        FROM order_items oi
        JOIN orders o ON o.id = oi.order_id
        JOIN products p ON p.id = oi.product_id
        LEFT JOIN store_inventory si ON si.store_id = o.store_id AND si.product_id = oi.product_id
        WHERE oi.order_id = $1
        "#,
        order_id,
    )
    .fetch_all(db)
    .await
}

/// Claims a CONFIRMED order for `picker_id` (the status CAS already ran in
/// the same transaction).
pub async fn set_picker(
    conn: &mut PgConnection,
    id: Uuid,
    picker_id: Option<Uuid>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE orders SET picker_id = $2,
            picking_started_at = CASE WHEN $2::uuid IS NULL THEN NULL ELSE now() END
        WHERE id = $1
        "#,
        id,
        picker_id,
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub enum ScanOutcome {
    /// Counted one unit of this product.
    Counted(Uuid),
    /// The product is in the order but already fully picked.
    AlreadyComplete(String),
    /// No line in this order has that barcode.
    NotInOrder,
}

/// Atomically counts one unit for the line whose product has `barcode`.
pub async fn scan<'e>(
    db: impl PgExecutor<'e> + Copy,
    order_id: Uuid,
    barcode: &str,
) -> Result<ScanOutcome, sqlx::Error> {
    let counted = sqlx::query_scalar!(
        r#"
        UPDATE order_items oi SET picked_quantity = coalesce(oi.picked_quantity, 0) + 1
        FROM products p
        WHERE oi.order_id = $1 AND p.id = oi.product_id AND p.barcode = $2
          AND coalesce(oi.picked_quantity, 0) < oi.quantity
        RETURNING oi.product_id
        "#,
        order_id,
        barcode,
    )
    .fetch_optional(db)
    .await?;
    if let Some(product_id) = counted {
        return Ok(ScanOutcome::Counted(product_id));
    }
    let complete = sqlx::query_scalar!(
        r#"
        SELECT oi.name FROM order_items oi JOIN products p ON p.id = oi.product_id
        WHERE oi.order_id = $1 AND p.barcode = $2
        "#,
        order_id,
        barcode,
    )
    .fetch_optional(db)
    .await?;
    Ok(complete.map_or(ScanOutcome::NotInOrder, ScanOutcome::AlreadyComplete))
}

/// Sets a line's count (0..=quantity). False if the line doesn't exist or the
/// count is out of range.
pub async fn set_picked<'e>(
    db: impl PgExecutor<'e>,
    order_id: Uuid,
    product_id: Uuid,
    picked: i32,
) -> Result<bool, sqlx::Error> {
    let res = sqlx::query!(
        r#"
        UPDATE order_items SET picked_quantity = $3
        WHERE order_id = $1 AND product_id = $2 AND $3 BETWEEN 0 AND quantity
        "#,
        order_id,
        product_id,
        picked,
    )
    .execute(db)
    .await?;
    Ok(res.rows_affected() == 1)
}

/// After packing: bill for what was actually picked, and record handoff info.
pub async fn finish_packing(
    conn: &mut PgConnection,
    id: Uuid,
    item_total_paise: i64,
    mrp_total_paise: i64,
    bag_count: i32,
    staging_slot: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE orders SET item_total_paise = $2, mrp_total_paise = $3,
            total_paise = $2 + delivery_fee_paise,
            bag_count = $4, staging_slot = $5, packed_at = now()
        WHERE id = $1
        "#,
        id,
        item_total_paise,
        mrp_total_paise,
        bag_count,
        staging_slot,
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// A picker couldn't find the item: the shelf is actually empty.
pub async fn zero_stock(
    conn: &mut PgConnection,
    store_id: Uuid,
    product_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE store_inventory SET quantity = 0 WHERE store_id = $1 AND product_id = $2",
        store_id,
        product_id,
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}
