use sqlx::{SqlitePool, Transaction};

use crate::error::{AppError, AppResult};
use crate::models::{Post, PostView, Thread};
use crate::services::ThreadService;

const POST_VIEW_SELECT: &str = r#"
    SELECT
        p.id, p.thread_id, p.author_id, p.body, p.created_at, p.updated_at,
        u.username AS author_username,
        u.display_name AS author_display_name
    FROM posts p
    INNER JOIN users u ON u.id = p.author_id
"#;

pub struct PostService;

impl PostService {
    pub async fn list_by_thread(
        db: &SqlitePool,
        thread_id: i64,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<PostView>> {
        let sql = format!(
            "{POST_VIEW_SELECT}
            WHERE p.thread_id = ?
            ORDER BY p.created_at ASC, p.id ASC
            LIMIT ? OFFSET ?"
        );
        let rows = sqlx::query_as::<_, PostView>(&sql)
            .bind(thread_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(db)
            .await?;
        Ok(rows)
    }

    pub async fn count_by_thread(db: &SqlitePool, thread_id: i64) -> AppResult<i64> {
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM posts WHERE thread_id = ?")
            .bind(thread_id)
            .fetch_one(db)
            .await?;
        Ok(count)
    }

    pub async fn get_view(db: &SqlitePool, post_id: i64) -> AppResult<PostView> {
        ThreadService::get_post_view(db, post_id).await
    }

    pub async fn create_reply(
        db: &SqlitePool,
        thread_id: i64,
        author_id: i64,
        body: &str,
    ) -> AppResult<PostView> {
        let thread = ThreadService::get_by_id(db, thread_id).await?;
        if thread.is_locked {
            return Err(AppError::Forbidden);
        }

        let mut tx: Transaction<'_, sqlx::Sqlite> = db.begin().await?;

        let post = sqlx::query_as::<_, Post>(
            r#"
            INSERT INTO posts (thread_id, author_id, body)
            VALUES (?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(thread_id)
        .bind(author_id)
        .bind(body)
        .fetch_one(&mut *tx)
        .await?;

        let _: Thread = sqlx::query_as(
            r#"
            UPDATE threads
            SET post_count = post_count + 1,
                last_post_at = ?,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            WHERE id = ?
            RETURNING *
            "#,
        )
        .bind(&post.created_at)
        .bind(thread_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        ThreadService::get_post_view(db, post.id).await
    }

    pub async fn delete(db: &SqlitePool, post_id: i64) -> AppResult<()> {
        let post = sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE id = ?")
            .bind(post_id)
            .fetch_optional(db)
            .await?
            .ok_or(AppError::NotFound)?;

        let mut tx: Transaction<'_, sqlx::Sqlite> = db.begin().await?;

        let result = sqlx::query("DELETE FROM posts WHERE id = ?")
            .bind(post_id)
            .execute(&mut *tx)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound);
        }

        // Keep post_count consistent (never below 0).
        sqlx::query(
            r#"
            UPDATE threads
            SET post_count = MAX(post_count - 1, 0),
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            WHERE id = ?
            "#,
        )
        .bind(post.thread_id)
        .execute(&mut *tx)
        .await?;

        // Refresh last_post_at from remaining posts.
        sqlx::query(
            r#"
            UPDATE threads
            SET last_post_at = (
                SELECT MAX(created_at) FROM posts WHERE thread_id = threads.id
            )
            WHERE id = ?
            "#,
        )
        .bind(post.thread_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }
}
