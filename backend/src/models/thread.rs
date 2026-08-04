use serde::Serialize;
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Thread {
    pub id: i64,
    pub category_id: i64,
    pub author_id: i64,
    pub title: String,
    pub slug: String,
    pub is_pinned: bool,
    pub is_locked: bool,
    pub post_count: i64,
    pub view_count: i64,
    pub last_post_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ThreadView {
    pub id: i64,
    pub category_id: i64,
    pub author_id: i64,
    pub title: String,
    pub slug: String,
    pub is_pinned: bool,
    pub is_locked: bool,
    pub post_count: i64,
    pub view_count: i64,
    pub last_post_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub author_username: String,
    pub author_display_name: Option<String>,
    /// Absolute-path media URL (`/media/...`) when the author has an avatar.
    pub author_avatar_url: Option<String>,
    pub me_too_count: i64,
}
