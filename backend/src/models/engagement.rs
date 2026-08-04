use serde::Serialize;
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Notification {
    pub id: i64,
    pub user_id: i64,
    pub kind: String,
    pub actor_id: Option<i64>,
    pub thread_id: Option<i64>,
    pub post_id: Option<i64>,
    pub category_id: Option<i64>,
    pub body: String,
    pub is_read: bool,
    pub created_at: String,
}
