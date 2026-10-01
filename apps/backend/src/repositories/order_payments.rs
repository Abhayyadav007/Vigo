//! Captured gateway payments (`order_payments`), one per order.

use sqlx::PgExecutor;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CapturedPayment {
    pub payment_id: String,
    pub amount_paise: i64,
    pub refund_id: Option<String>,
}

/// Records a capture; false if the order already has one (redelivery or a
/// second payment on the same gateway order).
pub async fn record<'e>(
    db: impl PgExecutor<'e>,
    order_id: Uuid,
    provider: &str,
    payment_id: &str,
    amount_paise: i64,
) -> Result<bool, sqlx::Error> {
    let inserted = sqlx::query!(
        r#"
        INSERT INTO order_payments (order_id, provider, payment_id, amount_paise)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT DO NOTHING
        "#,
        order_id,
        provider,
        payment_id,
        amount_paise,
    )
    .execute(db)
    .await?
    .rows_affected();
    Ok(inserted == 1)
}

pub async fn find<'e>(
    db: impl PgExecutor<'e>,
    order_id: Uuid,
) -> Result<Option<CapturedPayment>, sqlx::Error> {
    sqlx::query_as!(
        CapturedPayment,
        "SELECT payment_id, amount_paise, refund_id FROM order_payments WHERE order_id = $1",
        order_id,
    )
    .fetch_optional(db)
    .await
}

pub async fn set_refunded<'e>(
    db: impl PgExecutor<'e>,
    order_id: Uuid,
    refund_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE order_payments SET refund_id = $2, refunded_at = now() WHERE order_id = $1",
        order_id,
        refund_id,
    )
    .execute(db)
    .await?;
    Ok(())
}
