use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::Category;

pub struct CategoryService;

impl CategoryService {
    /// Top-level communities (no parent).
    pub async fn list_roots(db: &SqlitePool) -> AppResult<Vec<Category>> {
        let rows = sqlx::query_as::<_, Category>(
            r#"
            SELECT id, name, slug, description, sort_order, parent_id, created_at, updated_at,
                   (SELECT COUNT(*) FROM threads t WHERE t.category_id = categories.id) AS topic_count
            FROM categories
            WHERE parent_id IS NULL
            ORDER BY created_at ASC, id ASC
            "#,
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    pub async fn list_children(db: &SqlitePool, parent_id: i64) -> AppResult<Vec<Category>> {
        let rows = sqlx::query_as::<_, Category>(
            r#"
            SELECT id, name, slug, description, sort_order, parent_id, created_at, updated_at,
                   (SELECT COUNT(*) FROM threads t WHERE t.category_id = categories.id) AS topic_count
            FROM categories
            WHERE parent_id = ?
            ORDER BY created_at ASC, id ASC
            "#,
        )
        .bind(parent_id)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    pub async fn get_by_slug(db: &SqlitePool, slug: &str) -> AppResult<Category> {
        sqlx::query_as::<_, Category>(
            r#"
            SELECT id, name, slug, description, sort_order, parent_id, created_at, updated_at,
                   (SELECT COUNT(*) FROM threads t WHERE t.category_id = categories.id) AS topic_count
            FROM categories
            WHERE slug = ? COLLATE NOCASE
            "#,
        )
        .bind(slug)
        .fetch_optional(db)
        .await?
        .ok_or(AppError::NotFound)
    }

    pub async fn get_by_id(db: &SqlitePool, id: i64) -> AppResult<Category> {
        sqlx::query_as::<_, Category>(
            r#"
            SELECT id, name, slug, description, sort_order, parent_id, created_at, updated_at,
                   (SELECT COUNT(*) FROM threads t WHERE t.category_id = categories.id) AS topic_count
            FROM categories
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(db)
        .await?
        .ok_or(AppError::NotFound)
    }

    pub async fn slug_exists(db: &SqlitePool, slug: &str) -> AppResult<bool> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM categories WHERE slug = ? COLLATE NOCASE")
                .bind(slug)
                .fetch_one(db)
                .await?;
        Ok(count > 0)
    }

    /// Ensure slug is unique: `base`, then `base-2`, `base-3`, …
    pub async fn unique_slug(db: &SqlitePool, base: &str) -> AppResult<String> {
        let base = if base.is_empty() { "category" } else { base };
        if !Self::slug_exists(db, base).await? {
            return Ok(base.to_string());
        }
        for n in 2..1000 {
            let candidate = format!("{base}-{n}");
            if !Self::slug_exists(db, &candidate).await? {
                return Ok(candidate);
            }
        }
        Err(AppError::Conflict(
            "could not allocate unique category slug".into(),
        ))
    }

    pub async fn create(
        db: &SqlitePool,
        name: &str,
        slug: &str,
        description: Option<&str>,
        sort_order: i64,
        parent_id: Option<i64>,
    ) -> AppResult<Category> {
        if let Some(pid) = parent_id {
            // Ensure parent exists and is a top-level community (one level of nesting).
            let parent = Self::get_by_id(db, pid).await?;
            if parent.parent_id.is_some() {
                return Err(AppError::BadRequest(
                    "cannot nest under a subcategory".into(),
                ));
            }
        }

        let slug = Self::unique_slug(db, slug).await?;

        let result = sqlx::query_as::<_, Category>(
            r#"
            INSERT INTO categories (name, slug, description, sort_order, parent_id)
            VALUES (?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(name)
        .bind(&slug)
        .bind(description)
        .bind(sort_order)
        .bind(parent_id)
        .fetch_one(db)
        .await;

        match result {
            Ok(row) => Ok(row),
            Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
                Err(AppError::Conflict("category slug already exists".into()))
            }
            Err(err) => Err(AppError::Sqlx(err)),
        }
    }

    /// Flat list for admin: roots first, then children grouped by parent.
    pub async fn list_all(db: &SqlitePool) -> AppResult<Vec<Category>> {
        let rows = sqlx::query_as::<_, Category>(
            r#"
            SELECT id, name, slug, description, sort_order, parent_id, created_at, updated_at,
                   (SELECT COUNT(*) FROM threads t WHERE t.category_id = categories.id) AS topic_count
            FROM categories
            ORDER BY
                COALESCE(parent_id, id),
                CASE WHEN parent_id IS NULL THEN 0 ELSE 1 END,
                sort_order ASC,
                name ASC,
                id ASC
            "#,
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    pub async fn update(
        db: &SqlitePool,
        id: i64,
        name: Option<&str>,
        description: Option<Option<&str>>,
        sort_order: Option<i64>,
    ) -> AppResult<Category> {
        let _ = Self::get_by_id(db, id).await?;
        if name.is_none() && description.is_none() && sort_order.is_none() {
            return Err(AppError::BadRequest(
                "name, description, or sort_order is required".into(),
            ));
        }

        if let Some(n) = name {
            let n = n.trim();
            if n.is_empty() {
                return Err(AppError::BadRequest("name is required".into()));
            }
            sqlx::query(
                r#"
                UPDATE categories
                SET name = ?,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                WHERE id = ?
                "#,
            )
            .bind(n)
            .bind(id)
            .execute(db)
            .await?;
        }

        if let Some(desc) = description {
            sqlx::query(
                r#"
                UPDATE categories
                SET description = ?,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                WHERE id = ?
                "#,
            )
            .bind(desc)
            .bind(id)
            .execute(db)
            .await?;
        }

        if let Some(order) = sort_order {
            sqlx::query(
                r#"
                UPDATE categories
                SET sort_order = ?,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                WHERE id = ?
                "#,
            )
            .bind(order)
            .bind(id)
            .execute(db)
            .await?;
        }

        Self::get_by_id(db, id).await
    }

    /// Delete only when empty (no children, no threads).
    pub async fn delete_if_empty(db: &SqlitePool, id: i64) -> AppResult<()> {
        let _ = Self::get_by_id(db, id).await?;
        let child_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM categories WHERE parent_id = ?")
                .bind(id)
                .fetch_one(db)
                .await?;
        if child_count > 0 {
            return Err(AppError::BadRequest(
                "category has subcategories; delete them first".into(),
            ));
        }
        let thread_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM threads WHERE category_id = ?")
                .bind(id)
                .fetch_one(db)
                .await?;
        if thread_count > 0 {
            return Err(AppError::BadRequest(
                "category has threads; move or delete them first".into(),
            ));
        }
        sqlx::query("DELETE FROM categories WHERE id = ?")
            .bind(id)
            .execute(db)
            .await?;
        Ok(())
    }
}
