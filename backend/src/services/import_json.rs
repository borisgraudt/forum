//! JSON import for categories + threads (Discourse/NodeBB-ish dumps simplified).
//!
//! Format:
//! ```json
//! {
//!   "categories": [
//!     { "name": "General", "slug": "general", "description": "...", "children": [
//!       { "name": "Intro", "slug": "intro" }
//!     ]}
//!   ],
//!   "threads": [
//!     {
//!       "category_slug": "intro",
//!       "title": "Hello",
//!       "body": "Welcome!",
//!       "author": "admin"
//!     }
//!   ]
//! }
//! ```

use serde::Deserialize;
use sqlx::SqlitePool;
use std::path::Path;

use crate::error::{AppError, AppResult};
use crate::services::{AuthService, CategoryService, ThreadService};
use crate::utils::slugify;

#[derive(Debug, Deserialize)]
pub struct ImportFile {
    #[serde(default)]
    pub categories: Vec<ImportCategory>,
    #[serde(default)]
    pub threads: Vec<ImportThread>,
}

#[derive(Debug, Deserialize)]
pub struct ImportCategory {
    pub name: String,
    pub slug: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub children: Vec<ImportCategory>,
}

#[derive(Debug, Deserialize)]
pub struct ImportThread {
    pub category_slug: String,
    pub title: String,
    pub body: String,
    pub author: Option<String>,
}

pub struct ImportService;

impl ImportService {
    pub async fn from_path(db: &SqlitePool, path: &Path) -> AppResult<String> {
        let raw = std::fs::read_to_string(path)
            .map_err(|e| AppError::BadRequest(format!("read import file: {e}")))?;
        let data: ImportFile = serde_json::from_str(&raw)
            .map_err(|e| AppError::BadRequest(format!("invalid JSON: {e}")))?;
        Self::run(db, &data).await
    }

    pub async fn run(db: &SqlitePool, data: &ImportFile) -> AppResult<String> {
        let mut lines = Vec::new();
        let mut cat_count = 0i64;
        let mut thr_count = 0i64;

        for cat in &data.categories {
            cat_count += Self::upsert_category(db, cat, None).await?;
            for child in &cat.children {
                cat_count += Self::upsert_category(db, child, Some(&cat.slug_or_name())).await?;
            }
        }

        for thr in &data.threads {
            let category = CategoryService::get_by_slug(db, thr.category_slug.trim())
                .await
                .map_err(|_| {
                    AppError::BadRequest(format!("unknown category_slug '{}'", thr.category_slug))
                })?;
            let author_login = thr.author.as_deref().unwrap_or("admin");
            let author = AuthService::find_by_login(db, author_login)
                .await?
                .ok_or_else(|| {
                    AppError::BadRequest(format!("unknown author '{}'", author_login))
                })?;
            let slug = slugify(&thr.title);
            let exists: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM threads WHERE category_id = ? AND slug = ? COLLATE NOCASE",
            )
            .bind(category.id)
            .bind(&slug)
            .fetch_one(db)
            .await?;
            if exists > 0 {
                lines.push(format!("skip thread (exists): {slug}"));
                continue;
            }
            ThreadService::create_with_first_post(
                db,
                category.id,
                author.id,
                thr.title.trim(),
                &slug,
                thr.body.trim(),
            )
            .await?;
            thr_count += 1;
        }

        lines.push(format!(
            "import ok: categories_upserted≈{cat_count} threads_created={thr_count}"
        ));
        Ok(lines.join("\n"))
    }

    async fn upsert_category(
        db: &SqlitePool,
        cat: &ImportCategory,
        parent_slug: Option<&str>,
    ) -> AppResult<i64> {
        let slug = cat.slug_or_name();
        if CategoryService::get_by_slug(db, &slug).await.is_ok() {
            return Ok(0);
        }
        let parent_id = if let Some(ps) = parent_slug {
            Some(CategoryService::get_by_slug(db, ps).await?.id)
        } else {
            None
        };
        CategoryService::create(
            db,
            cat.name.trim(),
            &slug,
            cat.description.as_deref(),
            0,
            parent_id,
        )
        .await?;
        Ok(1)
    }
}

impl ImportCategory {
    fn slug_or_name(&self) -> String {
        self.slug
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| slugify(&self.name))
    }
}
