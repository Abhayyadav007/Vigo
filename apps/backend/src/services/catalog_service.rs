//! Admin catalog management: categories, products and per-store inventory.

use uuid::Uuid;

use crate::{
    dto::catalog::{CategoryRequest, InventoryRequest, ProductRequest},
    error::{AppError, AppResult, On, map_constraint},
    models::catalog::{Category, InventoryRow, Product},
    repositories::{
        categories::{self, CategoryInput},
        inventory::{self, InventoryInput},
        products::{self, ProductInput},
        stores,
    },
    services::slug::slugify,
    state::AppState,
};

const CATEGORY_CONSTRAINTS: &[(&str, On, &str)] = &[
    (
        "categories_slug_key",
        On::Conflict,
        "a category with this slug already exists",
    ),
    (
        "categories_parent_id_fkey",
        On::Invalid,
        "parent category does not exist",
    ),
];

const PRODUCT_CONSTRAINTS: &[(&str, On, &str)] = &[
    (
        "products_slug_key",
        On::Conflict,
        "a product with this slug already exists",
    ),
    (
        "products_barcode_key",
        On::Conflict,
        "another product already has this barcode",
    ),
    (
        "products_category_id_fkey",
        On::Invalid,
        "category does not exist",
    ),
];

// ---------- categories ----------

pub async fn create_category(state: &AppState, req: &CategoryRequest) -> AppResult<Category> {
    let slug = category_slug(req)?;
    categories::create(&state.db, &category_input(req, &slug))
        .await
        .map_err(|e| map_constraint(e, CATEGORY_CONSTRAINTS))
}

pub async fn update_category(
    state: &AppState,
    id: Uuid,
    req: &CategoryRequest,
) -> AppResult<Category> {
    if let Some(parent) = req.parent_id
        && categories::is_descendant_or_self(&state.db, id, parent).await?
    {
        return Err(AppError::Validation(
            "a category can't be moved under itself or its own subcategory".into(),
        ));
    }
    let slug = category_slug(req)?;
    categories::update(&state.db, id, &category_input(req, &slug))
        .await
        .map_err(|e| map_constraint(e, CATEGORY_CONSTRAINTS))?
        .ok_or(AppError::NotFound("category"))
}

fn category_slug(req: &CategoryRequest) -> AppResult<String> {
    let slug = req.slug.clone().unwrap_or_else(|| slugify(&[&req.name]));
    if slug.is_empty() {
        return Err(AppError::Validation(
            "slug is required for this name".into(),
        ));
    }
    Ok(slug)
}

fn category_input<'a>(req: &'a CategoryRequest, slug: &'a str) -> CategoryInput<'a> {
    CategoryInput {
        parent_id: req.parent_id,
        name: req.name.trim(),
        slug,
        image_url: req.image_url.as_deref(),
        sort_order: req.sort_order,
        is_active: req.is_active,
    }
}

// ---------- products ----------

pub async fn create_product(state: &AppState, req: &ProductRequest) -> AppResult<Product> {
    let slug = product_slug(req)?;
    check_prices(req.mrp_paise, req.price_paise)?;
    let id = products::create(&state.db, &product_input(req, &slug))
        .await
        .map_err(|e| map_constraint(e, PRODUCT_CONSTRAINTS))?;
    products::find(&state.db, id)
        .await?
        .ok_or(AppError::NotFound("product"))
}

pub async fn update_product(
    state: &AppState,
    id: Uuid,
    req: &ProductRequest,
) -> AppResult<Product> {
    let slug = product_slug(req)?;
    check_prices(req.mrp_paise, req.price_paise)?;
    if let Some(max) = products::max_override(&state.db, id).await?
        && max > req.mrp_paise
    {
        return Err(AppError::Validation(format!(
            "a store sells this at {}, above the new MRP; lower that store price first",
            rupees(max)
        )));
    }
    let found = products::update(&state.db, id, &product_input(req, &slug))
        .await
        .map_err(|e| map_constraint(e, PRODUCT_CONSTRAINTS))?;
    if !found {
        return Err(AppError::NotFound("product"));
    }
    products::find(&state.db, id)
        .await?
        .ok_or(AppError::NotFound("product"))
}

fn product_slug(req: &ProductRequest) -> AppResult<String> {
    let slug = req.slug.clone().unwrap_or_else(|| {
        slugify(&[
            req.brand.as_deref().unwrap_or(""),
            &req.name,
            &req.unit_label,
        ])
    });
    if slug.is_empty() {
        return Err(AppError::Validation(
            "slug is required for this name".into(),
        ));
    }
    Ok(slug)
}

fn check_prices(mrp: i64, price: i64) -> AppResult<()> {
    if price > mrp {
        return Err(AppError::Validation(
            "selling price can't be above MRP".into(),
        ));
    }
    Ok(())
}

fn product_input<'a>(req: &'a ProductRequest, slug: &'a str) -> ProductInput<'a> {
    ProductInput {
        category_id: req.category_id,
        name: req.name.trim(),
        slug,
        brand: req.brand.as_deref().map(str::trim),
        description: req
            .description
            .as_deref()
            .map(str::trim)
            .filter(|d| !d.is_empty()),
        unit_label: req.unit_label.trim(),
        barcode: req.barcode.as_deref(),
        mrp_paise: req.mrp_paise,
        price_paise: req.price_paise,
        image_urls: &req.image_urls,
        is_active: req.is_active,
    }
}

// ---------- inventory ----------

pub async fn set_inventory(
    state: &AppState,
    store_id: Uuid,
    product_id: Uuid,
    req: &InventoryRequest,
) -> AppResult<InventoryRow> {
    if stores::find(&state.db, store_id).await?.is_none() {
        return Err(AppError::NotFound("store"));
    }
    let mrp = products::mrp(&state.db, product_id)
        .await?
        .ok_or(AppError::NotFound("product"))?;
    if req.price_override_paise.is_some_and(|p| p > mrp) {
        return Err(AppError::Validation(format!(
            "store price can't be above MRP ({})",
            rupees(mrp)
        )));
    }
    // TODO(phase-4): mirror quantity changes into the Redis availability cache.
    let input = InventoryInput {
        quantity: req.quantity,
        bin_location: req.bin_location.as_deref().map(str::trim),
        price_override_paise: req.price_override_paise,
        is_available: req.is_available,
    };
    inventory::upsert(&state.db, store_id, product_id, &input).await?;
    inventory::find(&state.db, store_id, product_id)
        .await?
        .ok_or(AppError::NotFound("inventory"))
}

/// Paise to "₹1,234.50" (Indian digit grouping).
pub fn rupees(paise: i64) -> String {
    let rupees = paise / 100;
    let p = paise % 100;
    let digits = rupees.to_string();
    let grouped = if digits.len() <= 3 {
        digits
    } else {
        let (head, tail) = digits.split_at(digits.len() - 3);
        let mut out = String::new();
        for (i, ch) in head.chars().enumerate() {
            if i > 0 && (head.len() - i) % 2 == 0 {
                out.push(',');
            }
            out.push(ch);
        }
        format!("{out},{tail}")
    };
    if p == 0 {
        format!("₹{grouped}")
    } else {
        format!("₹{grouped}.{p:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::rupees;

    #[test]
    fn formats_indian_rupees() {
        assert_eq!(rupees(5_000), "₹50");
        assert_eq!(rupees(12_345), "₹123.45");
        assert_eq!(rupees(123_456_700), "₹12,34,567");
        assert_eq!(rupees(100_000), "₹1,000");
    }
}
