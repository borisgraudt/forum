use serde::Serialize;
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Attachment {
    pub id: i64,
    pub storage_key: String,
    pub thumb_key: Option<String>,
    pub original_name: Option<String>,
    pub mime: String,
    pub size_bytes: i64,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub uploader_id: i64,
    pub kind: String,
    pub created_at: String,
}

impl Attachment {
    /// Public URL path (served by API).
    pub fn url_path(&self) -> String {
        format!("/media/{}", self.storage_key)
    }

    pub fn thumb_url_path(&self) -> Option<String> {
        self.thumb_key
            .as_ref()
            .map(|k| format!("/media/{k}"))
            .or_else(|| Some(self.url_path()))
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AttachmentJson {
    pub id: i64,
    pub url: String,
    pub thumb_url: Option<String>,
    pub mime: String,
    pub size_bytes: i64,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub original_name: Option<String>,
}

impl From<Attachment> for AttachmentJson {
    fn from(a: Attachment) -> Self {
        let thumb = a.thumb_url_path();
        Self {
            id: a.id,
            url: a.url_path(),
            thumb_url: thumb,
            mime: a.mime,
            size_bytes: a.size_bytes,
            width: a.width,
            height: a.height,
            original_name: a.original_name,
        }
    }
}
