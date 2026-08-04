//! Ultra-light content-addressed upload store.
//!
//! Layout: `{data_dir}/uploads/{hash[0..2]}/{hash}.{ext}`
//! Thumbnails: `{data_dir}/uploads/{hash[0..2]}/{hash}_t.webp`
//!
//! Dedup by SHA-256 of raw bytes. Immutable files → aggressive HTTP cache.

use std::path::{Path, PathBuf};

use anyhow::anyhow;
use image::imageops::FilterType;
use image::ImageFormat;
use sha2::{Digest, Sha256};
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::error::{AppError, AppResult};

pub const MAX_AVATAR_BYTES: u64 = 2 * 1024 * 1024; // 2 MiB
pub const MAX_POST_IMAGE_BYTES: u64 = 8 * 1024 * 1024; // 8 MiB
pub const AVATAR_MAX_EDGE: u32 = 256;
pub const THUMB_MAX_EDGE: u32 = 640;

const ALLOWED_IMAGE: &[&str] = &["image/jpeg", "image/png", "image/gif", "image/webp"];

#[derive(Debug, Clone)]
pub struct StoredFile {
    pub storage_key: String,
    pub thumb_key: Option<String>,
    pub mime: String,
    pub size_bytes: i64,
    pub width: Option<i64>,
    pub height: Option<i64>,
}

pub struct StorageService;

impl StorageService {
    pub fn uploads_dir(data_dir: &Path) -> PathBuf {
        data_dir.join("uploads")
    }

    pub async fn ensure_dirs(data_dir: &Path) -> AppResult<()> {
        fs::create_dir_all(Self::uploads_dir(data_dir))
            .await
            .map_err(|e| AppError::Internal(anyhow!("create uploads dir: {e}")))?;
        Ok(())
    }

    pub fn is_allowed_image(mime: &str) -> bool {
        ALLOWED_IMAGE.contains(&mime)
    }

    pub fn detect_mime(bytes: &[u8], declared: Option<&str>) -> Option<&'static str> {
        let sniffed = match bytes {
            [0xFF, 0xD8, ..] => Some("image/jpeg"),
            [0x89, b'P', b'N', b'G', ..] => Some("image/png"),
            [b'G', b'I', b'F', b'8', ..] => Some("image/gif"),
            [b'R', b'I', b'F', b'F', ..] if bytes.len() > 12 && &bytes[8..12] == b"WEBP" => {
                Some("image/webp")
            }
            _ => None,
        };
        if let Some(m) = sniffed {
            return Some(m);
        }
        declared.and_then(|d| {
            let d = d.split(';').next().unwrap_or(d).trim();
            if Self::is_allowed_image(d) {
                Some(match d {
                    "image/jpeg" => "image/jpeg",
                    "image/png" => "image/png",
                    "image/gif" => "image/gif",
                    "image/webp" => "image/webp",
                    _ => return None,
                })
            } else {
                None
            }
        })
    }

    fn ext_for_mime(mime: &str) -> &'static str {
        match mime {
            "image/jpeg" => "jpg",
            "image/png" => "png",
            "image/gif" => "gif",
            "image/webp" => "webp",
            _ => "bin",
        }
    }

    pub fn hash_bytes(bytes: &[u8]) -> String {
        let mut h = Sha256::new();
        h.update(bytes);
        hex::encode(h.finalize())
    }

    fn path_for_key(data_dir: &Path, key: &str) -> PathBuf {
        Self::uploads_dir(data_dir).join(key)
    }

    /// Store image bytes; optionally produce WebP thumbnail.
    pub async fn store_image(
        data_dir: &Path,
        bytes: &[u8],
        declared_mime: Option<&str>,
        make_thumb: bool,
        avatar: bool,
    ) -> AppResult<StoredFile> {
        let mime = Self::detect_mime(bytes, declared_mime).ok_or_else(|| {
            AppError::BadRequest("only jpeg/png/gif/webp images are allowed".into())
        })?;

        let max = if avatar {
            MAX_AVATAR_BYTES
        } else {
            MAX_POST_IMAGE_BYTES
        };
        if bytes.len() as u64 > max {
            return Err(AppError::BadRequest(format!(
                "file too large (max {} bytes)",
                max
            )));
        }

        let hash = Self::hash_bytes(bytes);
        let ext = Self::ext_for_mime(mime);
        let prefix = &hash[..2];
        let storage_key = format!("{prefix}/{hash}.{ext}");

        let dir = Self::uploads_dir(data_dir).join(prefix);
        fs::create_dir_all(&dir)
            .await
            .map_err(|e| AppError::Internal(anyhow!("mkdir: {e}")))?;

        let full = Self::path_for_key(data_dir, &storage_key);
        if !full.exists() {
            let mut f = fs::File::create(&full)
                .await
                .map_err(|e| AppError::Internal(anyhow!("create file: {e}")))?;
            f.write_all(bytes)
                .await
                .map_err(|e| AppError::Internal(anyhow!("write file: {e}")))?;
        }

        let (width, height, thumb_key) = {
            let bytes = bytes.to_vec();
            let data_dir = data_dir.to_path_buf();
            let hash = hash.clone();
            let prefix = prefix.to_string();
            tokio::task::spawn_blocking(move || {
                process_image(&data_dir, &bytes, &hash, &prefix, make_thumb, avatar)
            })
            .await
            .map_err(|e| AppError::Internal(anyhow!("image task: {e}")))?
            .map_err(|e| AppError::BadRequest(format!("invalid image: {e}")))?
        };

        Ok(StoredFile {
            storage_key,
            thumb_key,
            mime: mime.to_string(),
            size_bytes: bytes.len() as i64,
            width,
            height,
        })
    }

    pub fn resolve_path(data_dir: &Path, key: &str) -> AppResult<PathBuf> {
        // Prevent path traversal: only hex/slash/underscore/dot
        if key.contains("..")
            || key.starts_with('/')
            || !key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '/' || c == '.' || c == '_' || c == '-')
        {
            return Err(AppError::NotFound);
        }
        let p = Self::path_for_key(data_dir, key);
        if !p.is_file() {
            return Err(AppError::NotFound);
        }
        Ok(p)
    }
}

type ImageMeta = (Option<i64>, Option<i64>, Option<String>);

fn process_image(
    data_dir: &Path,
    bytes: &[u8],
    hash: &str,
    prefix: &str,
    make_thumb: bool,
    avatar: bool,
) -> Result<ImageMeta, String> {
    let img = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
    let (w, h) = (img.width(), img.height());

    // JPEG thumbs: widely supported encode path, small, fast decode on clients.
    let thumb_key = if avatar {
        let edge = AVATAR_MAX_EDGE;
        let resized = img.resize_to_fill(edge, edge, FilterType::Triangle);
        let key = format!("{prefix}/{hash}_a.jpg");
        let path = StorageService::uploads_dir(data_dir).join(&key);
        if !path.exists() {
            resized
                .save_with_format(&path, ImageFormat::Jpeg)
                .map_err(|e| e.to_string())?;
        }
        Some(key)
    } else if make_thumb && (w > THUMB_MAX_EDGE || h > THUMB_MAX_EDGE) {
        let resized = img.resize(THUMB_MAX_EDGE, THUMB_MAX_EDGE, FilterType::Triangle);
        let key = format!("{prefix}/{hash}_t.jpg");
        let path = StorageService::uploads_dir(data_dir).join(&key);
        if !path.exists() {
            resized
                .save_with_format(&path, ImageFormat::Jpeg)
                .map_err(|e| e.to_string())?;
        }
        Some(key)
    } else {
        None
    };

    Ok((Some(w as i64), Some(h as i64), thumb_key))
}
