use sqlx::{SqlitePool, Transaction};

use crate::error::{AppError, AppResult};
use crate::models::{Post, PostView, Thread, ThreadView};

const THREAD_VIEW_SELECT: &str = r#"
    SELECT
        t.id, t.category_id, t.author_id, t.title, t.slug,
        t.is_pinned, t.is_locked, t.post_count, t.last_post_at,
        t.created_at, t.updated_at,
        u.username AS author_username,
        u.display_name AS author_display_name
    FROM threads t
    INNER JOIN users u ON u.id = t.author_id
"#;

const POST_VIEW_SELECT: &str = r#"
    SELECT
        p.id, p.thread_id, p.author_id, p.body, p.created_at, p.updated_at,
        u.username AS author_username,
        u.display_name AS author_display_name
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

    pub async fn get_post_view(db: &SqlitePool, post_id: i64) -> AppResult<PostView> {
        let sql = format!("{POST_VIEW_SELECT} WHERE p.id = ?");
        sqlx::query_as::<_, PostView>(&sql)
            .bind(post_id)
            .fetch_optional(db)
            .await?
            .ok_or(AppError::NotFound)
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
