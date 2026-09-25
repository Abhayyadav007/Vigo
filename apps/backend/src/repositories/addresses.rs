use sqlx::PgExecutor;
use uuid::Uuid;

use crate::models::order::Address;

pub struct AddressInput<'a> {
    pub label: &'a str,
    pub line1: &'a str,
    pub line2: Option<&'a str>,
    pub landmark: Option<&'a str>,
    pub city: &'a str,
    pub pincode: &'a str,
    pub lat: f64,
    pub lng: f64,
}

pub async fn create<'e>(
    db: impl PgExecutor<'e>,
    user_id: Uuid,
    a: &AddressInput<'_>,
) -> Result<Address, sqlx::Error> {
    sqlx::query_as!(
        Address,
        r#"
        INSERT INTO addresses (user_id, label, line1, line2, landmark, city, pincode, location)
        VALUES ($1, $2, $3, $4, $5, $6, $7, ST_SetSRID(ST_MakePoint($9, $8), 4326)::geography)
        RETURNING id, user_id, label, line1, line2, landmark, city, pincode,
                  ST_Y(location::geometry) AS "lat!", ST_X(location::geometry) AS "lng!"
        "#,
        user_id,
        a.label,
        a.line1,
        a.line2,
        a.landmark,
        a.city,
        a.pincode,
        a.lat,
        a.lng,
    )
    .fetch_one(db)
    .await
}

pub async fn update<'e>(
    db: impl PgExecutor<'e>,
    user_id: Uuid,
    id: Uuid,
    a: &AddressInput<'_>,
) -> Result<Option<Address>, sqlx::Error> {
    sqlx::query_as!(
        Address,
        r#"
        UPDATE addresses SET label = $3, line1 = $4, line2 = $5, landmark = $6, city = $7,
            pincode = $8, location = ST_SetSRID(ST_MakePoint($10, $9), 4326)::geography
        WHERE id = $2 AND user_id = $1
        RETURNING id, user_id, label, line1, line2, landmark, city, pincode,
                  ST_Y(location::geometry) AS "lat!", ST_X(location::geometry) AS "lng!"
        "#,
        user_id,
        id,
        a.label,
        a.line1,
        a.line2,
        a.landmark,
        a.city,
        a.pincode,
        a.lat,
        a.lng,
    )
    .fetch_optional(db)
    .await
}

pub async fn delete<'e>(
    db: impl PgExecutor<'e>,
    user_id: Uuid,
    id: Uuid,
) -> Result<bool, sqlx::Error> {
    let res = sqlx::query!(
        "DELETE FROM addresses WHERE id = $2 AND user_id = $1",
        user_id,
        id
    )
    .execute(db)
    .await?;
    Ok(res.rows_affected() == 1)
}

/// The user's addresses with the store currently serving each (if any).
pub async fn list_with_serving_store<'e>(
    db: impl PgExecutor<'e>,
    user_id: Uuid,
) -> Result<Vec<(Address, Option<Uuid>)>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT a.id, a.user_id, a.label, a.line1, a.line2, a.landmark, a.city, a.pincode,
               ST_Y(a.location::geometry) AS "lat!", ST_X(a.location::geometry) AS "lng!",
               (SELECT s.id FROM dark_stores s
                WHERE s.is_active AND ST_Covers(s.service_area, a.location)
                ORDER BY s.location <-> a.location LIMIT 1) AS serving_store_id
        FROM addresses a
        WHERE a.user_id = $1
        ORDER BY a.updated_at DESC
        "#,
        user_id,
    )
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            (
                Address {
                    id: r.id,
                    user_id: r.user_id,
                    label: r.label,
                    line1: r.line1,
                    line2: r.line2,
                    landmark: r.landmark,
                    city: r.city,
                    pincode: r.pincode,
                    lat: r.lat,
                    lng: r.lng,
                },
                r.serving_store_id,
            )
        })
        .collect())
}

pub async fn find<'e>(
    db: impl PgExecutor<'e>,
    user_id: Uuid,
    id: Uuid,
) -> Result<Option<Address>, sqlx::Error> {
    sqlx::query_as!(
        Address,
        r#"
        SELECT id, user_id, label, line1, line2, landmark, city, pincode,
               ST_Y(location::geometry) AS "lat!", ST_X(location::geometry) AS "lng!"
        FROM addresses WHERE id = $2 AND user_id = $1
        "#,
        user_id,
        id,
    )
    .fetch_optional(db)
    .await
}
