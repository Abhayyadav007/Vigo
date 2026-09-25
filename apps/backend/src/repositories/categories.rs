use sqlx::PgExecutor;
use uuid::Uuid;

use crate::models::catalog::Category;

pub struct CategoryInput<'a> {
    pub parent_id: Option<Uuid>,
    pub name: &'a str,
    pub slug: &'a str,
    pub image_url: Option<&'a str>,
    pub sort_order: i32,
    pub is_active: bool,
}

pub async fn create<'e>(
    db: impl PgExecutor<'e>,
    c: &CategoryInput<'_>,
) -> Result<Category, sqlx::Error> {
    sqlx::query_as!(
        Category,
        r#"
        INSERT INTO categories (parent_id, name, slug, image_url, sort_order, is_active)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, parent_id, name, slug, image_url, sort_order, is_active,
                  0::bigint AS "product_count!"
        "#,
        c.parent_id,
        c.name,
        c.slug,
        c.image_url,
        c.sort_order,
        c.is_active,
    )
    .fetch_one(db)
    .await
}

pub async fn update<'e>(
    db: impl PgExecutor<'e>,
    id: Uuid,
    c: &CategoryInput<'_>,
) -> Result<Option<Category>, sqlx::Error> {
    sqlx::query_as!(
        Category,
        r#"
        UPDATE categories SET parent_id = $2, name = $3, slug = $4, image_url = $5,
                              sort_order = $6, is_active = $7
        WHERE id = $1
        RETURNING id, parent_id, name, slug, image_url, sort_order, is_active,
                  (SELECT count(*) FROM products p WHERE p.category_id = categories.id) AS "product_count!"
        "#,
        id,
        c.parent_id,
        c.name,
        c.slug,
        c.image_url,
        c.sort_order,
        c.is_active,
    )
    .fetch_optional(db)
    .await
}

/// All categories (the tree is small), ordered for display.
pub async fn list_all<'e>(db: impl PgExecutor<'e>) -> Result<Vec<Category>, sqlx::Error> {
    sqlx::query_as!(
        Category,
        r#"
        SELECT c.id, c.parent_id, c.name, c.slug, c.image_url, c.sort_order, c.is_active,
               (SELECT count(*) FROM products p WHERE p.category_id = c.id) AS "product_count!"
        FROM categories c
        ORDER BY c.sort_order, c.name
        "#,
    )
    .fetch_all(db)
    .await
}

/// Would making `parent_id` the parent of `id` create a cycle?
pub async fn is_descendant_or_self<'e>(
    db: impl PgExecutor<'e>,
    id: Uuid,
    candidate_parent: Uuid,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        WITH RECURSIVE sub AS (
            SELECT id FROM categories WHERE id = $1
            UNION
            SELECT c.id FROM categories c JOIN sub ON c.parent_id = sub.id
        )
        SELECT EXISTS (SELECT 1 FROM sub WHERE id = $2) AS "exists!"
        "#,
        id,
        candidate_parent,
    )
    .fetch_one(db)
    .await
}
