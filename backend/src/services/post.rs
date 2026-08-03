use sqlx::{SqlitePool, Transaction};

use crate::error::{AppError, AppResult};
use crate::models::{Post, PostEdit, PostView, Thread};
use crate::services::ThreadService;

const POST_VIEW_SELECT: &str = r#"
    SELECT
        p.id, p.thread_id, p.author_id, p.body,
        p.reply_to_post_id, p.deleted_at, p.edited_at,
        p.created_at, p.updated_at,
        u.username AS author_username,
        u.display_name AS author_display_name,
        (SELECT COUNT(*) FROM post_helpful h WHERE h.post_id = p.id) AS helpful_count
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
        include_deleted: bool,
    ) -> AppResult<Vec<PostView>> {
        let filter = if include_deleted {
            ""
        } else {
            "AND p.deleted_at IS NULL"
        };
        let sql = format!(
            "{POST_VIEW_SELECT}
            WHERE p.thread_id = ? {filter}
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

    pub async fn count_by_thread(
        db: &SqlitePool,
        thread_id: i64,
        include_deleted: bool,
    ) -> AppResult<i64> {
        let sql = if include_deleted {
            "SELECT COUNT(*) FROM posts WHERE thread_id = ?"
        } else {
            "SELECT COUNT(*) FROM posts WHERE thread_id = ? AND deleted_at IS NULL"
        };
        let count = sqlx::query_scalar::<_, i64>(sql)
            .bind(thread_id)
            .fetch_one(db)
            .await?;
        Ok(count)
    }

    pub async fn get_view(db: &SqlitePool, post_id: i64) -> AppResult<PostView> {
        ThreadService::get_post_view(db, post_id).await
    }

    pub async fn viewer_helpful(db: &SqlitePool, post_id: i64, user_id: i64) -> AppResult<bool> {
        let n = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM post_helpful WHERE post_id = ? AND user_id = ?",
        )
        .bind(post_id)
        .bind(user_id)
        .fetch_one(db)
        .await?;
        Ok(n > 0)
    }

    pub async fn add_helpful(db: &SqlitePool, post_id: i64, user_id: i64) -> AppResult<i64> {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO post_helpful (post_id, user_id)
            VALUES (?, ?)
            "#,
        )
        .bind(post_id)
        .bind(user_id)
        .execute(db)
        .await?;
        Self::helpful_count(db, post_id).await
    }

    pub async fn remove_helpful(db: &SqlitePool, post_id: i64, user_id: i64) -> AppResult<i64> {
        sqlx::query("DELETE FROM post_helpful WHERE post_id = ? AND user_id = ?")
            .bind(post_id)
            .bind(user_id)
            .execute(db)
            .await?;
        Self::helpful_count(db, post_id).await
    }

    pub async fn helpful_count(db: &SqlitePool, post_id: i64) -> AppResult<i64> {
        let n = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM post_helpful WHERE post_id = ?")
            .bind(post_id)
            .fetch_one(db)
            .await?;
        Ok(n)
    }

    pub async fn create_reply(
        db: &SqlitePool,
        thread_id: i64,
        author_id: i64,
        body: &str,
        reply_to_post_id: Option<i64>,
    ) -> AppResult<PostView> {
        let thread = ThreadService::get_by_id(db, thread_id).await?;
        if thread.is_locked {
            return Err(AppError::Forbidden);
        }

        if let Some(parent_id) = reply_to_post_id {
            let parent = Self::get_view(db, parent_id).await?;
            if parent.thread_id != thread_id {
                return Err(AppError::BadRequest(
                    "reply_to_post_id must belong to this thread".into(),
                ));
            }
        }

        let mut tx: Transaction<'_, sqlx::Sqlite> = db.begin().await?;

        let post = sqlx::query_as::<_, Post>(
            r#"
            INSERT INTO posts (thread_id, author_id, body, reply_to_post_id)
            VALUES (?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(thread_id)
        .bind(author_id)
        .bind(body)
        .bind(reply_to_post_id)
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

    pub async fn update_body(
        db: &SqlitePool,
        post_id: i64,
        editor_id: i64,
        body: &str,
    ) -> AppResult<PostView> {
        let post = sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE id = ?")
            .bind(post_id)
            .fetch_optional(db)
            .await?
            .ok_or(AppError::NotFound)?;

        if post.deleted_at.is_some() {
            return Err(AppError::BadRequest("cannot edit a deleted post".into()));
        }

        let mut tx: Transaction<'_, sqlx::Sqlite> = db.begin().await?;

        sqlx::query(
            r#"
            INSERT INTO post_edits (post_id, editor_id, body_before)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(post_id)
        .bind(editor_id)
        .bind(&post.body)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            UPDATE posts
            SET body = ?,
                edited_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            WHERE id = ?
            "#,
        )
        .bind(body)
        .bind(post_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        ThreadService::get_post_view(db, post_id).await
    }

    pub async fn soft_delete(db: &SqlitePool, post_id: i64) -> AppResult<()> {
        let post = sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE id = ?")
            .bind(post_id)
            .fetch_optional(db)
            .await?
            .ok_or(AppError::NotFound)?;

        if post.deleted_at.is_some() {
            return Ok(());
        }

        let mut tx: Transaction<'_, sqlx::Sqlite> = db.begin().await?;

        sqlx::query(
            r#"
            UPDATE posts
            SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                body = '',
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            WHERE id = ?
            "#,
        )
        .bind(post_id)
        .execute(&mut *tx)
        .await?;

        // Drop FTS row for deleted content.
        sqlx::query("DELETE FROM forum_fts WHERE post_id = ?")
            .bind(post_id)
            .execute(&mut *tx)
            .await
            .ok();

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

        sqlx::query(
            r#"
            UPDATE threads
            SET last_post_at = (
                SELECT MAX(created_at) FROM posts
                WHERE thread_id = threads.id AND deleted_at IS NULL
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

    pub async fn list_edits(db: &SqlitePool, post_id: i64) -> AppResult<Vec<PostEdit>> {
        let rows = sqlx::query_as::<_, PostEdit>(
            r#"
            SELECT
                e.id,
                e.post_id,
                e.editor_id,
                e.body_before,
                e.created_at,
                u.username AS editor_username,
                u.display_name AS editor_display_name
            FROM post_edits e
            INNER JOIN users u ON u.id = e.editor_id
            WHERE e.post_id = ?
            ORDER BY e.created_at DESC, e.id DESC
            "#,
        )
        .bind(post_id)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }
}
