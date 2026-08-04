//! Watches, read cursors, notifications — lean engagement for v0.6.

use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::Notification;

pub struct EngagementService;

impl EngagementService {
    // ── Watch ─────────────────────────────────────────────────────────────

    pub async fn watch(
        db: &SqlitePool,
        user_id: i64,
        target_type: &str,
        target_id: i64,
    ) -> AppResult<()> {
        if !matches!(target_type, "thread" | "category") {
            return Err(AppError::BadRequest("invalid watch target".into()));
        }
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO watches (user_id, target_type, target_id)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(user_id)
        .bind(target_type)
        .bind(target_id)
        .execute(db)
        .await?;
        Ok(())
    }

    pub async fn unwatch(
        db: &SqlitePool,
        user_id: i64,
        target_type: &str,
        target_id: i64,
    ) -> AppResult<()> {
        sqlx::query("DELETE FROM watches WHERE user_id = ? AND target_type = ? AND target_id = ?")
            .bind(user_id)
            .bind(target_type)
            .bind(target_id)
            .execute(db)
            .await?;
        Ok(())
    }

    pub async fn is_watching(
        db: &SqlitePool,
        user_id: i64,
        target_type: &str,
        target_id: i64,
    ) -> AppResult<bool> {
        let n: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM watches WHERE user_id = ? AND target_type = ? AND target_id = ?",
        )
        .bind(user_id)
        .bind(target_type)
        .bind(target_id)
        .fetch_one(db)
        .await?;
        Ok(n > 0)
    }

    /// Batch: which of `target_ids` is the user watching?
    pub async fn watching_set(
        db: &SqlitePool,
        user_id: i64,
        target_type: &str,
        target_ids: &[i64],
    ) -> AppResult<std::collections::HashSet<i64>> {
        use std::collections::HashSet;
        if target_ids.is_empty() {
            return Ok(HashSet::new());
        }
        // SQLite has a low bind limit; pages are small (≤50). Build IN list safely.
        let placeholders = target_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT target_id FROM watches WHERE user_id = ? AND target_type = ? AND target_id IN ({placeholders})"
        );
        let mut q = sqlx::query_scalar::<_, i64>(&sql)
            .bind(user_id)
            .bind(target_type);
        for id in target_ids {
            q = q.bind(id);
        }
        let rows = q.fetch_all(db).await?;
        Ok(rows.into_iter().collect())
    }

    /// Batch read cursors for threads.
    pub async fn read_cursors(
        db: &SqlitePool,
        user_id: i64,
        thread_ids: &[i64],
    ) -> AppResult<std::collections::HashMap<i64, String>> {
        use std::collections::HashMap;
        if thread_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let placeholders = thread_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT thread_id, last_read_at FROM thread_reads WHERE user_id = ? AND thread_id IN ({placeholders})"
        );
        let mut q = sqlx::query_as::<_, (i64, String)>(&sql).bind(user_id);
        for id in thread_ids {
            q = q.bind(id);
        }
        let rows = q.fetch_all(db).await?;
        Ok(rows.into_iter().collect())
    }

    // ── Read cursor ───────────────────────────────────────────────────────

    pub async fn mark_thread_read(db: &SqlitePool, user_id: i64, thread_id: i64) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO thread_reads (user_id, thread_id, last_read_at)
            VALUES (?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            ON CONFLICT(user_id, thread_id) DO UPDATE SET
                last_read_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            "#,
        )
        .bind(user_id)
        .bind(thread_id)
        .execute(db)
        .await?;
        Ok(())
    }

    // ── Notifications ─────────────────────────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    pub async fn notify(
        db: &SqlitePool,
        user_id: i64,
        kind: &str,
        actor_id: Option<i64>,
        thread_id: Option<i64>,
        post_id: Option<i64>,
        category_id: Option<i64>,
        body: &str,
    ) -> AppResult<()> {
        if actor_id == Some(user_id) {
            return Ok(()); // never notify self
        }
        sqlx::query(
            r#"
            INSERT INTO notifications (
                user_id, kind, actor_id, thread_id, post_id, category_id, body
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(user_id)
        .bind(kind)
        .bind(actor_id)
        .bind(thread_id)
        .bind(post_id)
        .bind(category_id)
        .bind(body)
        .execute(db)
        .await?;
        Ok(())
    }

    /// Fan-out to watchers of a thread (and its category).
    pub async fn notify_watchers(
        db: &SqlitePool,
        thread_id: i64,
        category_id: i64,
        actor_id: i64,
        kind: &str,
        post_id: Option<i64>,
        body: &str,
    ) -> AppResult<()> {
        let watchers: Vec<i64> = sqlx::query_scalar(
            r#"
            SELECT DISTINCT user_id FROM watches
            WHERE (target_type = 'thread' AND target_id = ?)
               OR (target_type = 'category' AND target_id = ?)
            "#,
        )
        .bind(thread_id)
        .bind(category_id)
        .fetch_all(db)
        .await?;

        for uid in watchers {
            Self::notify(
                db,
                uid,
                kind,
                Some(actor_id),
                Some(thread_id),
                post_id,
                Some(category_id),
                body,
            )
            .await?;
        }
        Ok(())
    }

    pub async fn list_notifications(
        db: &SqlitePool,
        user_id: i64,
        limit: i64,
        offset: i64,
        unread_only: bool,
    ) -> AppResult<(Vec<Notification>, i64)> {
        let total = if unread_only {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM notifications WHERE user_id = ? AND is_read = 0",
            )
            .bind(user_id)
            .fetch_one(db)
            .await?
        } else {
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM notifications WHERE user_id = ?")
                .bind(user_id)
                .fetch_one(db)
                .await?
        };

        let sql = if unread_only {
            r#"
            SELECT * FROM notifications
            WHERE user_id = ? AND is_read = 0
            ORDER BY created_at DESC, id DESC
            LIMIT ? OFFSET ?
            "#
        } else {
            r#"
            SELECT * FROM notifications
            WHERE user_id = ?
            ORDER BY created_at DESC, id DESC
            LIMIT ? OFFSET ?
            "#
        };
        let rows = sqlx::query_as::<_, Notification>(sql)
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(db)
            .await?;
        Ok((rows, total))
    }

    pub async fn unread_count(db: &SqlitePool, user_id: i64) -> AppResult<i64> {
        let n = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM notifications WHERE user_id = ? AND is_read = 0",
        )
        .bind(user_id)
        .fetch_one(db)
        .await?;
        Ok(n)
    }

    pub async fn mark_read(db: &SqlitePool, user_id: i64, ids: &[i64]) -> AppResult<()> {
        if ids.is_empty() {
            sqlx::query("UPDATE notifications SET is_read = 1 WHERE user_id = ? AND is_read = 0")
                .bind(user_id)
                .execute(db)
                .await?;
            return Ok(());
        }
        for id in ids {
            sqlx::query("UPDATE notifications SET is_read = 1 WHERE id = ? AND user_id = ?")
                .bind(id)
                .bind(user_id)
                .execute(db)
                .await?;
        }
        Ok(())
    }
}
