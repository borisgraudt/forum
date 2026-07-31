use sqlx::{SqlitePool, Transaction};

use crate::error::{AppError, AppResult};
use crate::models::{Post, Thread};
use crate::services::ThreadService;

pub struct PostService;

impl PostService {
    pub async fn list_by_thread(
        db: &SqlitePool,
        thread_id: i64,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<Post>> {
        let rows = sqlx::query_as::<_, Post>(
            r#"
            SELECT * FROM posts
            WHERE thread_id = ?
            ORDER BY created_at ASC, id ASC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(thread_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    pub async fn create_reply(
        db: &SqlitePool,
        thread_id: i64,
        author_id: i64,
        body: &str,
    ) -> AppResult<Post> {
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

        let _thread: Thread = sqlx::query_as(
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
        Ok(post)
    }
}
