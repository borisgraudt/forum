use anyhow::{Context, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{ConnectOptions, SqlitePool};
use std::str::FromStr;
use std::time::Duration;
use tracing::log::LevelFilter;

pub async fn connect(database_url: &str) -> Result<SqlitePool> {
    // Hot path: never log every statement in production (was LevelFilter::Debug).
    // Only surface slow queries. Larger page cache + temp_store=memory for speed.
    let options = SqliteConnectOptions::from_str(database_url)
        .with_context(|| format!("invalid DATABASE_URL: {database_url}"))?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5))
        .pragma("cache_size", "-65536") // ~64 MiB page cache
        .pragma("temp_store", "MEMORY")
        .pragma("mmap_size", "268435456") // 256 MiB mmap
        .log_statements(LevelFilter::Off)
        .log_slow_statements(LevelFilter::Warn, Duration::from_millis(100));

    let pool = SqlitePoolOptions::new()
        .max_connections(16)
        .acquire_timeout(Duration::from_secs(3))
        .connect_with(options)
        .await
        .context("failed to connect to sqlite")?;

    Ok(pool)
}

pub async fn migrate(pool: &SqlitePool) -> Result<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .context("failed to run database migrations")?;
    Ok(())
}
