use sqlx::{PgExecutor, types::Json};
use uuid::Uuid;

use crate::{
    dto::geo::GeoJsonPolygon,
    models::store::{DarkStore, ServingStore},
};

pub struct StoreInput<'a> {
    pub code: &'a str,
    pub name: &'a str,
    pub address: &'a str,
    pub lat: f64,
    pub lng: f64,
    pub service_area: &'a GeoJsonPolygon,
    pub is_active: bool,
}

pub struct PolygonCheck {
    pub valid: bool,
    pub reason: String,
    pub area_sq_m: f64,
}

/// Geometric validity (self-intersections etc.) and area, straight from PostGIS.
pub async fn check_polygon<'e>(
    db: impl PgExecutor<'e>,
    polygon: &GeoJsonPolygon,
) -> Result<PolygonCheck, sqlx::Error> {
    sqlx::query_as!(
        PolygonCheck,
        r#"
        SELECT ST_IsValid(g) AS "valid!",
               ST_IsValidReason(g) AS "reason!",
               ST_Area(g::geography) AS "area_sq_m!"
        FROM ST_GeomFromGeoJSON($1::jsonb::text) AS g
        "#,
        Json(polygon) as _,
    )
    .fetch_one(db)
    .await
}

pub async fn create<'e>(
    db: impl PgExecutor<'e>,
    s: &StoreInput<'_>,
) -> Result<DarkStore, sqlx::Error> {
    sqlx::query_as!(
        DarkStore,
        r#"
        INSERT INTO dark_stores (code, name, address, location, service_area, is_active)
        VALUES ($1, $2, $3,
                ST_SetSRID(ST_MakePoint($5, $4), 4326)::geography,
                ST_GeomFromGeoJSON($6::jsonb::text)::geography,
                $7)
        RETURNING id, code, name, address,
                  ST_Y(location::geometry) AS "lat!", ST_X(location::geometry) AS "lng!",
                  ST_AsGeoJSON(service_area, 7)::jsonb AS "service_area!: Json<GeoJsonPolygon>",
                  ST_Area(service_area) AS "area_sq_m!",
                  is_active, created_at, updated_at
        "#,
        s.code,
        s.name,
        s.address,
        s.lat,
        s.lng,
        Json(s.service_area) as _,
        s.is_active,
    )
    .fetch_one(db)
    .await
}

pub async fn update<'e>(
    db: impl PgExecutor<'e>,
    id: Uuid,
    s: &StoreInput<'_>,
) -> Result<Option<DarkStore>, sqlx::Error> {
    sqlx::query_as!(
        DarkStore,
        r#"
        UPDATE dark_stores SET
            code = $2, name = $3, address = $4,
            location = ST_SetSRID(ST_MakePoint($6, $5), 4326)::geography,
            service_area = ST_GeomFromGeoJSON($7::jsonb::text)::geography,
            is_active = $8
        WHERE id = $1
        RETURNING id, code, name, address,
                  ST_Y(location::geometry) AS "lat!", ST_X(location::geometry) AS "lng!",
                  ST_AsGeoJSON(service_area, 7)::jsonb AS "service_area!: Json<GeoJsonPolygon>",
                  ST_Area(service_area) AS "area_sq_m!",
                  is_active, created_at, updated_at
        "#,
        id,
        s.code,
        s.name,
        s.address,
        s.lat,
        s.lng,
        Json(s.service_area) as _,
        s.is_active,
    )
    .fetch_optional(db)
    .await
}

pub async fn find<'e>(db: impl PgExecutor<'e>, id: Uuid) -> Result<Option<DarkStore>, sqlx::Error> {
    sqlx::query_as!(
        DarkStore,
        r#"
        SELECT id, code, name, address,
               ST_Y(location::geometry) AS "lat!", ST_X(location::geometry) AS "lng!",
               ST_AsGeoJSON(service_area, 7)::jsonb AS "service_area!: Json<GeoJsonPolygon>",
               ST_Area(service_area) AS "area_sq_m!",
               is_active, created_at, updated_at
        FROM dark_stores WHERE id = $1
        "#,
        id,
    )
    .fetch_optional(db)
    .await
}

pub async fn list<'e>(
    db: impl PgExecutor<'e>,
    q: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<(Vec<DarkStore>, i64), sqlx::Error> {
    let pattern = q.map(|q| format!("%{q}%"));
    let rows = sqlx::query!(
        r#"
        SELECT id, code, name, address,
               ST_Y(location::geometry) AS "lat!", ST_X(location::geometry) AS "lng!",
               ST_AsGeoJSON(service_area, 7)::jsonb AS "service_area!: Json<GeoJsonPolygon>",
               ST_Area(service_area) AS "area_sq_m!",
               is_active, created_at, updated_at,
               count(*) OVER () AS "total!"
        FROM dark_stores
        WHERE $1::text IS NULL OR name ILIKE $1 OR code ILIKE $1 OR address ILIKE $1
        ORDER BY name, id
        LIMIT $2 OFFSET $3
        "#,
        pattern,
        limit,
        offset,
    )
    .fetch_all(db)
    .await?;
    let total = rows.first().map_or(0, |r| r.total);
    let stores = rows
        .into_iter()
        .map(|r| DarkStore {
            id: r.id,
            code: r.code,
            name: r.name,
            address: r.address,
            lat: r.lat,
            lng: r.lng,
            service_area: r.service_area,
            area_sq_m: r.area_sq_m,
            is_active: r.is_active,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
        .collect();
    Ok((stores, total))
}

/// The active store whose service area covers the point; the nearest one if
/// several overlap.
pub async fn find_serving<'e>(
    db: impl PgExecutor<'e>,
    lat: f64,
    lng: f64,
) -> Result<Option<ServingStore>, sqlx::Error> {
    sqlx::query_as!(
        ServingStore,
        r#"
        WITH p AS (SELECT ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography AS pt)
        SELECT s.id, s.code, s.name, ST_Distance(s.location, p.pt) AS "distance_m!"
        FROM dark_stores s, p
        WHERE s.is_active AND ST_Covers(s.service_area, p.pt)
        ORDER BY s.location <-> p.pt
        LIMIT 1
        "#,
        lat,
        lng,
    )
    .fetch_optional(db)
    .await
}

/// True when the store exists and is active.
pub async fn is_active<'e>(db: impl PgExecutor<'e>, id: Uuid) -> Result<bool, sqlx::Error> {
    Ok(
        sqlx::query_scalar!("SELECT is_active FROM dark_stores WHERE id = $1", id)
            .fetch_optional(db)
            .await?
            .unwrap_or(false),
    )
}
