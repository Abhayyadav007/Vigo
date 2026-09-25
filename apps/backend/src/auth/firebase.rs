//! Firebase ID token verification (RS256 against Google's published JWKS).
//!
//! See <https://firebase.google.com/docs/auth/admin/verify-id-tokens#verify_id_tokens_using_a_third-party_jwt_library>.

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header, jwk::JwkSet};
use tokio::sync::{Mutex, RwLock};

use super::claims::FirebaseClaims;

pub const GOOGLE_JWKS_URL: &str =
    "https://www.googleapis.com/service_accounts/v1/jwk/securetoken@system.gserviceaccount.com";

/// Clock skew tolerated on `exp`, `iat` and `auth_time`.
const LEEWAY_SECS: u64 = 60;
/// Used when Google omits `Cache-Control: max-age`.
const DEFAULT_MAX_AGE: Duration = Duration::from_secs(3600);
/// A token with an unknown `kid` triggers a refetch at most this often, so
/// garbage tokens can't make us hammer Google.
const MIN_REFETCH_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Debug, thiserror::Error)]
pub enum TokenError {
    #[error("malformed token: {0}")]
    Malformed(String),
    #[error("unsupported token algorithm {0:?}")]
    Algorithm(String),
    #[error("unknown signing key id")]
    UnknownKey,
    #[error("invalid token: {0}")]
    Invalid(#[from] jsonwebtoken::errors::Error),
    #[error("invalid claim: {0}")]
    Claim(&'static str),
    #[error("fetching Google JWKS failed: {0}")]
    Jwks(String),
}

impl TokenError {
    /// True when the failure is on our side (JWKS unavailable), not the caller's.
    pub fn is_upstream(&self) -> bool {
        matches!(self, Self::Jwks(_))
    }
}

pub struct FirebaseVerifier {
    project_id: String,
    issuer: String,
    keys: KeySource,
}

enum KeySource {
    Google(JwksCache),
    /// Fixed keys by `kid`, for tests.
    Static(HashMap<String, DecodingKey>),
    /// Firebase Auth emulator: tokens are unsigned (`alg: none`). Dev only.
    Emulator,
}

impl FirebaseVerifier {
    pub fn google(project_id: impl Into<String>, http: reqwest::Client) -> Self {
        Self::new(
            project_id,
            KeySource::Google(JwksCache::new(http, GOOGLE_JWKS_URL)),
        )
    }

    pub fn emulator(project_id: impl Into<String>) -> Self {
        Self::new(project_id, KeySource::Emulator)
    }

    pub fn with_static_keys(
        project_id: impl Into<String>,
        keys: HashMap<String, DecodingKey>,
    ) -> Self {
        Self::new(project_id, KeySource::Static(keys))
    }

    fn new(project_id: impl Into<String>, keys: KeySource) -> Self {
        let project_id = project_id.into();
        Self {
            issuer: format!("https://securetoken.google.com/{project_id}"),
            project_id,
            keys,
        }
    }

    pub fn is_emulator(&self) -> bool {
        matches!(self.keys, KeySource::Emulator)
    }

    pub async fn verify(&self, token: &str) -> Result<FirebaseClaims, TokenError> {
        let claims = match &self.keys {
            KeySource::Emulator => decode_unsigned(token)?,
            KeySource::Google(cache) => {
                let kid = rs256_kid(token)?;
                let key = cache.get(&kid).await?;
                self.decode_rs256(token, &key)?
            }
            KeySource::Static(keys) => {
                let kid = rs256_kid(token)?;
                let key = keys.get(&kid).ok_or(TokenError::UnknownKey)?;
                self.decode_rs256(token, key)?
            }
        };
        self.check_claims(&claims)?;
        Ok(claims)
    }

    fn decode_rs256(&self, token: &str, key: &DecodingKey) -> Result<FirebaseClaims, TokenError> {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[&self.project_id]);
        validation.set_issuer(&[&self.issuer]);
        validation.set_required_spec_claims(&["exp", "iat", "aud", "iss", "sub"]);
        validation.leeway = LEEWAY_SECS;
        Ok(decode::<FirebaseClaims>(token, key, &validation)?.claims)
    }

    /// Checks shared by every key source. The RS256 path has already validated
    /// `exp`/`aud`/`iss` via jsonwebtoken; they are re-checked here so the
    /// emulator path gets identical rules.
    fn check_claims(&self, c: &FirebaseClaims) -> Result<(), TokenError> {
        let now = now_secs();
        let leeway = LEEWAY_SECS as i64;
        if c.aud != self.project_id {
            return Err(TokenError::Claim("aud"));
        }
        if c.iss != self.issuer {
            return Err(TokenError::Claim("iss"));
        }
        if c.exp + leeway < now {
            return Err(TokenError::Claim("exp"));
        }
        if c.iat - leeway > now {
            return Err(TokenError::Claim("iat"));
        }
        if c.auth_time.is_some_and(|t| t - leeway > now) {
            return Err(TokenError::Claim("auth_time"));
        }
        if c.sub.is_empty() || c.sub.len() > 128 {
            return Err(TokenError::Claim("sub"));
        }
        Ok(())
    }
}

fn now_secs() -> i64 {
    chrono::Utc::now().timestamp()
}

fn rs256_kid(token: &str) -> Result<String, TokenError> {
    let header = decode_header(token).map_err(|e| TokenError::Malformed(e.to_string()))?;
    if header.alg != Algorithm::RS256 {
        return Err(TokenError::Algorithm(format!("{:?}", header.alg)));
    }
    header
        .kid
        .ok_or(TokenError::Malformed("missing kid".into()))
}

/// Decodes an emulator token without a signature check (the emulator signs nothing).
fn decode_unsigned(token: &str) -> Result<FirebaseClaims, TokenError> {
    let mut parts = token.split('.');
    let (Some(header), Some(payload)) = (parts.next(), parts.next()) else {
        return Err(TokenError::Malformed(
            "expected header.payload.signature".into(),
        ));
    };
    let header: serde_json::Value = decode_segment(header)?;
    let alg = header
        .get("alg")
        .and_then(|a| a.as_str())
        .unwrap_or_default();
    if alg != "none" {
        return Err(TokenError::Algorithm(alg.to_owned()));
    }
    decode_segment(payload)
}

fn decode_segment<T: serde::de::DeserializeOwned>(segment: &str) -> Result<T, TokenError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(segment.trim_end_matches('='))
        .map_err(|e| TokenError::Malformed(e.to_string()))?;
    serde_json::from_slice(&bytes).map_err(|e| TokenError::Malformed(e.to_string()))
}

/// Google's signing keys, cached until the response's `Cache-Control: max-age`
/// runs out and refetched early when a token names a `kid` we haven't seen.
struct JwksCache {
    http: reqwest::Client,
    url: String,
    state: RwLock<CachedKeys>,
    /// Serialises refreshes so concurrent misses trigger one fetch.
    refresh: Mutex<()>,
}

#[derive(Default)]
struct CachedKeys {
    keys: HashMap<String, DecodingKey>,
    expires_at: Option<Instant>,
    fetched_at: Option<Instant>,
}

impl CachedKeys {
    fn lookup(&self, kid: &str, now: Instant) -> Lookup {
        let fresh = self.expires_at.is_some_and(|t| now < t);
        match self.keys.get(kid) {
            Some(key) if fresh => Lookup::Hit(key.clone()),
            None if fresh
                && self
                    .fetched_at
                    .is_some_and(|t| now.duration_since(t) < MIN_REFETCH_INTERVAL) =>
            {
                Lookup::Unknown
            }
            _ => Lookup::Refresh,
        }
    }
}

enum Lookup {
    Hit(DecodingKey),
    Unknown,
    Refresh,
}

impl JwksCache {
    fn new(http: reqwest::Client, url: &str) -> Self {
        Self {
            http,
            url: url.to_owned(),
            state: RwLock::default(),
            refresh: Mutex::new(()),
        }
    }

    async fn get(&self, kid: &str) -> Result<DecodingKey, TokenError> {
        match self.state.read().await.lookup(kid, Instant::now()) {
            Lookup::Hit(key) => return Ok(key),
            Lookup::Unknown => return Err(TokenError::UnknownKey),
            Lookup::Refresh => {}
        }

        let _guard = self.refresh.lock().await;
        // Another task may have refreshed while we waited for the lock.
        match self.state.read().await.lookup(kid, Instant::now()) {
            Lookup::Hit(key) => return Ok(key),
            Lookup::Unknown => return Err(TokenError::UnknownKey),
            Lookup::Refresh => {}
        }

        let fetched = self.fetch().await?;
        let mut state = self.state.write().await;
        *state = fetched;
        state.keys.get(kid).cloned().ok_or(TokenError::UnknownKey)
    }

    async fn fetch(&self) -> Result<CachedKeys, TokenError> {
        let res = self
            .http
            .get(&self.url)
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|e| TokenError::Jwks(e.to_string()))?;
        let max_age = res
            .headers()
            .get(reqwest::header::CACHE_CONTROL)
            .and_then(|v| v.to_str().ok())
            .and_then(parse_max_age)
            .unwrap_or(DEFAULT_MAX_AGE);
        let set: JwkSet = res
            .json()
            .await
            .map_err(|e| TokenError::Jwks(e.to_string()))?;

        let keys: HashMap<_, _> = set
            .keys
            .iter()
            .filter_map(|jwk| {
                let kid = jwk.common.key_id.clone()?;
                DecodingKey::from_jwk(jwk).ok().map(|key| (kid, key))
            })
            .collect();
        if keys.is_empty() {
            return Err(TokenError::Jwks("JWKS contained no usable keys".into()));
        }
        tracing::debug!(count = keys.len(), ?max_age, "refreshed Firebase JWKS");

        let now = Instant::now();
        Ok(CachedKeys {
            keys,
            expires_at: Some(now + max_age),
            fetched_at: Some(now),
        })
    }
}

/// Extracts `max-age` seconds from a `Cache-Control` header value.
fn parse_max_age(header: &str) -> Option<Duration> {
    header.split(',').find_map(|directive| {
        let (name, value) = directive.trim().split_once('=')?;
        name.trim()
            .eq_ignore_ascii_case("max-age")
            .then(|| value.trim().trim_matches('"').parse().ok())
            .flatten()
            .map(Duration::from_secs)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_max_age() {
        assert_eq!(
            parse_max_age("public, max-age=19204, must-revalidate, no-transform"),
            Some(Duration::from_secs(19204))
        );
        assert_eq!(parse_max_age("Max-Age=60"), Some(Duration::from_secs(60)));
        assert_eq!(parse_max_age("no-cache"), None);
        assert_eq!(parse_max_age("max-age=abc"), None);
    }

    #[test]
    fn unknown_kid_refetch_is_throttled() {
        let now = Instant::now();
        let state = CachedKeys {
            keys: HashMap::new(),
            expires_at: Some(now + Duration::from_secs(600)),
            fetched_at: Some(now),
        };
        assert!(matches!(state.lookup("nope", now), Lookup::Unknown));
        let later = now + MIN_REFETCH_INTERVAL + Duration::from_secs(1);
        assert!(matches!(state.lookup("nope", later), Lookup::Refresh));
    }

    #[test]
    fn expired_cache_refreshes() {
        let now = Instant::now();
        let state = CachedKeys {
            keys: HashMap::new(),
            expires_at: Some(now),
            fetched_at: Some(now),
        };
        assert!(matches!(state.lookup("any", now), Lookup::Refresh));
    }

    #[tokio::test]
    async fn emulator_rejects_signed_tokens_and_checks_claims() {
        let verifier = FirebaseVerifier::emulator("demo-vigo");
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"RS256","typ":"JWT"}"#);
        let err = verifier.verify(&format!("{header}.e30.")).await;
        assert!(matches!(err, Err(TokenError::Algorithm(_))));

        let now = now_secs();
        let payload = serde_json::json!({
            "sub": "uid-1", "aud": "other-project",
            "iss": "https://securetoken.google.com/other-project",
            "exp": now + 3600, "iat": now,
        });
        let token = format!(
            "{}.{}.",
            URL_SAFE_NO_PAD.encode(r#"{"alg":"none","typ":"JWT"}"#),
            URL_SAFE_NO_PAD.encode(payload.to_string())
        );
        assert!(matches!(
            verifier.verify(&token).await,
            Err(TokenError::Claim("aud"))
        ));
    }
}
