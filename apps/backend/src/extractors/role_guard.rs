//! Typed role guards: `async fn handler(Admin(user): Admin, ...)` only runs
//! for callers whose Postgres role is one of the guard's roles; everyone else
//! gets 403. Store-scoped roles (store managers) are further limited to their
//! own store with [`AuthUser::store_scope`] / [`AuthUser::ensure_store`].

use std::marker::PhantomData;

use axum::{extract::FromRequestParts, http::request::Parts};
use uuid::Uuid;

use super::auth_user::AuthUser;
use crate::{auth::Role, error::AppError, state::AppState};

pub trait RoleMarker: Send + Sync {
    const ROLES: &'static [Role];
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
        if !R::ROLES.contains(&user.role) {
            return Err(AppError::forbidden());
        }
        Ok(Self(user, PhantomData))
    }
}

impl AuthUser {
    /// The one store a store-scoped back-office user may act on; `None` means
    /// every store (super admins, support, catalog).
    pub fn store_scope(&self) -> Option<Uuid> {
        if self.role == Role::StoreManager {
            // A manager always has a store (enforced on role change); a nil
            // id matches nothing if that ever breaks.
            Some(self.store_id.unwrap_or_default())
        } else {
            None
        }
    }

    /// 404 (not 403) for another store's data, so ids don't leak.
    pub fn ensure_store(&self, store_id: Uuid) -> Result<(), AppError> {
        match self.store_scope() {
            Some(own) if own != store_id => Err(AppError::NotFound("store")),
            _ => Ok(()),
        }
    }

    /// Narrows an optional `?storeId=` filter to the caller's scope.
    pub fn scoped_store_filter(&self, requested: Option<Uuid>) -> Result<Option<Uuid>, AppError> {
        match (self.store_scope(), requested) {
            (Some(own), Some(id)) if own != id => Err(AppError::NotFound("store")),
            (Some(own), _) => Ok(Some(own)),
            (None, requested) => Ok(requested),
        }
    }
}

macro_rules! role_guard {
    ($($(#[$doc:meta])* $alias:ident => $marker:ident = [$($role:expr),+ $(,)?];)*) => {$(
        pub struct $marker;
        impl RoleMarker for $marker {
            const ROLES: &'static [Role] = &[$($role),+];
        }
        $(#[$doc])*
        pub type $alias = RequireRole<$marker>;
    )*};
}

role_guard! {
    Customer => CustomerRole = [Role::Customer];
    Picker => PickerRole = [Role::Picker];
    Rider => RiderRole = [Role::Rider];
    /// Super Admin only.
    Admin => AdminRole = [Role::Admin];
    /// Categories, products, uploads.
    CatalogStaff => CatalogStaffRole = [Role::Admin, Role::CatalogManager];
    /// Store inventory and open/closed (managers: own store only).
    StoreStaff => StoreStaffRole = [Role::Admin, Role::StoreManager];
    /// Order board, metrics, cancellations (managers: own store only).
    OrderStaff => OrderStaffRole = [Role::Admin, Role::SupportAgent, Role::StoreManager];
    /// Anyone who may sign in to the admin panel.
    BackOffice => BackOfficeRole = [
        Role::Admin,
        Role::StoreManager,
        Role::CatalogManager,
        Role::SupportAgent,
    ];
}
