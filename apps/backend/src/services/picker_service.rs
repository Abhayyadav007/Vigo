//! Dark-store picking: claim an order, count items by barcode (or by hand),
//! mark shortages, pack and stage it for the rider.

use std::cmp::Ordering;

use axum::http::StatusCode;
use uuid::Uuid;

use crate::{
    cache::inventory,
    dto::{
        order::display_number,
        picker::{PackRequest, PickLine, PickList, PickerQueueItem, ScanResult},
    },
    error::{AppError, AppResult},
    extractors::AuthUser,
    models::order::OrderStatus,
    repositories::{
        orders,
        picking::{self, PickHeader, PickLineRow, ScanOutcome},
    },
    services::{dispatch_service, order_service},
    state::AppState,
};

fn coded(status: StatusCode, code: &'static str, message: impl Into<String>) -> AppError {
    AppError::Coded {
        status,
        code,
        message: message.into(),
    }
}

/// The picker's store; pickers without one can't do anything.
fn store_of(picker: &AuthUser) -> AppResult<Uuid> {
    picker.store_id.ok_or(AppError::Forbidden {
        code: "NO_STORE",
        message: "you're not assigned to a store yet",
    })
}

/// The order, if it belongs to the picker's store (404 otherwise, so other
/// stores' orders aren't even confirmed to exist).
async fn header_in_store(state: &AppState, picker: &AuthUser, id: Uuid) -> AppResult<PickHeader> {
    let store = store_of(picker)?;
    picking::header(&state.db, id)
        .await?
        .filter(|h| h.store_id == store)
        .ok_or(AppError::NotFound("order"))
}

/// The order must be PICKING and claimed by this picker.
async fn mine(state: &AppState, picker: &AuthUser, id: Uuid) -> AppResult<PickHeader> {
    let h = header_in_store(state, picker, id).await?;
    if h.status != OrderStatus::Picking {
        return Err(coded(
            StatusCode::CONFLICT,
            "NOT_PICKING",
            format!("order is {}, not being picked", h.status.as_str()),
        ));
    }
    if h.picker_id != Some(picker.user_id) {
        return Err(coded(
            StatusCode::CONFLICT,
            "NOT_YOUR_ORDER",
            "another picker is working on this order",
        ));
    }
    Ok(h)
}

pub async fn queue(state: &AppState, picker: &AuthUser) -> AppResult<Vec<PickerQueueItem>> {
    let store = store_of(picker)?;
    let rows = picking::queue(&state.db, store).await?;
    Ok(rows
        .into_iter()
        .map(|r| PickerQueueItem {
            id: r.id,
            number: display_number(r.number),
            status: r.status,
            item_count: r.item_count,
            line_count: r.line_count,
            lines_done: r.lines_done,
            is_mine: r.picker_id == Some(picker.user_id),
            picker_id: r.picker_id,
            staging_slot: r.staging_slot,
            created_at: r.created_at,
        })
        .collect())
}

pub async fn pick_list(state: &AppState, picker: &AuthUser, id: Uuid) -> AppResult<PickList> {
    let h = header_in_store(state, picker, id).await?;
    build_pick_list(state, picker, h).await
}

async fn build_pick_list(
    state: &AppState,
    picker: &AuthUser,
    h: PickHeader,
) -> AppResult<PickList> {
    let mut rows = picking::lines(&state.db, h.id).await?;
    rows.sort_by(|a, b| {
        bin_order(a.bin_location.as_deref(), b.bin_location.as_deref())
            .then_with(|| a.name.cmp(&b.name))
    });
    let all_counted = rows.iter().all(|r| r.picked_quantity.is_some());
    let any_found = rows.iter().any(|r| r.picked_quantity.unwrap_or(0) > 0);
    Ok(PickList {
        id: h.id,
        number: display_number(h.number),
        status: h.status,
        picker_id: h.picker_id,
        is_mine: h.picker_id == Some(picker.user_id),
        lines: rows.into_iter().map(pick_line).collect(),
        can_pack: h.status == OrderStatus::Picking && all_counted && any_found,
        bag_count: h.bag_count,
        staging_slot: h.staging_slot,
        created_at: h.created_at,
    })
}

fn pick_line(r: PickLineRow) -> PickLine {
    PickLine {
        product_id: r.product_id,
        name: r.name,
        brand: r.brand,
        unit_label: r.unit_label,
        image_url: r.image_url,
        barcode: r.barcode,
        bin_location: r.bin_location,
        quantity: r.quantity,
        picked_quantity: r.picked_quantity,
    }
}

/// Walk order through the store: natural sort on the bin code (A-2 before
/// A-10), unlocated items last.
fn bin_order(a: Option<&str>, b: Option<&str>) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => natural_key(a).cmp(&natural_key(b)),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Chunk {
    Num(u64),
    Text(String),
}

fn natural_key(s: &str) -> Vec<Chunk> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut digits = false;
    let flush = |buf: &mut String, digits: bool, out: &mut Vec<Chunk>| {
        if !buf.is_empty() {
            out.push(if digits {
                Chunk::Num(buf.parse().unwrap_or(u64::MAX))
            } else {
                Chunk::Text(buf.to_ascii_uppercase())
            });
            buf.clear();
        }
    };
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            if ch.is_ascii_digit() != digits {
                flush(&mut buf, digits, &mut out);
                digits = ch.is_ascii_digit();
            }
            buf.push(ch);
        } else {
            flush(&mut buf, digits, &mut out);
        }
    }
    flush(&mut buf, digits, &mut out);
    out
}

/// CONFIRMED -> PICKING, claimed by this picker. First picker wins.
pub async fn start(state: &AppState, picker: &AuthUser, id: Uuid) -> AppResult<PickList> {
    let h = header_in_store(state, picker, id).await?;
    let mut tx = state.db.begin().await?;
    let prev = orders::set_status(
        &mut tx,
        id,
        &[OrderStatus::Confirmed],
        OrderStatus::Picking,
        Some(picker.user_id),
        Some("picking started"),
        None,
    )
    .await?;
    if prev.is_none() {
        tx.rollback().await?;
        if h.status == OrderStatus::Picking {
            return Err(coded(
                StatusCode::CONFLICT,
                "ALREADY_CLAIMED",
                "another picker already started this order",
            ));
        }
        return Err(order_service::invalid_transition(state, id, OrderStatus::Picking).await);
    }
    picking::set_picker(&mut tx, id, Some(picker.user_id)).await?;
    tx.commit().await?;
    order_service::publish_status(state, id, prev, OrderStatus::Picking).await;
    pick_list(state, picker, id).await
}

pub async fn scan(
    state: &AppState,
    picker: &AuthUser,
    id: Uuid,
    barcode: &str,
) -> AppResult<ScanResult> {
    mine(state, picker, id).await?;
    let barcode = barcode.trim();
    match picking::scan(&state.db, id, barcode).await? {
        ScanOutcome::Counted(product_id) => {
            let list = pick_list(state, picker, id).await?;
            let line = list
                .lines
                .iter()
                .find(|l| l.product_id == product_id)
                .ok_or(AppError::NotFound("line"))?;
            Ok(ScanResult {
                line_complete: line.picked_quantity == Some(line.quantity),
                line: PickLine {
                    product_id: line.product_id,
                    name: line.name.clone(),
                    brand: line.brand.clone(),
                    unit_label: line.unit_label.clone(),
                    image_url: line.image_url.clone(),
                    barcode: line.barcode.clone(),
                    bin_location: line.bin_location.clone(),
                    quantity: line.quantity,
                    picked_quantity: line.picked_quantity,
                },
                pick_list: list,
            })
        }
        ScanOutcome::AlreadyComplete(name) => Err(coded(
            StatusCode::CONFLICT,
            "LINE_COMPLETE",
            format!("all {name} already picked — put the extra back"),
        )),
        ScanOutcome::NotInOrder => Err(coded(
            StatusCode::UNPROCESSABLE_ENTITY,
            "SCAN_MISMATCH",
            "this item isn't in the order — check the product and try again",
        )),
    }
}

pub async fn set_picked(
    state: &AppState,
    picker: &AuthUser,
    id: Uuid,
    product_id: Uuid,
    picked: i32,
) -> AppResult<PickList> {
    mine(state, picker, id).await?;
    if !picking::set_picked(&state.db, id, product_id, picked).await? {
        return Err(AppError::Validation(
            "that line doesn't exist or the count is more than ordered".into(),
        ));
    }
    pick_list(state, picker, id).await
}

/// PICKING -> PACKED. The customer is billed for what was found; lines found
/// short zero the shelf so nobody else orders phantom stock.
pub async fn pack(
    state: &AppState,
    picker: &AuthUser,
    id: Uuid,
    req: &PackRequest,
) -> AppResult<PickList> {
    let h = mine(state, picker, id).await?;
    let lines = picking::lines(&state.db, id).await?;
    if lines.iter().any(|l| l.picked_quantity.is_none()) {
        return Err(coded(
            StatusCode::CONFLICT,
            "NOT_ALL_COUNTED",
            "scan or count every item before packing",
        ));
    }
    let picked = |l: &PickLineRow| i64::from(l.picked_quantity.unwrap_or(0));
    let item_total: i64 = lines.iter().map(|l| l.unit_price_paise * picked(l)).sum();
    let mrp_total: i64 = lines.iter().map(|l| l.unit_mrp_paise * picked(l)).sum();
    if item_total == 0 {
        return Err(coded(
            StatusCode::CONFLICT,
            "NOTHING_PICKED",
            "nothing was found; cancel the order instead",
        ));
    }
    let short: Vec<&PickLineRow> = lines
        .iter()
        .filter(|l| l.picked_quantity < Some(l.quantity))
        .collect();
    let note = if short.is_empty() {
        format!("packed in {} bag(s)", req.bag_count)
    } else {
        let missing: Vec<String> = short
            .iter()
            .map(|l| {
                format!(
                    "{}× {}",
                    l.quantity - l.picked_quantity.unwrap_or(0),
                    l.name
                )
            })
            .collect();
        format!(
            "packed in {} bag(s); missing {}",
            req.bag_count,
            missing.join(", ")
        )
    };

    let mut tx = state.db.begin().await?;
    let prev = orders::set_status(
        &mut tx,
        id,
        &[OrderStatus::Picking],
        OrderStatus::Packed,
        Some(picker.user_id),
        Some(&note),
        None,
    )
    .await?;
    if prev.is_none() {
        tx.rollback().await?;
        return Err(order_service::invalid_transition(state, id, OrderStatus::Packed).await);
    }
    let slot = req
        .staging_slot
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    picking::finish_packing(&mut tx, id, item_total, mrp_total, req.bag_count, slot).await?;
    for line in &short {
        picking::zero_stock(&mut tx, h.store_id, line.product_id).await?;
    }
    tx.commit().await?;

    for line in &short {
        if let Err(error) = inventory::set_stock(&state.redis, h.store_id, line.product_id, 0).await
        {
            tracing::error!(%error, "zeroing Redis stock after a shortage failed");
        }
    }
    // TODO(prod): partial refund for prepaid orders with shortages.
    order_service::publish_status(state, id, prev, OrderStatus::Packed).await;
    dispatch_service::dispatch_best_effort(state, id).await;
    pick_list(state, picker, id).await
}

/// PICKING -> CONFIRMED: hand the order back to the queue (progress is kept).
pub async fn release(state: &AppState, picker: &AuthUser, id: Uuid) -> AppResult<PickList> {
    mine(state, picker, id).await?;
    let mut tx = state.db.begin().await?;
    let prev = orders::set_status(
        &mut tx,
        id,
        &[OrderStatus::Picking],
        OrderStatus::Confirmed,
        Some(picker.user_id),
        Some("returned to the queue"),
        None,
    )
    .await?;
    if prev.is_none() {
        tx.rollback().await?;
        return Err(order_service::invalid_transition(state, id, OrderStatus::Confirmed).await);
    }
    picking::set_picker(&mut tx, id, None).await?;
    tx.commit().await?;
    order_service::publish_status(state, id, prev, OrderStatus::Confirmed).await;
    pick_list(state, picker, id).await
}

/// The picker can't fulfil the order at all.
pub async fn cancel(
    state: &AppState,
    picker: &AuthUser,
    id: Uuid,
    reason: &str,
) -> AppResult<PickList> {
    mine(state, picker, id).await?;
    order_service::cancel(
        state,
        id,
        Some(picker.user_id),
        reason.trim(),
        &[OrderStatus::Picking],
    )
    .await?;
    pick_list(state, picker, id).await
}

#[cfg(test)]
mod tests {
    use super::bin_order;

    #[test]
    fn bins_sort_naturally_with_unlocated_last() {
        let mut bins = vec![
            Some("B-01-1"),
            None,
            Some("A-10-1"),
            Some("A-2-3"),
            Some("a-2-1"),
        ];
        bins.sort_by(|a, b| bin_order(*a, *b));
        assert_eq!(
            bins,
            [
                Some("a-2-1"),
                Some("A-2-3"),
                Some("A-10-1"),
                Some("B-01-1"),
                None
            ]
        );
    }
}
