use sqlx::{SqlitePool, Transaction};

use crate::error::{AppError, AppResult};
use crate::models::{Post, PostView, Thread, ThreadView};

const THREAD_VIEW_SELECT: &str = r#"
    SELECT
        t.id, t.category_id, t.author_id, t.title, t.slug,
        t.is_pinned, t.is_locked, t.post_count, t.view_count, t.last_post_at,
        t.created_at, t.updated_at,
        u.username AS author_username,
        u.display_name AS author_display_name,
        (SELECT COUNT(*) FROM thread_me_too m WHERE m.thread_id = t.id) AS me_too_count
    FROM threads t
    INNER JOIN users u ON u.id = t.author_id
"#;

const POST_VIEW_SELECT: &str = r#"
    SELECT
        p.id, p.thread_id, p.author_id, p.body,
        p.reply_to_post_id, p.deleted_at, p.edited_at,
        p.created_at, p.updated_at,
        u.username AS author_username,
        u.display_name AS author_display_name,
        u.avatar_key AS author_avatar_key,
        (SELECT COUNT(*) FROM post_helpful h WHERE h.post_id = p.id) AS helpful_count
    FROM posts p
    INNER JOIN users u ON u.id = p.author_id
"#;

pub struct ThreadService;

impl ThreadService {
    pub async fn list_by_category(
        db: &SqlitePool,
        category_id: i64,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<ThreadView>> {
        let sql = format!(
            "{THREAD_VIEW_SELECT}
            WHERE t.category_id = ?
            ORDER BY t.is_pinned DESC, COALESCE(t.last_post_at, t.created_at) DESC
            LIMIT ? OFFSET ?"
        );
        let rows = sqlx::query_as::<_, ThreadView>(&sql)
            .bind(category_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(db)
            .await?;
        Ok(rows)
    }

    pub async fn count_by_category(db: &SqlitePool, category_id: i64) -> AppResult<i64> {
        let count =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM threads WHERE category_id = ?")
                .bind(category_id)
                .fetch_one(db)
                .await?;
        Ok(count)
    }

    pub async fn get_by_id(db: &SqlitePool, id: i64) -> AppResult<Thread> {
        sqlx::query_as::<_, Thread>("SELECT * FROM threads WHERE id = ?")
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or(AppError::NotFound)
    }

    pub async fn get_view_by_id(db: &SqlitePool, id: i64) -> AppResult<ThreadView> {
        let sql = format!("{THREAD_VIEW_SELECT} WHERE t.id = ?");
        sqlx::query_as::<_, ThreadView>(&sql)
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or(AppError::NotFound)
    }

    pub async fn get_by_category_and_slug(
        db: &SqlitePool,
        category_id: i64,
        slug: &str,
    ) -> AppResult<ThreadView> {
        let sql = format!(
            "{THREAD_VIEW_SELECT}
            WHERE t.category_id = ? AND t.slug = ? COLLATE NOCASE"
        );
        sqlx::query_as::<_, ThreadView>(&sql)
            .bind(category_id)
            .bind(slug)
            .fetch_optional(db)
            .await?
            .ok_or(AppError::NotFound)
    }

    pub async fn increment_views(db: &SqlitePool, thread_id: i64) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE threads
            SET view_count = view_count + 1
            WHERE id = ?
            "#,
        )
        .bind(thread_id)
        .execute(db)
        .await?;
        Ok(())
    }

    pub async fn set_flags(
        db: &SqlitePool,
        thread_id: i64,
        is_locked: Option<bool>,
        is_pinned: Option<bool>,
    ) -> AppResult<ThreadView> {
        if is_locked.is_none() && is_pinned.is_none() {
            return Err(AppError::BadRequest(
                "is_locked or is_pinned is required".into(),
            ));
        }

        if let Some(locked) = is_locked {
            sqlx::query(
                r#"
                UPDATE threads
                SET is_locked = ?,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                WHERE id = ?
                "#,
            )
            .bind(locked)
            .bind(thread_id)
            .execute(db)
            .await?;
        }

        if let Some(pinned) = is_pinned {
            sqlx::query(
                r#"
                UPDATE threads
                SET is_pinned = ?,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                WHERE id = ?
                "#,
            )
            .bind(pinned)
            .bind(thread_id)
            .execute(db)
            .await?;
        }

        Self::get_view_by_id(db, thread_id).await
    }

    pub async fn get_post_view(db: &SqlitePool, post_id: i64) -> AppResult<PostView> {
        let sql = format!("{POST_VIEW_SELECT} WHERE p.id = ?");
        sqlx::query_as::<_, PostView>(&sql)
            .bind(post_id)
            .fetch_optional(db)
            .await?
            .ok_or(AppError::NotFound)
    }

    pub async fn me_too_count(db: &SqlitePool, thread_id: i64) -> AppResult<i64> {
        let n =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM thread_me_too WHERE thread_id = ?")
                .bind(thread_id)
                .fetch_one(db)
                .await?;
        Ok(n)
    }

    pub async fn viewer_me_too(db: &SqlitePool, thread_id: i64, user_id: i64) -> AppResult<bool> {
        let n = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM thread_me_too WHERE thread_id = ? AND user_id = ?",
        )
        .bind(thread_id)
        .bind(user_id)
        .fetch_one(db)
        .await?;
        Ok(n > 0)
    }

    pub async fn add_me_too(db: &SqlitePool, thread_id: i64, user_id: i64) -> AppResult<i64> {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO thread_me_too (thread_id, user_id)
            VALUES (?, ?)
            "#,
        )
        .bind(thread_id)
        .bind(user_id)
        .execute(db)
        .await?;
        Self::me_too_count(db, thread_id).await
    }

    pub async fn remove_me_too(db: &SqlitePool, thread_id: i64, user_id: i64) -> AppResult<i64> {
        sqlx::query("DELETE FROM thread_me_too WHERE thread_id = ? AND user_id = ?")
            .bind(thread_id)
            .bind(user_id)
            .execute(db)
            .await?;
        Self::me_too_count(db, thread_id).await
    }

    pub async fn create_with_first_post(
        db: &SqlitePool,
        category_id: i64,
        author_id: i64,
        title: &str,
        slug: &str,
        body: &str,
    ) -> AppResult<(ThreadView, PostView)> {
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

        sqlx::query(
            r#"
            UPDATE threads
            SET post_count = 1,
                last_post_at = ?,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            WHERE id = ?
            "#,
        )
        .bind(&post.created_at)
        .bind(thread.id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        let thread = Self::get_view_by_id(db, thread.id).await?;
        let first_post = Self::get_post_view(db, post.id).await?;
        Ok((thread, first_post))
    }
}
