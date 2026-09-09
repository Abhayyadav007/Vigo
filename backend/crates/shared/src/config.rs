use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub port: u16,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("missing required environment variable: {0}")]
    Missing(&'static str),
    #[error("invalid value for {name}: {value}")]
    Invalid { name: &'static str, value: String },
}

impl Config {
    /// Loads `.env` if present, then reads the environment. Fails loudly at
    /// startup rather than leaving a half-configured process running.
    pub fn from_env() -> Result<Self, ConfigError> {
        let _ = dotenvy::dotenv();

        let database_url =
            env::var("DATABASE_URL").map_err(|_| ConfigError::Missing("DATABASE_URL"))?;

        let port = match env::var("PORT") {
            Ok(raw) => raw.parse().map_err(|_| ConfigError::Invalid {
                name: "PORT",
                value: raw,
            })?,
            Err(_) => 3000,
        };

        Ok(Self { database_url, port })
    }
}
