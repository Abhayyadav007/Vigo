use std::{
    env,
    net::SocketAddr,
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};

use anyhow::{Context, bail};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEnv {
    Development,
    Production,
}

impl FromStr for AppEnv {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "development" | "dev" | "local" => Ok(Self::Development),
            "production" | "prod" => Ok(Self::Production),
            other => bail!("invalid APP_ENV `{other}` (expected development|production)"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub app_env: AppEnv,
    pub addr: SocketAddr,
    pub database_url: String,
    pub db_max_connections: u32,
    pub redis_url: String,
    pub firebase_project_id: String,
    /// `host:port` of the Firebase Auth emulator. When set, the backend accepts
    /// the emulator's unsigned tokens. Refused when `APP_ENV=production`.
    pub firebase_auth_emulator_host: Option<String>,
    /// How long `firebase_uid -> session` lookups stay cached in Redis.
    pub session_cache_ttl: Duration,
    pub cors_allowed_origins: Vec<String>,
    pub request_timeout: Duration,
    /// Where uploaded images are stored (local media store).
    pub media_dir: PathBuf,
    /// How long checkout holds stock for an unpaid order.
    pub reservation_ttl: Duration,
    /// Razorpay key id + webhook secret. Online payments are off unless both are set.
    pub razorpay: Option<(String, String)>,
    pub dispatch: DispatchConfig,
}

/// Rider dispatch tuning.
#[derive(Debug, Clone)]
pub struct DispatchConfig {
    /// Only riders within this distance of the store get offers.
    pub radius_m: f64,
    /// How long riders have to accept an offer.
    pub offer_ttl: Duration,
    /// Riders offered each order at once (first to accept wins).
    pub wave_size: usize,
    /// Riders with no location fix for this long are skipped.
    pub stale_after: Duration,
}

impl Default for DispatchConfig {
    fn default() -> Self {
        Self {
            radius_m: 5_000.0,
            offer_ttl: Duration::from_secs(30),
            wave_size: 3,
            stale_after: Duration::from_secs(60),
        }
    }
}

impl Config {
    /// Reads configuration from the process environment (and `.env` if present).
    /// Relative paths (`MEDIA_DIR`) resolve against `base_dir`: the directory of
    /// the loaded `.env`, so they don't depend on where the binary was started.
    pub fn from_env(base_dir: Option<&Path>) -> anyhow::Result<Self> {
        let host = var_or("HOST", "0.0.0.0");
        let port: u16 = parse_or("PORT", 8080)?;
        let addr = format!("{host}:{port}")
            .parse()
            .with_context(|| format!("invalid HOST/PORT `{host}:{port}`"))?;

        let app_env = parse_or("APP_ENV", AppEnv::Development)?;
        let firebase_auth_emulator_host = env::var("FIREBASE_AUTH_EMULATOR_HOST")
            .ok()
            .filter(|s| !s.trim().is_empty());
        if app_env == AppEnv::Production && firebase_auth_emulator_host.is_some() {
            bail!("FIREBASE_AUTH_EMULATOR_HOST must not be set when APP_ENV=production");
        }

        Ok(Self {
            app_env,
            addr,
            database_url: required("DATABASE_URL")?,
            db_max_connections: parse_or("DB_MAX_CONNECTIONS", 10)?,
            redis_url: required("REDIS_URL")?,
            firebase_project_id: required("FIREBASE_PROJECT_ID")?,
            firebase_auth_emulator_host,
            session_cache_ttl: Duration::from_secs(parse_or("SESSION_CACHE_TTL_SECS", 300)?),
            cors_allowed_origins: var_or("CORS_ALLOWED_ORIGINS", "")
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect(),
            request_timeout: Duration::from_secs(parse_or("REQUEST_TIMEOUT_SECS", 15)?),
            reservation_ttl: Duration::from_secs(parse_or("RESERVATION_TTL_SECS", 600)?),
            dispatch: DispatchConfig {
                radius_m: parse_or("DISPATCH_RADIUS_M", 5_000.0)?,
                offer_ttl: Duration::from_secs(parse_or("DISPATCH_OFFER_TTL_SECS", 30)?),
                wave_size: parse_or("DISPATCH_WAVE_SIZE", 3)?,
                stale_after: Duration::from_secs(parse_or("RIDER_STALE_SECS", 60)?),
            },
            razorpay: match (
                env::var("RAZORPAY_KEY_ID").ok().filter(|s| !s.is_empty()),
                env::var("RAZORPAY_WEBHOOK_SECRET")
                    .ok()
                    .filter(|s| !s.is_empty()),
            ) {
                (Some(key), Some(secret)) => Some((key, secret)),
                _ => None,
            },
            media_dir: {
                let dir = PathBuf::from(var_or("MEDIA_DIR", "media"));
                match base_dir {
                    Some(base) if dir.is_relative() => base.join(dir),
                    _ => dir,
                }
            },
        })
    }

    pub fn is_production(&self) -> bool {
        self.app_env == AppEnv::Production
    }
}

fn required(key: &str) -> anyhow::Result<String> {
    env::var(key).with_context(|| format!("missing required env var {key}"))
}

fn var_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_owned())
}

fn parse_or<T>(key: &str, default: T) -> anyhow::Result<T>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    match env::var(key) {
        Ok(raw) => raw
            .parse()
            .map_err(|e| anyhow::anyhow!("invalid {key} `{raw}`: {e}")),
        Err(_) => Ok(default),
    }
}
