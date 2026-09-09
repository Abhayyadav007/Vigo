use domain::NearbyShop;
use sqlx::PgPool;

/// Shops that can serve `(lat, lng)` right now, soonest first.
///
/// Ordered by ETA rather than distance: a nearer shop that preps slowly ranks
/// below a slightly further one that is fast. See db/queries/nearest_open_shops.sql
///
/// Note the casts. sqlx has no mapping for PostGIS `geography`, so the raw
/// `location` column is never selected - only projections of it, which are
/// float8. `::float8` on NUMERIC avoids pulling in BigDecimal, and `!` marks
/// aggregate columns that sqlx conservatively infers as nullable.
pub async fn nearby(pool: &PgPool, lat: f64, lng: f64) -> Result<Vec<NearbyShop>, sqlx::Error> {
    sqlx::query_as!(
        NearbyShop,
        r#"
        SELECT s.id                                            AS "id!",
               s.name                                          AS "name!",
               ST_Distance(s.location, cust.geo)::float8        AS "metres!",
               s.avg_prep_seconds                               AS "prep_seconds!",
               s.acceptance_rate::float8                        AS "acceptance_rate!",
               ((s.avg_prep_seconds + ST_Distance(s.location, cust.geo) / 5.0)
                    / 60.0)::float8                             AS "eta_minutes!",
               count(si.id) FILTER (
                   WHERE si.is_available AND si.stock_qty > 0
               )                                                AS "item_count!"
        FROM shops s
        CROSS JOIN (
            SELECT ST_MakePoint($1::float8, $2::float8)::geography AS geo
        ) cust
        LEFT JOIN shop_items si ON si.shop_id = s.id
        WHERE s.status = 'active'
          AND s.is_open
          AND s.is_accepting_orders
          AND ST_DWithin(s.location, cust.geo, s.delivery_radius_m)
        GROUP BY s.id, cust.geo
        ORDER BY (s.avg_prep_seconds + ST_Distance(s.location, cust.geo) / 5.0),
                 ST_Distance(s.location, cust.geo)
        "#,
        lng,
        lat,
    )
    .fetch_all(pool)
    .await
}
