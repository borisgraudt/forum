use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::{AuditEntry, Report, ReportView, SanctionKind, UserSanction, UserSanctionView};

pub struct ModerationService;

impl ModerationService {
    // ── Reports ───────────────────────────────────────────────────────────

    pub async fn create_report(
        db: &SqlitePool,
        reporter_id: i64,
        target_type: &str,
        target_id: i64,
        reason: &str,
        details: Option<&str>,
    ) -> AppResult<Report> {
        if !matches!(target_type, "post" | "thread" | "user") {
            return Err(AppError::BadRequest(
                "target_type must be post, thread, or user".into(),
            ));
        }
        let reason = reason.trim();
        if reason.is_empty() || reason.len() > 200 {
            return Err(AppError::BadRequest(
                "reason must be 1-200 characters".into(),
            ));
        }

        // Prevent reporting yourself as a user target.
        if target_type == "user" && target_id == reporter_id {
            return Err(AppError::BadRequest("cannot report yourself".into()));
        }

        // Dedupe open reports from same reporter for same target.
        let existing: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM reports
            WHERE reporter_id = ?
              AND target_type = ?
              AND target_id = ?
              AND status = 'open'
            "#,
        )
        .bind(reporter_id)
        .bind(target_type)
        .bind(target_id)
        .fetch_one(db)
        .await?;
        if existing > 0 {
            return Err(AppError::Conflict(
                "you already have an open report for this target".into(),
            ));
        }

        let report = sqlx::query_as::<_, Report>(
            r#"
            INSERT INTO reports (reporter_id, target_type, target_id, reason, details)
            VALUES (?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(reporter_id)
        .bind(target_type)
        .bind(target_id)
        .bind(reason)
        .bind(details)
        .fetch_one(db)
        .await?;

        Ok(report)
    }

    pub async fn list_reports(
        db: &SqlitePool,
        status: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> AppResult<(Vec<ReportView>, i64)> {
        let status = status
            .map(str::trim)
            .filter(|s| matches!(*s, "open" | "resolved" | "dismissed"));

        let total = if let Some(st) = status {
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM reports WHERE status = ?")
                .bind(st)
                .fetch_one(db)
                .await?
        } else {
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM reports")
                .fetch_one(db)
                .await?
        };

        let rows = if let Some(st) = status {
            sqlx::query_as::<_, ReportView>(
                r#"
                SELECT
                    r.*,
                    u.username AS reporter_username
                FROM reports r
                INNER JOIN users u ON u.id = r.reporter_id
                WHERE r.status = ?
                ORDER BY r.created_at DESC, r.id DESC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(st)
            .bind(limit)
            .bind(offset)
            .fetch_all(db)
            .await?
        } else {
            sqlx::query_as::<_, ReportView>(
                r#"
                SELECT
                    r.*,
                    u.username AS reporter_username
                FROM reports r
                INNER JOIN users u ON u.id = r.reporter_id
                ORDER BY r.created_at DESC, r.id DESC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(db)
            .await?
        };

        Ok((rows, total))
    }

    pub async fn resolve_report(
        db: &SqlitePool,
        report_id: i64,
        resolver_id: i64,
        status: &str,
        note: Option<&str>,
    ) -> AppResult<Report> {
        if !matches!(status, "resolved" | "dismissed") {
            return Err(AppError::BadRequest(
                "status must be resolved or dismissed".into(),
            ));
        }

        let updated = sqlx::query_as::<_, Report>(
            r#"
            UPDATE reports
            SET status = ?,
                resolved_by = ?,
                resolved_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                resolution_note = ?,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            WHERE id = ? AND status = 'open'
            RETURNING *
            "#,
        )
        .bind(status)
        .bind(resolver_id)
        .bind(note)
        .bind(report_id)
        .fetch_optional(db)
        .await?
        .ok_or(AppError::NotFound)?;

        Ok(updated)
    }

    // ── Sanctions ─────────────────────────────────────────────────────────

    pub async fn create_sanction(
        db: &SqlitePool,
        user_id: i64,
        kind: SanctionKind,
        reason: Option<&str>,
        created_by: i64,
        ends_at: Option<&str>,
    ) -> AppResult<UserSanction> {
        if user_id == created_by {
            return Err(AppError::BadRequest("cannot sanction yourself".into()));
        }

        // Deactivate previous active sanctions of the same kind.
        sqlx::query(
            r#"
            UPDATE user_sanctions
            SET is_active = 0
            WHERE user_id = ? AND kind = ? AND is_active = 1
            "#,
        )
        .bind(user_id)
        .bind(kind.as_str())
        .execute(db)
        .await?;

        // Permanent ban also deactivates the account.
        if kind == SanctionKind::Ban {
            sqlx::query(
                r#"
                UPDATE users
                SET is_active = 0,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                WHERE id = ?
                "#,
            )
            .bind(user_id)
            .execute(db)
            .await?;
        }

        let row = sqlx::query_as::<_, UserSanction>(
            r#"
            INSERT INTO user_sanctions (user_id, kind, reason, created_by, ends_at)
            VALUES (?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(kind.as_str())
        .bind(reason)
        .bind(created_by)
        .bind(ends_at)
        .fetch_one(db)
        .await?;

        Ok(row)
    }

    pub async fn lift_sanction(db: &SqlitePool, sanction_id: i64) -> AppResult<UserSanction> {
        let existing =
            sqlx::query_as::<_, UserSanction>("SELECT * FROM user_sanctions WHERE id = ?")
                .bind(sanction_id)
                .fetch_optional(db)
                .await?
                .ok_or(AppError::NotFound)?;

        let row = sqlx::query_as::<_, UserSanction>(
            r#"
            UPDATE user_sanctions
            SET is_active = 0
            WHERE id = ?
            RETURNING *
            "#,
        )
        .bind(sanction_id)
        .fetch_one(db)
        .await?;

        // If lifting a ban, re-enable account (unless another active ban exists).
        if existing.kind == "ban" {
            let other_bans: i64 = sqlx::query_scalar(
                r#"
                SELECT COUNT(*) FROM user_sanctions
                WHERE user_id = ? AND kind = 'ban' AND is_active = 1
                  AND (ends_at IS NULL OR ends_at > strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
                "#,
            )
            .bind(existing.user_id)
            .fetch_one(db)
            .await?;
            if other_bans == 0 {
                sqlx::query(
                    r#"
                    UPDATE users
                    SET is_active = 1,
                        updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                    WHERE id = ?
                    "#,
                )
                .bind(existing.user_id)
                .execute(db)
                .await?;
            }
        }

        Ok(row)
    }

    pub async fn is_banned(db: &SqlitePool, user_id: i64) -> AppResult<bool> {
        let n: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM user_sanctions
            WHERE user_id = ? AND kind = 'ban' AND is_active = 1
              AND (ends_at IS NULL OR ends_at > strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            "#,
        )
        .bind(user_id)
        .fetch_one(db)
        .await?;
        Ok(n > 0)
    }

    /// Mute or timeout blocks posting (not reading / voting).
    pub async fn is_muted(db: &SqlitePool, user_id: i64) -> AppResult<bool> {
        let n: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM user_sanctions
            WHERE user_id = ?
              AND kind IN ('mute', 'timeout')
              AND is_active = 1
              AND (ends_at IS NULL OR ends_at > strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            "#,
        )
        .bind(user_id)
        .fetch_one(db)
        .await?;
        Ok(n > 0)
    }

    /// Banned or muted users cannot create posts/threads.
    pub async fn ensure_can_post(db: &SqlitePool, user_id: i64) -> AppResult<()> {
        if Self::is_banned(db, user_id).await? {
            return Err(AppError::Forbidden);
        }
        if Self::is_muted(db, user_id).await? {
            return Err(AppError::Forbidden);
        }
        Ok(())
    }

    pub async fn list_sanctions(
        db: &SqlitePool,
        active_only: bool,
        limit: i64,
        offset: i64,
    ) -> AppResult<(Vec<UserSanctionView>, i64)> {
        let total = if active_only {
            sqlx::query_scalar::<_, i64>(
                r#"
                SELECT COUNT(*) FROM user_sanctions
                WHERE is_active = 1
                  AND (ends_at IS NULL OR ends_at > strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
                "#,
            )
            .fetch_one(db)
            .await?
        } else {
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM user_sanctions")
                .fetch_one(db)
                .await?
        };

        let sql = if active_only {
            r#"
            SELECT
                s.*,
                u.username AS username,
                a.username AS created_by_username
            FROM user_sanctions s
            INNER JOIN users u ON u.id = s.user_id
            INNER JOIN users a ON a.id = s.created_by
            WHERE s.is_active = 1
              AND (s.ends_at IS NULL OR s.ends_at > strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            ORDER BY s.created_at DESC, s.id DESC
            LIMIT ? OFFSET ?
            "#
        } else {
            r#"
            SELECT
                s.*,
                u.username AS username,
                a.username AS created_by_username
            FROM user_sanctions s
            INNER JOIN users u ON u.id = s.user_id
            INNER JOIN users a ON a.id = s.created_by
            ORDER BY s.created_at DESC, s.id DESC
            LIMIT ? OFFSET ?
            "#
        };

        let rows = sqlx::query_as::<_, UserSanctionView>(sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(db)
            .await?;
        Ok((rows, total))
    }

    // ── Audit ─────────────────────────────────────────────────────────────

    pub async fn audit(
        db: &SqlitePool,
        actor_id: Option<i64>,
        action: &str,
        target_type: Option<&str>,
        target_id: Option<i64>,
        meta: Option<&str>,
    ) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO audit_log (actor_id, action, target_type, target_id, meta)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(actor_id)
        .bind(action)
        .bind(target_type)
        .bind(target_id)
        .bind(meta)
        .execute(db)
        .await?;
        Ok(())
    }

    pub async fn list_audit(
        db: &SqlitePool,
        limit: i64,
        offset: i64,
    ) -> AppResult<(Vec<AuditEntry>, i64)> {
        let total = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM audit_log")
            .fetch_one(db)
            .await?;
        let rows = sqlx::query_as::<_, AuditEntry>(
            r#"
            SELECT
                a.id,
                a.actor_id,
                u.username AS actor_username,
                a.action,
                a.target_type,
                a.target_id,
                a.meta,
                a.created_at
            FROM audit_log a
            LEFT JOIN users u ON u.id = a.actor_id
            ORDER BY a.created_at DESC, a.id DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await?;
        Ok((rows, total))
    }
}
