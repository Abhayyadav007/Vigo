use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Verified contents of a Firebase ID token.
#[derive(Debug, Clone, Deserialize)]
pub struct FirebaseClaims {
    /// Firebase UID.
    pub sub: String,
    pub aud: String,
    pub iss: String,
    pub exp: i64,
    pub iat: i64,
    #[serde(default)]
    pub auth_time: Option<i64>,
    /// E.164, present for phone-auth users.
    #[serde(default)]
    pub phone_number: Option<String>,
    #[serde(default)]
    pub firebase: Option<FirebaseInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FirebaseInfo {
    #[serde(default)]
    pub sign_in_provider: Option<String>,
}

/// Account role. Stored in Postgres (`user_role` enum), never taken from the token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, TS)]
#[sqlx(type_name = "user_role", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export)]
pub enum Role {
    Customer,
    Picker,
    Rider,
    Admin,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Customer => "CUSTOMER",
            Self::Picker => "PICKER",
            Self::Rider => "RIDER",
            Self::Admin => "ADMIN",
        }
    }
}
