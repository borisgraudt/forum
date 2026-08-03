use serde::Serialize;
use sqlx::FromRow;

use crate::utils::render_markdown;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Post {
    pub id: i64,
    pub thread_id: i64,
    pub author_id: i64,
    pub body: String,
    pub reply_to_post_id: Option<i64>,
    pub deleted_at: Option<String>,
    pub edited_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct PostView {
    pub id: i64,
    pub thread_id: i64,
    pub author_id: i64,
    pub body: String,
    pub reply_to_post_id: Option<i64>,
    pub deleted_at: Option<String>,
    pub edited_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub author_username: String,
    pub author_display_name: Option<String>,
    pub author_avatar_key: Option<String>,
    pub helpful_count: i64,
}

/// API projection with sanitized HTML body + vote flags.
#[derive(Debug, Clone, Serialize)]
pub struct PostViewJson {
    pub id: i64,
    pub thread_id: i64,
    pub author_id: i64,
    pub body: String,
    pub body_html: String,
    pub reply_to_post_id: Option<i64>,
    pub is_deleted: bool,
    pub edited_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub author_username: String,
    pub author_display_name: Option<String>,
    pub author_avatar_url: Option<String>,
    pub helpful_count: i64,
    pub viewer_marked_helpful: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<crate::models::AttachmentJson>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub embeds: Vec<crate::services::embed::LinkEmbed>,
}

impl PostViewJson {
    pub fn from_view(p: PostView, viewer_marked_helpful: bool) -> Self {
        let is_deleted = p.deleted_at.is_some();
        let body = if is_deleted {
            String::new()
        } else {
            p.body.clone()
        };
        let body_html = if is_deleted {
            String::new()
        } else {
            render_markdown(&p.body)
        };
        Self {
            id: p.id,
            thread_id: p.thread_id,
            author_id: p.author_id,
            body,
            body_html,
            reply_to_post_id: p.reply_to_post_id,
            is_deleted,
            edited_at: p.edited_at,
            created_at: p.created_at,
            updated_at: p.updated_at,
            author_username: p.author_username,
            author_display_name: p.author_display_name,
            author_avatar_url: p.author_avatar_key.as_ref().map(|k| format!("/media/{k}")),
            helpful_count: p.helpful_count,
            viewer_marked_helpful,
            attachments: Vec::new(),
            embeds: Vec::new(),
        }
    }
}

impl From<PostView> for PostViewJson {
    fn from(p: PostView) -> Self {
        Self::from_view(p, false)
    }
}

/// One row from `post_edits` (previous body snapshot).
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct PostEdit {
    pub id: i64,
    pub post_id: i64,
    pub editor_id: i64,
    pub body_before: String,
    pub created_at: String,
    pub editor_username: String,
    pub editor_display_name: Option<String>,
}
