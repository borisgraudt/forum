use std::env;
use std::net::SocketAddr;

use anyhow::{bail, Context, Result};

#[derive(Clone, Debug)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    /// Loaded early so misconfiguration fails at boot (used by auth in a later PR).
    #[allow(dead_code)]
    pub jwt_secret: String,
    pub cors_origin: String,
    pub rust_log: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        // Load .env if present; missing file is fine in CI/production.
        let _ = dotenvy::dotenv();

        let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into());
        let port = env::var("PORT")
            .unwrap_or_else(|_| "3000".into())
            .parse::<u16>()
            .context("PORT must be a valid u16")?;
        let database_url =
            env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:forum.db?mode=rwc".into());
        let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| "dev-only-change-me".into());
        let cors_origin =
            env::var("CORS_ORIGIN").unwrap_or_else(|_| "http://localhost:4321".into());
        let rust_log = env::var("RUST_LOG").unwrap_or_else(|_| "info,forum_backend=debug".into());

        if jwt_secret.len() < 16 {
            bail!("JWT_SECRET must be at least 16 characters");
        }

        Ok(Self {
            host,
            port,
            database_url,
            jwt_secret,
            cors_origin,
            rust_log,
        })
    }

    pub fn socket_addr(&self) -> Result<SocketAddr> {
        let addr = format!("{}:{}", self.host, self.port);
        addr.parse()
            .with_context(|| format!("invalid listen address: {addr}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socket_addr_formats_host_and_port() {
        let cfg = Config {
            host: "127.0.0.1".into(),
            port: 3000,
            database_url: "sqlite::memory:".into(),
            jwt_secret: "long-enough-secret".into(),
            cors_origin: "http://localhost:4321".into(),
            rust_log: "info".into(),
        };
        assert_eq!(cfg.socket_addr().unwrap().to_string(), "127.0.0.1:3000");
    }
}
