//! Customer-facing reads. A product is visible in a store when the store,
//! product and category are active and the store has an available
//! inventory row for it. Out-of-stock items are listed last.

use sqlx::PgExecutor;
use uuid::Uuid;

use crate::models::catalog::{CatalogCategoryRow, CatalogProductRow};

pub async fn categories<'e>(
    db: impl PgExecutor<'e>,
    store_id: Uuid,
) -> Result<Vec<CatalogCategoryRow>, sqlx::Error> {
    sqlx::query_as!(
        CatalogCategoryRow,
        r#"
        SELECT c.id, c.name, c.slug, c.image_url, count(*) AS "product_count!"
        FROM store_inventory i
        JOIN dark_stores s ON s.id = i.store_id AND s.is_active
        JOIN products p ON p.id = i.product_id AND p.is_active
        JOIN categories c ON c.id = p.category_id AND c.is_active
        WHERE i.store_id = $1 AND i.is_available
        GROUP BY c.id
        ORDER BY c.sort_order, c.name
        "#,
        store_id,
    )
    .fetch_all(db)
    .await
}

pub struct CatalogFilter<'a> {
    pub category_id: Option<Uuid>,
    pub q: Option<&'a str>,
    pub limit: i64,
    pub offset: i64,
}

pub async fn products<'e>(
    db: impl PgExecutor<'e>,
    store_id: Uuid,
    f: &CatalogFilter<'_>,
) -> Result<(Vec<CatalogProductRow>, i64), sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT p.id, p.category_id, p.name, p.brand, p.unit_label, p.image_urls, p.mrp_paise,
               coalesce(i.price_override_paise, p.price_paise) AS "price_paise!",
               i.quantity,
               count(*) OVER () AS "total!"
        FROM store_inventory i
        JOIN dark_stores s ON s.id = i.store_id AND s.is_active
        JOIN products p ON p.id = i.product_id AND p.is_active
        JOIN categories c ON c.id = p.category_id AND c.is_active
        WHERE i.store_id = $1 AND i.is_available
          AND ($2::uuid IS NULL OR p.category_id = $2)
          AND ($3::text IS NULL
               OR (p.name || ' ' || coalesce(p.brand, '')) ILIKE '%' || $3 || '%'
               -- Typo tolerance: best-matching word ("panner" -> "Paneer").
               OR word_similarity($3, p.name || ' ' || coalesce(p.brand, '')) >= 0.4)
        ORDER BY (i.quantity = 0),
                 CASE WHEN $3::text IS NULL THEN 0
                      ELSE 1 - word_similarity($3, p.name || ' ' || coalesce(p.brand, '')) END,
                 p.name, p.id
        LIMIT $4 OFFSET $5
        "#,
        store_id,
        f.category_id,
        f.q,
        f.limit,
        f.offset,
    )
    .fetch_all(db)
    .await?;
    let total = rows.first().map_or(0, |r| r.total);
    let products = rows
        .into_iter()
        .map(|r| CatalogProductRow {
            id: r.id,
            category_id: r.category_id,
            name: r.name,
            brand: r.brand,
            unit_label: r.unit_label,
            image_urls: r.image_urls,
            mrp_paise: r.mrp_paise,
            price_paise: r.price_paise,
            quantity: r.quantity,
            description: None,
        })
        .collect();
    Ok((products, total))
}

pub async fn product<'e>(
    db: impl PgExecutor<'e>,
    store_id: Uuid,
    product_id: Uuid,
) -> Result<Option<CatalogProductRow>, sqlx::Error> {
    sqlx::query_as!(
        CatalogProductRow,
        r#"
        SELECT p.id, p.category_id, p.name, p.brand, p.unit_label, p.image_urls, p.mrp_paise,
               coalesce(i.price_override_paise, p.price_paise) AS "price_paise!",
               i.quantity, p.description
        FROM store_inventory i
        JOIN dark_stores s ON s.id = i.store_id AND s.is_active
        JOIN products p ON p.id = i.product_id AND p.is_active
        JOIN categories c ON c.id = p.category_id AND c.is_active
        WHERE i.store_id = $1 AND i.product_id = $2 AND i.is_available
        "#,
        store_id,
        product_id,
    )
    .fetch_optional(db)
    .await
}
