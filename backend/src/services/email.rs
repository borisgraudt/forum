//! Minimal mailer: log in dev, SMTP later via env.
//! Tokens for verify + password reset.

use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};

pub struct EmailService;

impl EmailService {
    pub fn hash_token(token: &str) -> String {
        let mut h = Sha256::new();
        h.update(token.as_bytes());
        hex::encode(h.finalize())
    }

    pub fn generate_token() -> String {
        let mut buf = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut buf);
        hex::encode(buf)
    }

    pub async fn issue_token(
        db: &SqlitePool,
        user_id: i64,
        kind: &str,
        ttl_hours: i64,
    ) -> AppResult<String> {
        if !matches!(kind, "verify" | "reset") {
            return Err(AppError::BadRequest("invalid token kind".into()));
        }
        let raw = Self::generate_token();
        let hash = Self::hash_token(&raw);
        // invalidate previous unused
        sqlx::query(
            r#"
            UPDATE email_tokens SET used_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            WHERE user_id = ? AND kind = ? AND used_at IS NULL
            "#,
        )
        .bind(user_id)
        .bind(kind)
        .execute(db)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO email_tokens (user_id, kind, token_hash, expires_at)
            VALUES (?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now', ?))
            "#,
        )
        .bind(user_id)
        .bind(kind)
        .bind(&hash)
        .bind(format!("+{ttl_hours} hours"))
        .execute(db)
        .await?;

        Ok(raw)
    }

    pub async fn consume_token(db: &SqlitePool, kind: &str, raw_token: &str) -> AppResult<i64> {
        let hash = Self::hash_token(raw_token);
        let user_id: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT user_id FROM email_tokens
            WHERE token_hash = ?
              AND kind = ?
              AND used_at IS NULL
              AND expires_at > strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            "#,
        )
        .bind(&hash)
        .bind(kind)
        .fetch_optional(db)
        .await?;

        let user_id = user_id.ok_or(AppError::BadRequest("invalid or expired token".into()))?;

        sqlx::query(
            r#"
            UPDATE email_tokens
            SET used_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            WHERE token_hash = ?
            "#,
        )
        .bind(&hash)
        .execute(db)
        .await?;

        Ok(user_id)
    }

    /// Dev-friendly: always log. Optional SMTP later.
    pub fn send_log(kind: &str, to: &str, subject: &str, body: &str) {
        tracing::info!(%kind, %to, %subject, body = %body, "email (dev log transport)");
    }
}
