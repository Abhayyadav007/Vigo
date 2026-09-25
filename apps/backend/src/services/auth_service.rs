use crate::{
    auth::FirebaseClaims,
    cache::session::{self, SessionUser},
    error::{AppError, AppResult},
    models::user::User,
    repositories::users,
    state::AppState,
};

const ACCOUNT_DISABLED: AppError = AppError::Forbidden {
    code: "ACCOUNT_DISABLED",
    message: "this account has been disabled",
};

/// India-only: `+91` followed by a 10-digit mobile number starting 6-9.
pub fn is_indian_mobile(phone: &str) -> bool {
    phone.strip_prefix("+91").is_some_and(|rest| {
        rest.len() == 10
            && rest.bytes().all(|b| b.is_ascii_digit())
            && matches!(rest.as_bytes()[0], b'6'..=b'9')
    })
}

/// `POST /v1/auth/sync`: make sure a Postgres user exists for this Firebase
/// identity. New users are always `CUSTOMER`; staff roles come from an admin.
pub async fn sync(state: &AppState, claims: &FirebaseClaims) -> AppResult<User> {
    let phone = claims.phone_number.as_deref().ok_or_else(|| {
        AppError::Validation("token has no phone number; sign in with phone OTP".into())
    })?;
    if !is_indian_mobile(phone) {
        return Err(AppError::Validation(
            "only Indian (+91) mobile numbers are supported".into(),
        ));
    }

    let user = match users::upsert_from_firebase(&state.db, &claims.sub, phone).await {
        Ok(user) => user,
        Err(e) if is_unique_violation(&e, "users_phone_key") => {
            relink(state, phone, &claims.sub).await?
        }
        Err(e) => return Err(e.into()),
    };

    if !user.is_active {
        return Err(ACCOUNT_DISABLED);
    }
    cache_best_effort(state, &claims.sub, &SessionUser::from(&user)).await;
    Ok(user)
}

async fn relink(state: &AppState, phone: &str, firebase_uid: &str) -> AppResult<User> {
    let previous = users::find_by_phone(&state.db, phone).await?;
    let user = users::relink_firebase_uid(&state.db, phone, firebase_uid).await?;
    if let Some(prev) = previous {
        tracing::info!(user_id = %user.id, "phone re-linked to a new Firebase UID");
        invalidate_best_effort(state, &prev.firebase_uid).await;
    }
    Ok(user)
}

/// Resolves the Postgres identity behind a verified Firebase UID, via the
/// Redis session cache. Redis trouble degrades to a Postgres lookup.
pub async fn resolve_session(state: &AppState, firebase_uid: &str) -> AppResult<SessionUser> {
    let cached = match session::get(&state.redis, firebase_uid).await {
        Ok(hit) => hit,
        Err(error) => {
            tracing::warn!(%error, "session cache read failed; falling back to Postgres");
            None
        }
    };

    let session = match cached {
        Some(s) => s,
        None => {
            let user = users::find_by_firebase_uid(&state.db, firebase_uid)
                .await?
                .ok_or(AppError::Forbidden {
                    code: "USER_NOT_REGISTERED",
                    message: "call POST /v1/auth/sync after signing in",
                })?;
            let s = SessionUser::from(&user);
            cache_best_effort(state, firebase_uid, &s).await;
            s
        }
    };

    if !session.is_active {
        return Err(ACCOUNT_DISABLED);
    }
    Ok(session)
}

pub async fn invalidate_best_effort(state: &AppState, firebase_uid: &str) {
    if let Err(error) = session::invalidate(&state.redis, firebase_uid).await {
        // The entry still expires after the TTL.
        tracing::warn!(%error, "session cache invalidation failed");
    }
}

async fn cache_best_effort(state: &AppState, firebase_uid: &str, s: &SessionUser) {
    if let Err(error) = session::set(
        &state.redis,
        firebase_uid,
        s,
        state.config.session_cache_ttl,
    )
    .await
    {
        tracing::warn!(%error, "session cache write failed");
    }
}

fn is_unique_violation(e: &sqlx::Error, constraint: &str) -> bool {
    matches!(e, sqlx::Error::Database(db) if db.is_unique_violation() && db.constraint() == Some(constraint))
}

#[cfg(test)]
mod tests {
    use super::is_indian_mobile;

    #[test]
    fn indian_mobile_validation() {
        assert!(is_indian_mobile("+919876543210"));
        assert!(is_indian_mobile("+916000000000"));
        assert!(!is_indian_mobile("+915876543210"), "must start 6-9");
        assert!(!is_indian_mobile("+14155550100"), "non-Indian");
        assert!(!is_indian_mobile("+91987654321"), "too short");
        assert!(!is_indian_mobile("+9198765432100"), "too long");
        assert!(!is_indian_mobile("9876543210"), "missing country code");
    }
}
