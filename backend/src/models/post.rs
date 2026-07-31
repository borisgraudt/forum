use serde::Serialize;
use sqlx::FromRow;

use crate::utils::render_markdown;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Post {
    pub id: i64,
    pub thread_id: i64,
    pub author_id: i64,
    pub body: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct PostView {
    pub id: i64,
    pub thread_id: i64,
    pub author_id: i64,
    pub body: String,
    pub created_at: String,
    pub updated_at: String,
    pub author_username: String,
    pub author_display_name: Option<String>,
}

/// API projection with sanitized HTML body.
#[derive(Debug, Clone, Serialize)]
pub struct PostViewJson {
    pub id: i64,
    pub thread_id: i64,
    pub author_id: i64,
    pub body: String,
    pub body_html: String,
    pub created_at: String,
    pub updated_at: String,
    pub author_username: String,
    pub author_display_name: Option<String>,
}

impl From<PostView> for PostViewJson {
    fn from(p: PostView) -> Self {
        let body_html = render_markdown(&p.body);
        Self {
            id: p.id,
            thread_id: p.thread_id,
            author_id: p.author_id,
            body: p.body,
            body_html,
            created_at: p.created_at,
            updated_at: p.updated_at,
            author_username: p.author_username,
            author_display_name: p.author_display_name,
        }
    }
}
