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
            ORDER BY sort_order ASC, name ASC
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
            ORDER BY sort_order ASC, name ASC
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

        let result = sqlx::query_as::<_, Category>(
            r#"
            INSERT INTO categories (name, slug, description, sort_order, parent_id)
            VALUES (?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(name)
        .bind(slug)
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
