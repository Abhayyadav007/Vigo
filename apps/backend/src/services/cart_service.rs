use uuid::Uuid;

use crate::{
    dto::{
        customer::MAX_PER_ITEM,
        order::{CartLine, CartResponse},
    },
    error::{AppError, AppResult},
    repositories::{carts, catalog},
    services::pricing,
    state::AppState,
};

pub async fn get(state: &AppState, user_id: Uuid, store_id: Uuid) -> AppResult<CartResponse> {
    let rows = carts::lines(&state.db, user_id, store_id).await?;
    let mut item_total = 0;
    let mut mrp_total = 0;
    let mut item_count = 0;
    let items: Vec<CartLine> = rows
        .into_iter()
        .map(|r| {
            let available = r.sellable && r.stock >= r.quantity;
            if available {
                item_total += r.price_paise * i64::from(r.quantity);
                mrp_total += r.mrp_paise * i64::from(r.quantity);
                item_count += r.quantity;
            }
            CartLine {
                product_id: r.product_id,
                name: r.name,
                brand: r.brand,
                unit_label: r.unit_label,
                image_url: r.image_urls.into_iter().next(),
                quantity: r.quantity,
                unit_price_paise: r.price_paise,
                unit_mrp_paise: r.mrp_paise,
                line_total_paise: r.price_paise * i64::from(r.quantity),
                available,
                max_quantity: if r.sellable {
                    r.stock.clamp(0, MAX_PER_ITEM)
                } else {
                    0
                },
            }
        })
        .collect();
    let can_checkout = !items.is_empty() && items.iter().all(|l| l.available);
    Ok(CartResponse {
        store_id,
        items,
        item_count,
        bill: pricing::bill(item_total, mrp_total),
        can_checkout,
    })
}

pub async fn set_item(
    state: &AppState,
    user_id: Uuid,
    store_id: Uuid,
    product_id: Uuid,
    quantity: i32,
) -> AppResult<CartResponse> {
    if quantity > 0 {
        let product = catalog::product(&state.db, store_id, product_id)
            .await?
            .ok_or(AppError::NotFound("product"))?;
        if product.quantity == 0 {
            return Err(AppError::Conflict(format!(
                "{} is out of stock",
                product.name
            )));
        }
        if quantity > product.quantity {
            return Err(AppError::Validation(format!(
                "only {} of {} left",
                product.quantity, product.name
            )));
        }
    }
    carts::set_item(&state.db, user_id, store_id, product_id, quantity).await?;
    get(state, user_id, store_id).await
}

pub async fn clear(state: &AppState, user_id: Uuid, store_id: Uuid) -> AppResult<CartResponse> {
    carts::clear(&state.db, user_id, store_id).await?;
    get(state, user_id, store_id).await
}
