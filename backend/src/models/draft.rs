use serde::Serialize;
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Draft {
    pub id: i64,
    pub user_id: i64,
    pub kind: String,
    pub category_slug: Option<String>,
    pub thread_id: Option<i64>,
    pub title: Option<String>,
    pub body: String,
    pub updated_at: String,
}
