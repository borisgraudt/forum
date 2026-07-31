use sqlx::{SqlitePool, Transaction};

use crate::error::{AppError, AppResult};
use crate::models::{Post, Thread};

pub struct ThreadService;

impl ThreadService {
    pub async fn list_by_category(
        db: &SqlitePool,
        category_id: i64,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<Thread>> {
        let rows = sqlx::query_as::<_, Thread>(
            r#"
            SELECT * FROM threads
            WHERE category_id = ?
            ORDER BY is_pinned DESC, COALESCE(last_post_at, created_at) DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(category_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    pub async fn get_by_id(db: &SqlitePool, id: i64) -> AppResult<Thread> {
        sqlx::query_as::<_, Thread>("SELECT * FROM threads WHERE id = ?")
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or(AppError::NotFound)
    }

    pub async fn get_by_category_and_slug(
        db: &SqlitePool,
        category_id: i64,
        slug: &str,
    ) -> AppResult<Thread> {
        sqlx::query_as::<_, Thread>(
            r#"
            SELECT * FROM threads
            WHERE category_id = ? AND slug = ? COLLATE NOCASE
            "#,
        )
        .bind(category_id)
        .bind(slug)
        .fetch_optional(db)
        .await?
        .ok_or(AppError::NotFound)
    }

    /// Create a thread and its opening post in one transaction.
    pub async fn create_with_first_post(
        db: &SqlitePool,
        category_id: i64,
        author_id: i64,
        title: &str,
        slug: &str,
        body: &str,
    ) -> AppResult<(Thread, Post)> {
        let mut tx: Transaction<'_, sqlx::Sqlite> = db.begin().await?;

        let thread = sqlx::query_as::<_, Thread>(
            r#"
            INSERT INTO threads (
                category_id, author_id, title, slug, post_count, last_post_at
            )
            VALUES (?, ?, ?, ?, 0, NULL)
            RETURNING *
            "#,
        )
        .bind(category_id)
        .bind(author_id)
        .bind(title)
        .bind(slug)
        .fetch_one(&mut *tx)
        .await
        .map_err(|err| match err {
            sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
                AppError::Conflict("thread slug already exists in this category".into())
            }
            other => AppError::Sqlx(other),
        })?;

        let post = sqlx::query_as::<_, Post>(
            r#"
            INSERT INTO posts (thread_id, author_id, body)
            VALUES (?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(thread.id)
        .bind(author_id)
        .bind(body)
        .fetch_one(&mut *tx)
        .await?;

        let thread = sqlx::query_as::<_, Thread>(
            r#"
            UPDATE threads
            SET post_count = 1,
                last_post_at = ?,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            WHERE id = ?
            RETURNING *
            "#,
        )
        .bind(&post.created_at)
        .bind(thread.id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok((thread, post))
    }
}
