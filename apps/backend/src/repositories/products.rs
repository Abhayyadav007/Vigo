use sqlx::PgExecutor;
use uuid::Uuid;

use crate::models::catalog::Product;

pub struct ProductInput<'a> {
    pub category_id: Uuid,
    pub name: &'a str,
    pub slug: &'a str,
    pub brand: Option<&'a str>,
    pub description: Option<&'a str>,
    pub unit_label: &'a str,
    pub barcode: Option<&'a str>,
    pub mrp_paise: i64,
    pub price_paise: i64,
    pub image_urls: &'a [String],
    pub is_active: bool,
}

pub async fn create<'e>(
    db: impl PgExecutor<'e>,
    p: &ProductInput<'_>,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        INSERT INTO products (category_id, name, slug, brand, description, unit_label, barcode,
                              mrp_paise, price_paise, image_urls, is_active)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        RETURNING id
        "#,
        p.category_id,
        p.name,
        p.slug,
        p.brand,
        p.description,
        p.unit_label,
        p.barcode,
        p.mrp_paise,
        p.price_paise,
        p.image_urls,
        p.is_active,
    )
    .fetch_one(db)
    .await
}

pub async fn update<'e>(
    db: impl PgExecutor<'e>,
    id: Uuid,
    p: &ProductInput<'_>,
) -> Result<bool, sqlx::Error> {
    let res = sqlx::query!(
        r#"
        UPDATE products SET category_id = $2, name = $3, slug = $4, brand = $5, description = $6,
                            unit_label = $7, barcode = $8, mrp_paise = $9, price_paise = $10,
                            image_urls = $11, is_active = $12
        WHERE id = $1
        "#,
        id,
        p.category_id,
        p.name,
        p.slug,
        p.brand,
        p.description,
        p.unit_label,
        p.barcode,
        p.mrp_paise,
        p.price_paise,
        p.image_urls,
        p.is_active,
    )
    .execute(db)
    .await?;
    Ok(res.rows_affected() == 1)
}

pub async fn find<'e>(db: impl PgExecutor<'e>, id: Uuid) -> Result<Option<Product>, sqlx::Error> {
    sqlx::query_as!(
        Product,
        r#"
        SELECT p.id, p.category_id, c.name AS category_name, p.name, p.slug, p.brand,
               p.description, p.unit_label, p.barcode, p.mrp_paise, p.price_paise,
               p.image_urls, p.is_active, p.created_at, p.updated_at
        FROM products p JOIN categories c ON c.id = p.category_id
        WHERE p.id = $1
        "#,
        id,
    )
    .fetch_optional(db)
    .await
}

pub struct ProductFilter<'a> {
    pub q: Option<&'a str>,
    pub category_id: Option<Uuid>,
    pub is_active: Option<bool>,
    pub limit: i64,
    pub offset: i64,
}

pub async fn list<'e>(
    db: impl PgExecutor<'e>,
    f: &ProductFilter<'_>,
) -> Result<(Vec<Product>, i64), sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT p.id, p.category_id, c.name AS category_name, p.name, p.slug, p.brand,
               p.description, p.unit_label, p.barcode, p.mrp_paise, p.price_paise,
               p.image_urls, p.is_active, p.created_at, p.updated_at,
               count(*) OVER () AS "total!"
        FROM products p JOIN categories c ON c.id = p.category_id
        WHERE ($1::text IS NULL
               OR p.barcode = $1
               OR (p.name || ' ' || coalesce(p.brand, '')) ILIKE '%' || $1 || '%'
               OR word_similarity($1, p.name || ' ' || coalesce(p.brand, '')) >= 0.4)
          AND ($2::uuid IS NULL OR p.category_id = $2)
          AND ($3::bool IS NULL OR p.is_active = $3)
        ORDER BY p.name, p.id
        LIMIT $4 OFFSET $5
        "#,
        f.q,
        f.category_id,
        f.is_active,
        f.limit,
        f.offset,
    )
    .fetch_all(db)
    .await?;
    let total = rows.first().map_or(0, |r| r.total);
    let products = rows
        .into_iter()
        .map(|r| Product {
            id: r.id,
            category_id: r.category_id,
            category_name: r.category_name,
            name: r.name,
            slug: r.slug,
            brand: r.brand,
            description: r.description,
            unit_label: r.unit_label,
            barcode: r.barcode,
            mrp_paise: r.mrp_paise,
            price_paise: r.price_paise,
            image_urls: r.image_urls,
            is_active: r.is_active,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
        .collect();
    Ok((products, total))
}

/// Mrp of a product, for validating store price overrides.
pub async fn mrp<'e>(db: impl PgExecutor<'e>, id: Uuid) -> Result<Option<i64>, sqlx::Error> {
    sqlx::query_scalar!("SELECT mrp_paise FROM products WHERE id = $1", id)
        .fetch_optional(db)
        .await
}

/// Highest store override for a product, so an MRP cut can't leave an
/// override above the new MRP.
pub async fn max_override<'e>(
    db: impl PgExecutor<'e>,
    id: Uuid,
) -> Result<Option<i64>, sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT max(price_override_paise) FROM store_inventory WHERE product_id = $1",
        id
    )
    .fetch_one(db)
    .await
}
