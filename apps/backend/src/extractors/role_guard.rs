//! Typed role guards: `async fn handler(Admin(user): Admin, ...)` only runs
//! for callers whose Postgres role is `ADMIN`; everyone else gets 403.

use std::marker::PhantomData;

use axum::{extract::FromRequestParts, http::request::Parts};

use super::auth_user::AuthUser;
use crate::{auth::Role, error::AppError, state::AppState};

pub trait RoleMarker: Send + Sync {
    const ROLE: Role;
}

pub struct RequireRole<R: RoleMarker>(pub AuthUser, PhantomData<R>);

impl<R: RoleMarker> RequireRole<R> {
    pub fn into_inner(self) -> AuthUser {
        self.0
    }
}

impl<R: RoleMarker> std::ops::Deref for RequireRole<R> {
    type Target = AuthUser;
    fn deref(&self) -> &AuthUser {
        &self.0
    }
}

impl<R: RoleMarker> FromRequestParts<AppState> for RequireRole<R> {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        if user.role != R::ROLE {
            return Err(AppError::forbidden());
        }
        Ok(Self(user, PhantomData))
    }
}

macro_rules! role_guard {
    ($($alias:ident => $marker:ident = $role:expr;)*) => {$(
        pub struct $marker;
        impl RoleMarker for $marker {
            const ROLE: Role = $role;
        }
        pub type $alias = RequireRole<$marker>;
    )*};
}

role_guard! {
    Customer => CustomerRole = Role::Customer;
    Picker => PickerRole = Role::Picker;
    Rider => RiderRole = Role::Rider;
    Admin => AdminRole = Role::Admin;
}
