use serde::Serialize;
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub sort_order: i64,
    pub parent_id: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
    /// Number of topics (threads) in this category. Defaults when not selected.
    #[sqlx(default)]
    pub topic_count: i64,
}
