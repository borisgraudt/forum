use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::Draft;

pub struct DraftService;

impl DraftService {
    pub async fn list_for_user(db: &SqlitePool, user_id: i64) -> AppResult<Vec<Draft>> {
        let rows = sqlx::query_as::<_, Draft>(
            r#"
            SELECT * FROM drafts
            WHERE user_id = ?
            ORDER BY updated_at DESC, id DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    pub async fn upsert(
        db: &SqlitePool,
        user_id: i64,
        kind: &str,
        category_slug: Option<&str>,
        thread_id: Option<i64>,
        title: Option<&str>,
        body: &str,
    ) -> AppResult<Draft> {
        if kind != "thread" && kind != "reply" {
            return Err(AppError::BadRequest(
                "kind must be 'thread' or 'reply'".into(),
            ));
        }

        // Match existing draft by kind + category + thread for this user.
        let existing = sqlx::query_as::<_, Draft>(
            r#"
            SELECT * FROM drafts
            WHERE user_id = ?
              AND kind = ?
              AND IFNULL(category_slug, '') = IFNULL(?, '')
              AND IFNULL(thread_id, -1) = IFNULL(?, -1)
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .bind(kind)
        .bind(category_slug)
        .bind(thread_id)
        .fetch_optional(db)
        .await?;

        if let Some(d) = existing {
            sqlx::query(
                r#"
                UPDATE drafts
                SET title = ?,
                    body = ?,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                WHERE id = ?
                "#,
            )
            .bind(title)
            .bind(body)
            .bind(d.id)
            .execute(db)
            .await?;
            return Self::get(db, d.id, user_id).await;
        }

        let id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO drafts (user_id, kind, category_slug, thread_id, title, body)
            VALUES (?, ?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(user_id)
        .bind(kind)
        .bind(category_slug)
        .bind(thread_id)
        .bind(title)
        .bind(body)
        .fetch_one(db)
        .await?;

        Self::get(db, id, user_id).await
    }

    pub async fn get(db: &SqlitePool, id: i64, user_id: i64) -> AppResult<Draft> {
        sqlx::query_as::<_, Draft>("SELECT * FROM drafts WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user_id)
            .fetch_optional(db)
            .await?
            .ok_or(AppError::NotFound)
    }

    pub async fn delete(db: &SqlitePool, id: i64, user_id: i64) -> AppResult<()> {
        let res = sqlx::query("DELETE FROM drafts WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user_id)
            .execute(db)
            .await?;
        if res.rows_affected() == 0 {
            return Err(AppError::NotFound);
        }
        Ok(())
    }
}
