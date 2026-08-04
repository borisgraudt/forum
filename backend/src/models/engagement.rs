use serde::Serialize;
use sqlx::FromRow;

/// Notification + path slugs for deep links in the inbox.
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct NotificationView {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_slug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_slug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_title: Option<String>,
}
