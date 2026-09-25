use sqlx::PgExecutor;
use uuid::Uuid;

use crate::models::order::CartLineRow;

/// Sets a line's quantity, creating the cart if needed; 0 removes the line.
pub async fn set_item<'e>(
    db: impl PgExecutor<'e>,
    user_id: Uuid,
    store_id: Uuid,
    product_id: Uuid,
    quantity: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        WITH cart AS (
            INSERT INTO carts (user_id, store_id) VALUES ($1, $2)
            ON CONFLICT (user_id, store_id) DO UPDATE SET updated_at = now()
            RETURNING id
        ),
        removed AS (
            DELETE FROM cart_items
            WHERE $4 = 0 AND cart_id = (SELECT id FROM cart) AND product_id = $3
        )
        INSERT INTO cart_items (cart_id, product_id, quantity)
        SELECT id, $3, $4 FROM cart WHERE $4 > 0
        ON CONFLICT (cart_id, product_id) DO UPDATE SET quantity = EXCLUDED.quantity
        "#,
        user_id,
        store_id,
        product_id,
        quantity,
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn clear<'e>(
    db: impl PgExecutor<'e>,
    user_id: Uuid,
    store_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        DELETE FROM cart_items
        WHERE cart_id = (SELECT id FROM carts WHERE user_id = $1 AND store_id = $2)
        "#,
        user_id,
        store_id,
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Cart lines with live price/stock from the store catalog. `sellable` is
/// false if the store, product or category was deactivated or the store no
/// longer offers the product.
pub async fn lines<'e>(
    db: impl PgExecutor<'e>,
    user_id: Uuid,
    store_id: Uuid,
) -> Result<Vec<CartLineRow>, sqlx::Error> {
    sqlx::query_as!(
        CartLineRow,
        r#"
        SELECT p.id AS product_id, p.name, p.brand, p.unit_label, p.image_urls, ci.quantity,
               coalesce(i.price_override_paise, p.price_paise) AS "price_paise!",
               p.mrp_paise,
               coalesce(i.quantity, 0) AS "stock!",
               (i.is_available IS TRUE AND p.is_active AND c.is_active AND s.is_active) AS "sellable!"
        FROM carts ca
        JOIN cart_items ci ON ci.cart_id = ca.id
        JOIN products p ON p.id = ci.product_id
        JOIN categories c ON c.id = p.category_id
        JOIN dark_stores s ON s.id = ca.store_id
        LEFT JOIN store_inventory i ON i.store_id = ca.store_id AND i.product_id = p.id
        WHERE ca.user_id = $1 AND ca.store_id = $2
        ORDER BY ci.added_at, p.id
        "#,
        user_id,
        store_id,
    )
    .fetch_all(db)
    .await
}

/// Postgres quantities for loading the Redis stock mirror.
pub async fn stock_levels<'e>(
    db: impl PgExecutor<'e>,
    store_id: Uuid,
    product_ids: &[Uuid],
) -> Result<Vec<(Uuid, i32)>, sqlx::Error> {
    let rows = sqlx::query!(
        "SELECT product_id, quantity FROM store_inventory WHERE store_id = $1 AND product_id = ANY($2)",
        store_id,
        product_ids,
    )
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| (r.product_id, r.quantity))
        .collect())
}
