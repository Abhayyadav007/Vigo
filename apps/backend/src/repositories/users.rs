use sqlx::PgExecutor;
use uuid::Uuid;

use crate::{auth::Role, models::user::User};

/// Creates the user on first login, or refreshes the phone number (Firebase
/// lets a user change it). Role and other fields are never touched here.
///
/// Fails with a unique violation on `users_phone_key` when the phone already
/// belongs to a different Firebase UID; see [`relink_firebase_uid`].
pub async fn upsert_from_firebase<'e>(
    db: impl PgExecutor<'e>,
    firebase_uid: &str,
    phone: &str,
) -> Result<User, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"
        INSERT INTO users (firebase_uid, phone)
        VALUES ($1, $2)
        ON CONFLICT (firebase_uid) DO UPDATE
            SET phone = EXCLUDED.phone
        RETURNING id, firebase_uid, phone, name, role AS "role: Role", store_id,
                  is_active, created_at, updated_at
        "#,
        firebase_uid,
        phone,
    )
    .fetch_one(db)
    .await
}

/// Points an existing account at a new Firebase UID. Used when the same phone
/// signs in under a fresh UID (e.g. the Firebase user was deleted and
/// recreated); Firebase has just proven ownership of the number via OTP.
pub async fn relink_firebase_uid<'e>(
    db: impl PgExecutor<'e>,
    phone: &str,
    firebase_uid: &str,
) -> Result<User, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"
        UPDATE users SET firebase_uid = $2
        WHERE phone = $1
        RETURNING id, firebase_uid, phone, name, role AS "role: Role", store_id,
                  is_active, created_at, updated_at
        "#,
        phone,
        firebase_uid,
    )
    .fetch_one(db)
    .await
}

pub async fn find_by_firebase_uid<'e>(
    db: impl PgExecutor<'e>,
    firebase_uid: &str,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"
        SELECT id, firebase_uid, phone, name, role AS "role: Role", store_id,
               is_active, created_at, updated_at
        FROM users WHERE firebase_uid = $1
        "#,
        firebase_uid,
    )
    .fetch_optional(db)
    .await
}

pub async fn find_by_phone<'e>(
    db: impl PgExecutor<'e>,
    phone: &str,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"
        SELECT id, firebase_uid, phone, name, role AS "role: Role", store_id,
               is_active, created_at, updated_at
        FROM users WHERE phone = $1
        "#,
        phone,
    )
    .fetch_optional(db)
    .await
}

pub async fn find_by_id<'e>(
    db: impl PgExecutor<'e>,
    id: Uuid,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"
        SELECT id, firebase_uid, phone, name, role AS "role: Role", store_id,
               is_active, created_at, updated_at
        FROM users WHERE id = $1
        "#,
        id,
    )
    .fetch_optional(db)
    .await
}

pub struct UserFilter<'a> {
    /// Substring match on the phone number.
    pub phone: Option<&'a str>,
    pub role: Option<Role>,
    pub limit: i64,
    pub offset: i64,
}

pub async fn list<'e>(
    db: impl PgExecutor<'e>,
    filter: &UserFilter<'_>,
) -> Result<(Vec<User>, i64), sqlx::Error> {
    let phone_pattern = filter.phone.map(|p| format!("%{p}%"));
    let rows = sqlx::query!(
        r#"
        SELECT id, firebase_uid, phone, name, role AS "role: Role", store_id,
               is_active, created_at, updated_at,
               count(*) OVER () AS "total!"
        FROM users
        WHERE ($1::text IS NULL OR phone LIKE $1)
          AND ($2::user_role IS NULL OR role = $2)
        ORDER BY created_at DESC, id
        LIMIT $3 OFFSET $4
        "#,
        phone_pattern,
        filter.role as Option<Role>,
        filter.limit,
        filter.offset,
    )
    .fetch_all(db)
    .await?;

    let total = rows.first().map_or(0, |r| r.total);
    let users = rows
        .into_iter()
        .map(|r| User {
            id: r.id,
            firebase_uid: r.firebase_uid,
            phone: r.phone,
            name: r.name,
            role: r.role,
            store_id: r.store_id,
            is_active: r.is_active,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
        .collect();
    Ok((users, total))
}

pub async fn update_role<'e>(
    db: impl PgExecutor<'e>,
    id: Uuid,
    role: Role,
    store_id: Option<Uuid>,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"
        UPDATE users SET role = $2, store_id = $3
        WHERE id = $1
        RETURNING id, firebase_uid, phone, name, role AS "role: Role", store_id,
                  is_active, created_at, updated_at
        "#,
        id,
        role as Role,
        store_id,
    )
    .fetch_optional(db)
    .await
}

pub async fn set_role_by_phone<'e>(
    db: impl PgExecutor<'e>,
    phone: &str,
    role: Role,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"
        UPDATE users SET role = $2
        WHERE phone = $1
        RETURNING id, firebase_uid, phone, name, role AS "role: Role", store_id,
                  is_active, created_at, updated_at
        "#,
        phone,
        role as Role,
    )
    .fetch_optional(db)
    .await
}
