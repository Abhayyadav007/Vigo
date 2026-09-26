//! Orders: checkout, the status state machine, cancellation, payment events.
//!
//! Stock flow: checkout reserves units in Redis (all lines atomically, with a
//! TTL). Confirmation (COD immediately, online on the payment webhook)
//! decrements Postgres with conditional updates in one transaction and turns
//! the reservation into sold units. Cancelling before confirmation releases
//! the reservation; after confirmation it restocks Postgres and Redis.

use chrono::Utc;
use uuid::Uuid;

use crate::{
    cache::{
        events,
        inventory::{self, ReserveOutcome},
    },
    dto::order::{
        CheckoutRequest, CheckoutResponse, OrderDetail, OrderEvent, OrderItemDto,
        OrderStatusChanged, OrderSummary, RazorpayCheckout, display_number,
    },
    dto::rider::AssignedRider,
    error::{AppError, AppResult},
    extractors::AuthUser,
    models::order::{AddressSnapshot, Order, OrderStatus, PaymentMethod, PaymentStatus},
    repositories::{
        addresses, carts,
        orders::{self, NewOrder, NewOrderItem},
        riders, stores,
    },
    services::{
        payments::{PaymentInit, PaymentProvider, RazorpayWebhook},
        pricing,
    },
    state::AppState,
};

/// 4 random digits for the doorstep handover.
fn delivery_otp() -> String {
    let bytes = Uuid::new_v4().into_bytes();
    let n = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) % 10_000;
    format!("{n:04}")
}

// ---------- checkout ----------

pub async fn checkout(
    state: &AppState,
    user: &AuthUser,
    idempotency_key: &str,
    req: &CheckoutRequest,
) -> AppResult<CheckoutResponse> {
    // Same key, same order: safe client retries (flaky mobile networks).
    if let Some(existing) =
        orders::find_by_idempotency_key(&state.db, user.user_id, idempotency_key).await?
    {
        return checkout_response(state, existing).await;
    }

    if req.payment_method == PaymentMethod::Online && state.payments.razorpay.is_none() {
        return Err(AppError::Validation(
            "online payment isn't available right now; choose cash on delivery".into(),
        ));
    }

    let address = addresses::find(&state.db, user.user_id, req.address_id)
        .await?
        .ok_or(AppError::NotFound("address"))?;
    match stores::find_serving(&state.db, address.lat, address.lng).await? {
        Some(s) if s.id == req.store_id => {}
        Some(_) => {
            return Err(AppError::Validation(
                "this address is served by a different store; your cart is for another area".into(),
            ));
        }
        None => {
            return Err(AppError::Validation(
                "we don't deliver to this address yet".into(),
            ));
        }
    }

    let lines = carts::lines(&state.db, user.user_id, req.store_id).await?;
    if lines.is_empty() {
        return Err(AppError::Validation("your cart is empty".into()));
    }
    if let Some(bad) = lines.iter().find(|l| !l.sellable || l.stock < l.quantity) {
        return Err(AppError::Conflict(format!(
            "{} is no longer available in that quantity; please review your cart",
            bad.name
        )));
    }

    let item_total: i64 = lines
        .iter()
        .map(|l| l.price_paise * i64::from(l.quantity))
        .sum();
    let mrp_total: i64 = lines
        .iter()
        .map(|l| l.mrp_paise * i64::from(l.quantity))
        .sum();
    let bill = pricing::bill(item_total, mrp_total);
    let order_id = Uuid::new_v4();

    // Reserve stock in Redis: all lines or nothing.
    let reservation: Vec<(Uuid, i32)> = lines.iter().map(|l| (l.product_id, l.quantity)).collect();
    reserve_all(state, req.store_id, order_id, &reservation)
        .await
        .map_err(|e| match e {
            ReserveError::Insufficient(pid) => {
                let name = lines
                    .iter()
                    .find(|l| l.product_id == pid)
                    .map_or("An item", |l| l.name.as_str());
                AppError::Conflict(format!("{name} just sold out; please review your cart"))
            }
            ReserveError::App(e) => e,
        })?;

    let address_snapshot = AddressSnapshot {
        label: address.label,
        line1: address.line1,
        line2: address.line2,
        landmark: address.landmark,
        city: address.city,
        pincode: address.pincode,
        lat: address.lat,
        lng: address.lng,
    };
    let otp = delivery_otp();
    let new_order = NewOrder {
        id: order_id,
        user_id: user.user_id,
        store_id: req.store_id,
        payment_method: req.payment_method,
        item_total_paise: bill.item_total_paise,
        mrp_total_paise: bill.mrp_total_paise,
        delivery_fee_paise: bill.delivery_fee_paise,
        address: &address_snapshot,
        delivery_otp: &otp,
        idempotency_key,
    };
    let items: Vec<NewOrderItem> = lines
        .iter()
        .map(|l| NewOrderItem {
            product_id: l.product_id,
            name: &l.name,
            brand: l.brand.as_deref(),
            unit_label: &l.unit_label,
            image_url: l.image_urls.first().map(String::as_str),
            quantity: l.quantity,
            unit_price_paise: l.price_paise,
            unit_mrp_paise: l.mrp_paise,
        })
        .collect();

    let inserted = async {
        let mut tx = state.db.begin().await?;
        orders::insert(&mut tx, &new_order, &items).await?;
        tx.commit().await
    }
    .await;
    if let Err(e) = inserted {
        release_best_effort(state, req.store_id, order_id).await;
        // A concurrent request with the same key won the race: return its order.
        if let sqlx::Error::Database(db) = &e
            && db.constraint() == Some("orders_user_id_idempotency_key_key")
            && let Some(existing) =
                orders::find_by_idempotency_key(&state.db, user.user_id, idempotency_key).await?
        {
            return checkout_response(state, existing).await;
        }
        return Err(e.into());
    }
    publish_status(state, order_id, None, OrderStatus::Placed).await;

    let order = orders::find(&state.db, order_id)
        .await?
        .ok_or(AppError::NotFound("order"))?;
    let init = match req.payment_method {
        PaymentMethod::Cod => state.payments.cod.initiate(&order).await?,
        PaymentMethod::Online => match &state.payments.razorpay {
            Some(rp) => rp.initiate(&order).await?,
            None => {
                return Err(AppError::Internal(anyhow::anyhow!(
                    "razorpay vanished mid-checkout"
                )));
            }
        },
    };
    match init {
        PaymentInit::CollectOnDelivery => {
            confirm(state, order_id, Some(user.user_id), false).await?;
        }
        PaymentInit::Gateway(g) => {
            orders::set_gateway_order_id(&state.db, order_id, &g.gateway_order_id).await?;
        }
    }
    checkout_response(state, order_id).await
}

enum ReserveError {
    Insufficient(Uuid),
    App(AppError),
}

impl From<AppError> for ReserveError {
    fn from(e: AppError) -> Self {
        Self::App(e)
    }
}

impl From<sqlx::Error> for ReserveError {
    fn from(e: sqlx::Error) -> Self {
        Self::App(e.into())
    }
}

async fn reserve_all(
    state: &AppState,
    store_id: Uuid,
    order_id: Uuid,
    lines: &[(Uuid, i32)],
) -> Result<(), ReserveError> {
    let expires_at = Utc::now().timestamp_millis()
        + i64::try_from(state.config.reservation_ttl.as_millis()).unwrap_or(i64::MAX);
    // Load any products the Redis mirror hasn't seen, then reserve. One retry
    // covers a product whose mirror entry appeared missing mid-flight.
    for _ in 0..2 {
        let ids: Vec<Uuid> = lines.iter().map(|(id, _)| *id).collect();
        let missing = inventory::missing_stock(&state.redis, store_id, &ids).await?;
        if !missing.is_empty() {
            let levels = carts::stock_levels(&state.db, store_id, &missing).await?;
            inventory::load_stock(&state.redis, store_id, &levels).await?;
        }
        match inventory::reserve(&state.redis, store_id, order_id, lines, expires_at).await? {
            ReserveOutcome::Reserved => return Ok(()),
            ReserveOutcome::Insufficient(pid) => return Err(ReserveError::Insufficient(pid)),
            ReserveOutcome::NotLoaded(_) => continue,
        }
    }
    Err(ReserveError::App(AppError::ServiceUnavailable(
        "couldn't reserve stock, please try again".into(),
    )))
}

async fn release_best_effort(state: &AppState, store_id: Uuid, order_id: Uuid) {
    if let Err(error) = inventory::release(&state.redis, store_id, order_id).await {
        // The sweeper releases it when the reservation expires.
        tracing::warn!(%error, %order_id, "releasing reservation failed");
    }
}

async fn checkout_response(state: &AppState, order_id: Uuid) -> AppResult<CheckoutResponse> {
    let order = orders::find(&state.db, order_id)
        .await?
        .ok_or(AppError::NotFound("order"))?;
    let razorpay = match (&order.gateway_order_id, &state.payments.razorpay) {
        (Some(gw), Some(rp))
            if order.payment_status == PaymentStatus::Pending
                && order.status == OrderStatus::Placed =>
        {
            Some(RazorpayCheckout {
                key_id: rp.key_id.clone(),
                gateway_order_id: gw.clone(),
                amount_paise: order.total_paise,
                currency: "INR".into(),
            })
        }
        _ => None,
    };
    Ok(CheckoutResponse {
        order: detail(state, order).await?,
        razorpay,
    })
}

// ---------- state machine ----------

/// Moves an order to `to` if the state machine allows it from the current
/// status (compare-and-set in SQL), logging the event. The only generic way
/// to change status; `confirm`/`cancel` add stock and payment handling.
pub async fn transition(
    state: &AppState,
    order_id: Uuid,
    to: OrderStatus,
    actor: Option<Uuid>,
    note: Option<&str>,
) -> AppResult<OrderStatus> {
    let mut tx = state.db.begin().await?;
    let prev = orders::set_status(
        &mut tx,
        order_id,
        &OrderStatus::predecessors(to),
        to,
        actor,
        note,
        None,
    )
    .await?;
    let Some(prev) = prev else {
        return Err(invalid_transition(state, order_id, to).await);
    };
    tx.commit().await?;
    publish_status(state, order_id, Some(prev), to).await;
    Ok(prev)
}

pub(crate) async fn invalid_transition(
    state: &AppState,
    order_id: Uuid,
    to: OrderStatus,
) -> AppError {
    match orders::find(&state.db, order_id).await {
        Ok(Some(o)) => AppError::Conflict(format!(
            "order is {} and can't become {}",
            o.status.as_str(),
            to.as_str()
        )),
        Ok(None) => AppError::NotFound("order"),
        Err(e) => e.into(),
    }
}

pub(crate) async fn publish_status(
    state: &AppState,
    order_id: Uuid,
    from: Option<OrderStatus>,
    to: OrderStatus,
) {
    let order = match orders::find(&state.db, order_id).await {
        Ok(Some(o)) => o,
        _ => return,
    };
    let event = OrderStatusChanged {
        order_id,
        store_id: order.store_id,
        number: display_number(order.number),
        from,
        status: to,
        at: Utc::now(),
    };
    let mut channels = vec![
        events::order_channel(order_id),
        events::store_channel(order.store_id),
        events::admin_channel(),
    ];
    // The assigned (or just-unassigned) rider hears about their order too.
    if let Ok(Some(rider)) = riders::latest_rider_for_order(&state.db, order_id).await {
        channels.push(events::rider_channel(rider));
    }
    if let Err(error) = events::publish(&state.redis, &channels, &event).await {
        tracing::warn!(%error, %order_id, "publishing order event failed");
    }
}

/// PLACED -> CONFIRMED: commit stock in Postgres (all lines or none) and in
/// Redis. If Postgres is short (should be rare: Redis reserved it), the order
/// is cancelled and 409 returned.
pub async fn confirm(
    state: &AppState,
    order_id: Uuid,
    actor: Option<Uuid>,
    paid: bool,
) -> AppResult<()> {
    let order = orders::find(&state.db, order_id)
        .await?
        .ok_or(AppError::NotFound("order"))?;
    let items = orders::items(&state.db, order_id).await?;

    let mut tx = state.db.begin().await?;
    for item in &items {
        if !orders::decrement_stock(&mut tx, order.store_id, item.product_id, item.quantity).await?
        {
            tx.rollback().await?;
            tracing::error!(%order_id, product_id = %item.product_id, "Postgres stock short at confirmation");
            cancel(
                state,
                order_id,
                None,
                "An item went out of stock at the store",
                &[OrderStatus::Placed],
            )
            .await?;
            return Err(AppError::Conflict(format!(
                "{} went out of stock; the order was cancelled",
                item.name
            )));
        }
    }
    let note = if paid {
        "payment received"
    } else {
        "cash on delivery"
    };
    let prev = orders::set_status(
        &mut tx,
        order_id,
        &[OrderStatus::Placed],
        OrderStatus::Confirmed,
        actor,
        Some(note),
        None,
    )
    .await?;
    if prev.is_none() {
        tx.rollback().await?;
        return Err(invalid_transition(state, order_id, OrderStatus::Confirmed).await);
    }
    if paid {
        orders::set_payment_status(&mut *tx, order_id, PaymentStatus::Paid).await?;
    }
    tx.commit().await?;

    if let Err(error) = inventory::commit(&state.redis, order.store_id, order_id).await {
        // TODO(phase-8): the reconciliation job resyncs the mirror from Postgres.
        tracing::error!(%error, %order_id, "committing reservation in Redis failed");
    }
    if let Err(error) = carts::clear(&state.db, order.user_id, order.store_id).await {
        tracing::warn!(%error, %order_id, "clearing cart failed");
    }
    publish_status(
        state,
        order_id,
        Some(OrderStatus::Placed),
        OrderStatus::Confirmed,
    )
    .await;
    Ok(())
}

/// Cancels from any of `allowed_from`. Before confirmation the reservation is
/// released; after it, stock goes back to Postgres and Redis.
pub async fn cancel(
    state: &AppState,
    order_id: Uuid,
    actor: Option<Uuid>,
    reason: &str,
    allowed_from: &[OrderStatus],
) -> AppResult<()> {
    let order = orders::find(&state.db, order_id)
        .await?
        .ok_or(AppError::NotFound("order"))?;
    let items = orders::items(&state.db, order_id).await?;
    let from: Vec<OrderStatus> = OrderStatus::predecessors(OrderStatus::Cancelled)
        .into_iter()
        .filter(|s| allowed_from.contains(s))
        .collect();

    let mut tx = state.db.begin().await?;
    let Some(prev) = orders::set_status(
        &mut tx,
        order_id,
        &from,
        OrderStatus::Cancelled,
        actor,
        Some(reason),
        Some(reason),
    )
    .await?
    else {
        tx.rollback().await?;
        return Err(invalid_transition(state, order_id, OrderStatus::Cancelled).await);
    };
    let stock_was_taken = prev != OrderStatus::Placed;
    let restock = restock_quantities(&items);
    let had_rider = riders::end_active_for_order(&mut tx, order_id)
        .await?
        .is_some();
    if stock_was_taken {
        for &(product_id, qty) in &restock {
            orders::increment_stock(&mut tx, order.store_id, product_id, qty).await?;
        }
    }
    if order.payment_status == PaymentStatus::Paid {
        // TODO(phase-8): issue the refund through the payment provider.
        orders::set_payment_status(&mut *tx, order_id, PaymentStatus::Refunded).await?;
    } else if order.payment_method == PaymentMethod::Online {
        orders::set_payment_status(&mut *tx, order_id, PaymentStatus::Failed).await?;
    }
    tx.commit().await?;

    if stock_was_taken {
        for &(product_id, qty) in &restock {
            if let Err(error) =
                inventory::add_stock(&state.redis, order.store_id, product_id, qty).await
            {
                tracing::error!(%error, %order_id, "restocking Redis mirror failed");
            }
        }
    } else {
        release_best_effort(state, order.store_id, order_id).await;
    }
    if had_rider || prev == OrderStatus::Packed {
        // Frees the dispatch claim; open offers die with the PACKED status.
        if let Err(error) = crate::cache::geo::clear_winner(&state.redis, order_id).await {
            tracing::warn!(%error, %order_id, "clearing dispatch winner failed");
        }
    }
    publish_status(state, order_id, Some(prev), OrderStatus::Cancelled).await;
    Ok(())
}

/// Units that go back on the shelf when a confirmed order is cancelled. Once
/// a picker has counted a line, only what they found is restocked; units they
/// marked missing were never there.
fn restock_quantities(items: &[crate::models::order::OrderItem]) -> Vec<(Uuid, i32)> {
    items
        .iter()
        .map(|i| (i.product_id, i.picked_quantity.unwrap_or(i.quantity)))
        .filter(|(_, qty)| *qty > 0)
        .collect()
}

/// Customer-initiated cancel: own order, and only before picking starts.
pub async fn cancel_by_customer(
    state: &AppState,
    user: &AuthUser,
    order_id: Uuid,
    reason: Option<&str>,
) -> AppResult<OrderDetail> {
    let order = orders::find(&state.db, order_id)
        .await?
        .filter(|o| o.user_id == user.user_id);
    let order = order.ok_or(AppError::NotFound("order"))?;
    if !order.status.customer_cancellable() {
        return Err(AppError::Conflict(
            "the store has started packing this order, so it can't be cancelled".into(),
        ));
    }
    let reason = reason
        .map(str::trim)
        .filter(|r| !r.is_empty())
        .unwrap_or("Cancelled by customer");
    cancel(
        state,
        order_id,
        Some(user.user_id),
        reason,
        &[OrderStatus::Placed, OrderStatus::Confirmed],
    )
    .await?;
    let order = orders::find(&state.db, order_id)
        .await?
        .ok_or(AppError::NotFound("order"))?;
    detail(state, order).await
}

// ---------- reservation expiry ----------

/// Releases expired reservations. Unpaid online orders behind them are
/// cancelled; orphans (checkout crashed before inserting the order) are just
/// released. Safe to run on every instance: each step is idempotent.
pub async fn sweep_expired_reservations(state: &AppState) -> AppResult<usize> {
    let now = Utc::now().timestamp_millis();
    let mut swept = 0;
    for store_id in orders::active_store_ids(&state.db).await? {
        for reservation_id in inventory::expired_reservations(&state.redis, store_id, now).await? {
            match orders::find(&state.db, reservation_id).await? {
                Some(order) if order.status == OrderStatus::Placed => {
                    match cancel(
                        state,
                        reservation_id,
                        None,
                        "Payment wasn't completed in time",
                        &[OrderStatus::Placed],
                    )
                    .await
                    {
                        Ok(()) | Err(AppError::Conflict(_)) => {}
                        Err(e) => return Err(e),
                    }
                }
                _ => inventory::release(&state.redis, store_id, reservation_id).await?,
            }
            swept += 1;
        }
    }
    Ok(swept)
}

// ---------- payment webhooks ----------

pub async fn handle_razorpay_webhook(
    state: &AppState,
    body: &[u8],
    signature: Option<&str>,
    event_id: Option<&str>,
) -> AppResult<()> {
    let razorpay = state
        .payments
        .razorpay
        .as_ref()
        .ok_or(AppError::NotFound("payment provider"))?;
    let signature = signature.ok_or(AppError::Unauthorized)?;
    if !razorpay.verify_webhook(body, signature) {
        tracing::warn!("rejected Razorpay webhook with a bad signature");
        return Err(AppError::Unauthorized);
    }
    let event_id =
        event_id.ok_or_else(|| AppError::BadRequest("missing X-Razorpay-Event-Id".into()))?;
    let raw: serde_json::Value = serde_json::from_slice(body)
        .map_err(|e| AppError::BadRequest(format!("invalid JSON: {e}")))?;
    let hook: RazorpayWebhook = serde_json::from_value(raw.clone())
        .map_err(|e| AppError::BadRequest(format!("unexpected payload: {e}")))?;

    let payment = hook.payload.payment.map(|p| p.entity);
    let order_id = match payment.as_ref().and_then(|p| p.order_id.as_deref()) {
        Some(gw) => orders::find_by_gateway_order_id(&state.db, gw).await?,
        None => None,
    };
    // Deliveries are at-least-once: act on each event id once.
    if !orders::record_payment_event(&state.db, "razorpay", event_id, &hook.event, order_id, &raw)
        .await?
    {
        tracing::info!(event_id, "duplicate Razorpay webhook ignored");
        return Ok(());
    }
    let (Some(order_id), Some(payment)) = (order_id, payment) else {
        tracing::info!(event = hook.event, "Razorpay webhook for no known order");
        return Ok(());
    };
    let order = orders::find(&state.db, order_id)
        .await?
        .ok_or(AppError::NotFound("order"))?;

    match hook.event.as_str() {
        "payment.captured" | "order.paid" => {
            if payment.amount != order.total_paise || payment.currency != "INR" {
                tracing::error!(%order_id, amount = payment.amount, "payment amount mismatch; not confirming");
                return Err(AppError::Validation(
                    "payment amount doesn't match the order".into(),
                ));
            }
            match order.status {
                OrderStatus::Placed => confirm(state, order_id, None, true).await?,
                OrderStatus::Cancelled => {
                    // Paid after the reservation expired.
                    // TODO(phase-8): refund automatically through the provider.
                    orders::set_payment_status(&state.db, order_id, PaymentStatus::Refunded)
                        .await?;
                    tracing::error!(%order_id, payment_id = payment.id, "payment for a cancelled order; refund needed");
                }
                _ => orders::set_payment_status(&state.db, order_id, PaymentStatus::Paid).await?,
            }
        }
        "payment.failed" if order.status == OrderStatus::Placed => {
            cancel(
                state,
                order_id,
                None,
                "Payment failed",
                &[OrderStatus::Placed],
            )
            .await?;
        }
        other => tracing::debug!(event = other, "Razorpay webhook ignored"),
    }
    Ok(())
}

// ---------- read models ----------

pub fn summary(order: &Order, item_count: i32) -> OrderSummary {
    OrderSummary {
        id: order.id,
        number: display_number(order.number),
        status: order.status,
        payment_method: order.payment_method,
        payment_status: order.payment_status,
        total_paise: order.total_paise,
        item_count,
        created_at: order.created_at,
    }
}

pub async fn detail(state: &AppState, order: Order) -> AppResult<OrderDetail> {
    let rider = assigned_rider(state, &order).await?;
    let items = orders::items(&state.db, order.id).await?;
    let events = orders::events(&state.db, order.id).await?;
    let item_count = items.iter().map(|i| i.quantity).sum();
    let mut bill = pricing::bill(order.item_total_paise, order.mrp_total_paise);
    // The fee charged at checkout, even if the rules change later.
    bill.delivery_fee_paise = order.delivery_fee_paise;
    bill.total_paise = order.total_paise;
    Ok(OrderDetail {
        summary: summary(&order, item_count),
        store_id: order.store_id,
        items: items
            .into_iter()
            .map(|i| OrderItemDto {
                product_id: i.product_id,
                name: i.name,
                brand: i.brand,
                unit_label: i.unit_label,
                image_url: i.image_url,
                quantity: i.quantity,
                unit_price_paise: i.unit_price_paise,
                unit_mrp_paise: i.unit_mrp_paise,
                picked_quantity: i.picked_quantity,
            })
            .collect(),
        bill,
        address: order.address.0,
        delivery_otp: (!order.status.is_terminal() && order.status != OrderStatus::Placed)
            .then_some(order.delivery_otp),
        rider,
        cancel_reason: order.cancel_reason,
        can_cancel: order.status.customer_cancellable(),
        events: events
            .into_iter()
            .map(|e| OrderEvent {
                status: e.to_status,
                note: e.note,
                at: e.created_at,
            })
            .collect(),
    })
}

async fn assigned_rider(state: &AppState, order: &Order) -> AppResult<Option<AssignedRider>> {
    if !matches!(
        order.status,
        OrderStatus::RiderAssigned | OrderStatus::PickedUp | OrderStatus::OutForDelivery
    ) {
        return Ok(None);
    }
    let Some(d) = riders::active_for_order(&state.db, order.id).await? else {
        return Ok(None);
    };
    let location = crate::cache::geo::last_fix(&state.redis, d.rider_id)
        .await?
        .map(|f| crate::dto::geo::LatLng {
            lat: f.lat,
            lng: f.lng,
        });
    Ok(riders::contact(&state.db, d.rider_id)
        .await?
        .map(|c| AssignedRider {
            location,
            name: c.name,
            phone: c.phone,
            vehicle_type: c.vehicle_type,
            vehicle_number: c.vehicle_number,
        }))
}

pub async fn get_for_customer(
    state: &AppState,
    user: &AuthUser,
    order_id: Uuid,
) -> AppResult<OrderDetail> {
    let order = orders::find(&state.db, order_id)
        .await?
        .filter(|o| o.user_id == user.user_id)
        .ok_or(AppError::NotFound("order"))?;
    detail(state, order).await
}

#[cfg(test)]
mod tests {
    use super::delivery_otp;

    #[test]
    fn otp_is_four_digits() {
        for _ in 0..200 {
            let otp = delivery_otp();
            assert_eq!(otp.len(), 4);
            assert!(otp.bytes().all(|b| b.is_ascii_digit()));
        }
    }
}
