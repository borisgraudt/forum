use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{bail, Context, Result};

/// Default session lifetime: 7 days.
pub const DEFAULT_JWT_TTL_SECS: u64 = 60 * 60 * 24 * 7;

pub const AUTH_COOKIE_NAME: &str = "session";

#[derive(Clone, Debug)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_ttl: Duration,
    pub cookie_secure: bool,
    /// Send Strict-Transport-Security (also implied when cookie_secure).
    pub enable_hsts: bool,
    /// `development` | `production` — production refuses weak secrets.
    pub forum_env: String,
    pub cors_origin: String,
    pub rust_log: String,
    /// Local media root (uploads live under `{data_dir}/uploads`).
    pub data_dir: PathBuf,
    /// Public site origin for links in emails (e.g. http://localhost:4321).
    pub public_origin: String,
}

fn env_truthy(key: &str, default: bool) -> bool {
    env::var(key)
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(default)
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let _ = dotenvy::dotenv();

        let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into());
        let port = env::var("PORT")
            .unwrap_or_else(|_| "3000".into())
            .parse::<u16>()
            .context("PORT must be a valid u16")?;
        let database_url =
            env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:forum.db?mode=rwc".into());
        let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| "dev-only-change-me".into());
        let jwt_ttl_secs = env::var("JWT_TTL_SECS")
            .ok()
            .map(|v| v.parse::<u64>())
            .transpose()
            .context("JWT_TTL_SECS must be a positive integer")?
            .unwrap_or(DEFAULT_JWT_TTL_SECS);
        let cookie_secure = env_truthy("COOKIE_SECURE", false);
        let enable_hsts = env_truthy("ENABLE_HSTS", cookie_secure);
        let forum_env = env::var("FORUM_ENV")
            .unwrap_or_else(|_| "development".into())
            .to_ascii_lowercase();
        let cors_origin =
            env::var("CORS_ORIGIN").unwrap_or_else(|_| "http://localhost:4321".into());
        let rust_log = env::var("RUST_LOG").unwrap_or_else(|_| "info,forum_backend=debug".into());
        let data_dir = env::var("DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./data"));
        let public_origin = env::var("PUBLIC_ORIGIN").unwrap_or_else(|_| cors_origin.clone());

        if jwt_secret.len() < 16 {
            bail!("JWT_SECRET must be at least 16 characters");
        }
        if jwt_ttl_secs == 0 {
            bail!("JWT_TTL_SECS must be greater than 0");
        }

        let is_prod = forum_env == "production" || forum_env == "prod";
        if is_prod {
            let weak = jwt_secret == "dev-only-change-me"
                || jwt_secret == "dev-only-change-me-please"
                || jwt_secret == "change-me-to-a-long-random-string-at-least-32-chars"
                || jwt_secret.len() < 32;
            if weak {
                bail!("FORUM_ENV=production requires a strong JWT_SECRET (32+ random chars)");
            }
            if !cookie_secure {
                eprintln!(
                    "warning: COOKIE_SECURE=false under production — set COOKIE_SECURE=true behind HTTPS"
                );
            }
        }

        Ok(Self {
            host,
            port,
            database_url,
            jwt_secret,
            jwt_ttl: Duration::from_secs(jwt_ttl_secs),
            cookie_secure,
            enable_hsts,
            forum_env,
            cors_origin,
            rust_log,
            data_dir,
            public_origin,
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
            jwt_ttl: Duration::from_secs(DEFAULT_JWT_TTL_SECS),
            cookie_secure: false,
            enable_hsts: false,
            forum_env: "development".into(),
            cors_origin: "http://localhost:4321".into(),
            rust_log: "info".into(),
            data_dir: PathBuf::from("./data"),
            public_origin: "http://localhost:4321".into(),
        };
        assert_eq!(cfg.socket_addr().unwrap().to_string(), "127.0.0.1:3000");
    }
}
