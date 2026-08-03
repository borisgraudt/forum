use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::{ProfilePostItem, ProfileThreadItem, User, UserProfile, UserPublic, UserRole};

pub struct UserService;

impl UserService {
    pub async fn list(
        db: &SqlitePool,
        limit: i64,
        offset: i64,
    ) -> AppResult<(Vec<UserPublic>, i64)> {
        let total = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
            .fetch_one(db)
            .await?;
        let rows = sqlx::query_as::<_, User>(
            r#"
            SELECT * FROM users
            ORDER BY created_at DESC, id DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await?;
        Ok((rows.into_iter().map(User::into_public).collect(), total))
    }

    pub async fn find_by_username(db: &SqlitePool, username: &str) -> AppResult<User> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ? COLLATE NOCASE")
            .bind(username)
            .fetch_optional(db)
            .await?
            .ok_or(AppError::NotFound)
    }

    pub async fn find_by_id(db: &SqlitePool, id: i64) -> AppResult<User> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or(AppError::NotFound)
    }

    pub async fn profile(db: &SqlitePool, username: &str) -> AppResult<UserProfile> {
        let user = Self::find_by_username(db, username).await?;
        let post_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM posts WHERE author_id = ? AND deleted_at IS NULL",
        )
        .bind(user.id)
        .fetch_one(db)
        .await?;
        let thread_count =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM threads WHERE author_id = ?")
                .bind(user.id)
                .fetch_one(db)
                .await?;
        let helpful_received = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM post_helpful h
            INNER JOIN posts p ON p.id = h.post_id
            WHERE p.author_id = ?
            "#,
        )
        .bind(user.id)
        .fetch_one(db)
        .await?;
        // Simple reputation: helpful received * 10 + threads * 2 + posts.
        let reputation = helpful_received * 10 + thread_count * 2 + post_count;

        let recent_threads = sqlx::query_as::<_, ProfileThreadItem>(
            r#"
            SELECT
                t.id,
                t.title,
                t.slug,
                c.slug AS category_slug,
                c.name AS category_name,
                t.created_at,
                t.post_count
            FROM threads t
            INNER JOIN categories c ON c.id = t.category_id
            WHERE t.author_id = ?
            ORDER BY t.created_at DESC, t.id DESC
            LIMIT 8
            "#,
        )
        .bind(user.id)
        .fetch_all(db)
        .await?;

        let recent_posts = sqlx::query_as::<_, ProfilePostItem>(
            r#"
            SELECT
                p.id,
                p.thread_id,
                t.title AS thread_title,
                t.slug AS thread_slug,
                c.slug AS category_slug,
                CASE
                    WHEN length(p.body) > 160 THEN substr(p.body, 1, 160) || '…'
                    ELSE p.body
                END AS body_preview,
                p.created_at
            FROM posts p
            INNER JOIN threads t ON t.id = p.thread_id
            INNER JOIN categories c ON c.id = t.category_id
            WHERE p.author_id = ?
              AND p.deleted_at IS NULL
              AND p.id NOT IN (
                  SELECT MIN(p2.id) FROM posts p2
                  WHERE p2.thread_id = p.thread_id
              )
            ORDER BY p.created_at DESC, p.id DESC
            LIMIT 8
            "#,
        )
        .bind(user.id)
        .fetch_all(db)
        .await?;

        Ok(UserProfile {
            user: user.into_public(),
            post_count,
            thread_count,
            helpful_received,
            reputation,
            recent_threads,
            recent_posts,
        })
    }

    pub async fn update_profile(
        db: &SqlitePool,
        user_id: i64,
        display_name: Option<Option<String>>,
        bio: Option<Option<String>>,
    ) -> AppResult<UserPublic> {
        if display_name.is_none() && bio.is_none() {
            return Err(AppError::BadRequest(
                "display_name or bio is required".into(),
            ));
        }

        if let Some(dn) = display_name {
            sqlx::query(
                r#"
                UPDATE users
                SET display_name = ?,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                WHERE id = ?
                "#,
            )
            .bind(dn)
            .bind(user_id)
            .execute(db)
            .await?;
        }

        if let Some(b) = bio {
            sqlx::query(
                r#"
                UPDATE users
                SET bio = ?,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                WHERE id = ?
                "#,
            )
            .bind(b)
            .bind(user_id)
            .execute(db)
            .await?;
        }

        Ok(Self::find_by_id(db, user_id).await?.into_public())
    }

    pub async fn admin_update(
        db: &SqlitePool,
        user_id: i64,
        role: Option<&str>,
        is_active: Option<bool>,
    ) -> AppResult<UserPublic> {
        if role.is_none() && is_active.is_none() {
            return Err(AppError::BadRequest("role or is_active is required".into()));
        }

        if let Some(role) = role {
            let _ = role.parse::<UserRole>().map_err(AppError::BadRequest)?;
            sqlx::query(
                r#"
                UPDATE users
                SET role = ?,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                WHERE id = ?
                "#,
            )
            .bind(role)
            .bind(user_id)
            .execute(db)
            .await?;
        }

        if let Some(active) = is_active {
            sqlx::query(
                r#"
                UPDATE users
                SET is_active = ?,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                WHERE id = ?
                "#,
            )
            .bind(active)
            .bind(user_id)
            .execute(db)
            .await?;
        }

        Ok(Self::find_by_id(db, user_id).await?.into_public())
    }
}
