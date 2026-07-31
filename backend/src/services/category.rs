use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::Category;

pub struct CategoryService;

impl CategoryService {
    pub async fn list(db: &SqlitePool) -> AppResult<Vec<Category>> {
        let rows = sqlx::query_as::<_, Category>(
            r#"
            SELECT * FROM categories
            ORDER BY sort_order ASC, name ASC
            "#,
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    pub async fn get_by_slug(db: &SqlitePool, slug: &str) -> AppResult<Category> {
        sqlx::query_as::<_, Category>("SELECT * FROM categories WHERE slug = ? COLLATE NOCASE")
            .bind(slug)
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
    ) -> AppResult<Category> {
        let result = sqlx::query_as::<_, Category>(
            r#"
            INSERT INTO categories (name, slug, description, sort_order)
            VALUES (?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(name)
        .bind(slug)
        .bind(description)
        .bind(sort_order)
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
