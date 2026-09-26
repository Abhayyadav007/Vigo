use sqlx::PgExecutor;
use uuid::Uuid;

use crate::models::catalog::InventoryRow;

pub struct InventoryInput<'a> {
    pub quantity: i32,
    pub bin_location: Option<&'a str>,
    pub price_override_paise: Option<i64>,
    pub is_available: bool,
}

pub async fn upsert<'e>(
    db: impl PgExecutor<'e>,
    store_id: Uuid,
    product_id: Uuid,
    i: &InventoryInput<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO store_inventory (store_id, product_id, quantity, bin_location,
                                     price_override_paise, is_available)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (store_id, product_id) DO UPDATE SET
            quantity = EXCLUDED.quantity,
            bin_location = EXCLUDED.bin_location,
            price_override_paise = EXCLUDED.price_override_paise,
            is_available = EXCLUDED.is_available
        "#,
        store_id,
        product_id,
        i.quantity,
        i.bin_location,
        i.price_override_paise,
        i.is_available,
    )
    .execute(db)
    .await?;
    Ok(())
}

pub struct InventoryFilter<'a> {
    pub q: Option<&'a str>,
    pub stocked: Option<bool>,
    pub limit: i64,
    pub offset: i64,
}

/// Every product, joined with this store's inventory row if it has one.
pub async fn list_for_store<'e>(
    db: impl PgExecutor<'e>,
    store_id: Uuid,
    f: &InventoryFilter<'_>,
) -> Result<(Vec<InventoryRow>, i64), sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT p.id AS product_id, p.name AS product_name, p.brand, p.unit_label, p.barcode,
               p.image_urls, c.name AS category_name, p.mrp_paise, p.price_paise,
               (i.product_id IS NOT NULL) AS "stocked!",
               coalesce(i.quantity, 0) AS "quantity!",
               i.bin_location AS "bin_location?",
               i.price_override_paise AS "price_override_paise?",
               coalesce(i.is_available, false) AS "is_available!",
               i.updated_at AS "updated_at?",
               count(*) OVER () AS "total!"
        FROM products p
        JOIN categories c ON c.id = p.category_id
        LEFT JOIN store_inventory i ON i.product_id = p.id AND i.store_id = $1
        WHERE ($2::text IS NULL
               OR p.barcode = $2
               OR i.bin_location ILIKE $2 || '%'
               OR (p.name || ' ' || coalesce(p.brand, '')) ILIKE '%' || $2 || '%')
          AND ($3::bool IS NULL OR (i.product_id IS NOT NULL) = $3)
        ORDER BY (i.product_id IS NULL), i.bin_location NULLS LAST, p.name, p.id
        LIMIT $4 OFFSET $5
        "#,
        store_id,
        f.q,
        f.stocked,
        f.limit,
        f.offset,
    )
    .fetch_all(db)
    .await?;
    let total = rows.first().map_or(0, |r| r.total);
    let items = rows
        .into_iter()
        .map(|r| InventoryRow {
            product_id: r.product_id,
            product_name: r.product_name,
            brand: r.brand,
            unit_label: r.unit_label,
            barcode: r.barcode,
            image_urls: r.image_urls,
            category_name: r.category_name,
            mrp_paise: r.mrp_paise,
            price_paise: r.price_paise,
            stocked: r.stocked,
            quantity: r.quantity,
            bin_location: r.bin_location,
            price_override_paise: r.price_override_paise,
            is_available: r.is_available,
            updated_at: r.updated_at,
        })
        .collect();
    Ok((items, total))
}

pub async fn find<'e>(
    db: impl PgExecutor<'e>,
    store_id: Uuid,
    product_id: Uuid,
) -> Result<Option<InventoryRow>, sqlx::Error> {
    sqlx::query_as!(
        InventoryRow,
        r#"
        SELECT p.id AS product_id, p.name AS product_name, p.brand, p.unit_label, p.barcode,
               p.image_urls, c.name AS category_name, p.mrp_paise, p.price_paise,
               true AS "stocked!", i.quantity, i.bin_location, i.price_override_paise,
               i.is_available, i.updated_at AS "updated_at?"
        FROM store_inventory i
        JOIN products p ON p.id = i.product_id
        JOIN categories c ON c.id = p.category_id
        WHERE i.store_id = $1 AND i.product_id = $2
        "#,
        store_id,
        product_id,
    )
    .fetch_optional(db)
    .await
}

/// Every product's quantity at a store (reconciliation).
pub async fn levels<'e>(
    db: impl PgExecutor<'e>,
    store_id: Uuid,
) -> Result<Vec<(Uuid, i32)>, sqlx::Error> {
    let rows = sqlx::query!(
        "SELECT product_id, quantity FROM store_inventory WHERE store_id = $1",
        store_id
    )
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| (r.product_id, r.quantity))
        .collect())
}
