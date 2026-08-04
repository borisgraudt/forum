use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::Attachment;
use crate::services::storage::{StorageService, StoredFile};
use std::path::Path;

pub struct MediaService;

impl MediaService {
    pub async fn insert(
        db: &SqlitePool,
        stored: &StoredFile,
        uploader_id: i64,
        kind: &str,
        original_name: Option<&str>,
    ) -> AppResult<Attachment> {
        // Dedup: return existing row with same storage_key if present.
        if let Some(existing) =
            sqlx::query_as::<_, Attachment>("SELECT * FROM attachments WHERE storage_key = ?")
                .bind(&stored.storage_key)
                .fetch_optional(db)
                .await?
        {
            return Ok(existing);
        }

        let row = sqlx::query_as::<_, Attachment>(
            r#"
            INSERT INTO attachments (
                storage_key, thumb_key, original_name, mime, size_bytes,
                width, height, uploader_id, kind
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(&stored.storage_key)
        .bind(&stored.thumb_key)
        .bind(original_name)
        .bind(&stored.mime)
        .bind(stored.size_bytes)
        .bind(stored.width)
        .bind(stored.height)
        .bind(uploader_id)
        .bind(kind)
        .fetch_one(db)
        .await?;
        Ok(row)
    }

    pub async fn get(db: &SqlitePool, id: i64) -> AppResult<Attachment> {
        sqlx::query_as::<_, Attachment>("SELECT * FROM attachments WHERE id = ?")
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or(AppError::NotFound)
    }

    pub async fn link_to_post(
        db: &SqlitePool,
        post_id: i64,
        attachment_ids: &[i64],
        uploader_id: i64,
    ) -> AppResult<()> {
        for (i, aid) in attachment_ids.iter().enumerate() {
            let att = Self::get(db, *aid).await?;
            if att.uploader_id != uploader_id {
                return Err(AppError::Forbidden);
            }
            sqlx::query(
                r#"
                INSERT OR IGNORE INTO post_attachments (post_id, attachment_id, sort_order)
                VALUES (?, ?, ?)
                "#,
            )
            .bind(post_id)
            .bind(aid)
            .bind(i as i64)
            .execute(db)
            .await?;
        }
        Ok(())
    }

    pub async fn list_for_post(db: &SqlitePool, post_id: i64) -> AppResult<Vec<Attachment>> {
        let rows = sqlx::query_as::<_, Attachment>(
            r#"
            SELECT a.*
            FROM attachments a
            INNER JOIN post_attachments pa ON pa.attachment_id = a.id
            WHERE pa.post_id = ?
            ORDER BY pa.sort_order ASC, a.id ASC
            "#,
        )
        .bind(post_id)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    pub async fn set_avatar(
        db: &SqlitePool,
        data_dir: &Path,
        user_id: i64,
        bytes: &[u8],
        declared_mime: Option<&str>,
    ) -> AppResult<Attachment> {
        let stored =
            StorageService::store_image(data_dir, bytes, declared_mime, false, true).await?;
        // Prefer avatar webp key for display if present.
        let display_key = stored
            .thumb_key
            .clone()
            .unwrap_or_else(|| stored.storage_key.clone());
        let att = Self::insert(db, &stored, user_id, "avatar", Some("avatar")).await?;
        sqlx::query(
            r#"
            UPDATE users
            SET avatar_key = ?,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            WHERE id = ?
            "#,
        )
        .bind(&display_key)
        .bind(user_id)
        .execute(db)
        .await?;
        Ok(att)
    }
}
