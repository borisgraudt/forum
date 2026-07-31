use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::Category;

pub struct CategoryService;

impl CategoryService {
    /// Top-level communities (no parent).
    pub async fn list_roots(db: &SqlitePool) -> AppResult<Vec<Category>> {
        let rows = sqlx::query_as::<_, Category>(
            r#"
            SELECT id, name, slug, description, sort_order, parent_id, created_at, updated_at
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
            SELECT id, name, slug, description, sort_order, parent_id, created_at, updated_at
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
            SELECT id, name, slug, description, sort_order, parent_id, created_at, updated_at
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
            SELECT id, name, slug, description, sort_order, parent_id, created_at, updated_at
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
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM categories WHERE slug = ? COLLATE NOCASE",
        )
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
        Err(AppError::Conflict("could not allocate unique category slug".into()))
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
            // Ensure parent exists.
            let _ = Self::get_by_id(db, pid).await?;
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
}
