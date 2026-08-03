//! Fast link previews: Open Graph scrape with hard timeouts + SQLite cache.
//! No headless browser. Fail open. SSRF-hardened.

use std::time::Duration;

use regex::Regex;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};

const FETCH_TIMEOUT: Duration = Duration::from_secs(3);
const CACHE_OK_SECS: i64 = 7 * 24 * 3600;
const CACHE_FAIL_SECS: i64 = 3600;
const MAX_BODY: usize = 512 * 1024;

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct LinkEmbed {
    pub url_hash: String,
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub site_name: Option<String>,
    pub status: String,
    pub fetched_at: String,
    pub expires_at: String,
}

pub struct EmbedService;

impl EmbedService {
    pub fn url_hash(url: &str) -> String {
        let mut h = Sha256::new();
        h.update(url.trim().as_bytes());
        hex::encode(h.finalize())
    }

    /// Allow only http(s) to public hosts (block localhost / private IPs loosely).
    pub fn is_safe_url(url: &str) -> bool {
        let Ok(u) = url::Url::parse(url) else {
            return false;
        };
        if u.scheme() != "http" && u.scheme() != "https" {
            return false;
        }
        let Some(host) = u.host_str() else {
            return false;
        };
        let host_l = host.to_ascii_lowercase();
        if host_l == "localhost"
            || host_l.ends_with(".localhost")
            || host_l.ends_with(".local")
            || host_l == "0.0.0.0"
        {
            return false;
        }
        // Block obvious private/link-local literals
        if host_l.starts_with("127.")
            || host_l.starts_with("10.")
            || host_l.starts_with("192.168.")
            || host_l.starts_with("169.254.")
            || host_l == "::1"
            || host_l.starts_with("fc")
            || host_l.starts_with("fd")
            || host_l.starts_with("fe80")
        {
            return false;
        }
        // 172.16.0.0 – 172.31.255.255
        if let Some(rest) = host_l.strip_prefix("172.") {
            if let Some((a, _)) = rest.split_once('.') {
                if let Ok(n) = a.parse::<u8>() {
                    if (16..=31).contains(&n) {
                        return false;
                    }
                }
            }
        }
        true
    }

    pub async fn get_cached(db: &SqlitePool, url: &str) -> AppResult<Option<LinkEmbed>> {
        let hash = Self::url_hash(url);
        let row = sqlx::query_as::<_, LinkEmbed>(
            r#"
            SELECT * FROM link_embeds
            WHERE url_hash = ?
              AND expires_at > strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            "#,
        )
        .bind(&hash)
        .fetch_optional(db)
        .await?;
        Ok(row)
    }

    pub async fn get_or_fetch(db: &SqlitePool, url: &str) -> AppResult<LinkEmbed> {
        let url = url.trim();
        if !Self::is_safe_url(url) {
            return Err(AppError::BadRequest("url not allowed for preview".into()));
        }
        if let Some(c) = Self::get_cached(db, url).await? {
            return Ok(c);
        }
        Self::fetch_and_store(db, url).await
    }

    async fn fetch_and_store(db: &SqlitePool, url: &str) -> AppResult<LinkEmbed> {
        let hash = Self::url_hash(url);
        let client = reqwest::Client::builder()
            .timeout(FETCH_TIMEOUT)
            .connect_timeout(Duration::from_secs(2))
            .redirect(reqwest::redirect::Policy::limited(3))
            .user_agent("ForumBot/0.5 (+https://localhost; link-preview)")
            .build()
            .map_err(|e| AppError::Internal(anyhow::anyhow!("http client: {e}")))?;

        let result = client.get(url).send().await;
        let (status, title, description, image_url, site_name) = match result {
            Ok(resp) => {
                let ct = resp
                    .headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if !ct.contains("text/html") && !ct.contains("application/xhtml") {
                    ("failed", None, None, None, None)
                } else {
                    match resp.bytes().await {
                        Ok(bytes) => {
                            let slice = if bytes.len() > MAX_BODY {
                                &bytes[..MAX_BODY]
                            } else {
                                &bytes
                            };
                            let html = String::from_utf8_lossy(slice);
                            let meta = parse_og(&html);
                            (
                                "ok",
                                meta.title,
                                meta.description,
                                meta.image_url,
                                meta.site_name,
                            )
                        }
                        Err(_) => ("failed", None, None, None, None),
                    }
                }
            }
            Err(_) => ("failed", None, None, None, None),
        };

        let ttl = if status == "ok" {
            CACHE_OK_SECS
        } else {
            CACHE_FAIL_SECS
        };

        // SQLite datetime add via strftime modifiers
        sqlx::query(
            r#"
            INSERT INTO link_embeds (
                url_hash, url, title, description, image_url, site_name, status, expires_at
            ) VALUES (
                ?, ?, ?, ?, ?, ?, ?,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now', ?)
            )
            ON CONFLICT(url_hash) DO UPDATE SET
                title = excluded.title,
                description = excluded.description,
                image_url = excluded.image_url,
                site_name = excluded.site_name,
                status = excluded.status,
                fetched_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                expires_at = excluded.expires_at
            "#,
        )
        .bind(&hash)
        .bind(url)
        .bind(&title)
        .bind(&description)
        .bind(&image_url)
        .bind(&site_name)
        .bind(status)
        .bind(format!("+{ttl} seconds"))
        .execute(db)
        .await?;

        Self::get_cached(db, url)
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("embed missing after store")))
    }

    /// Extract http(s) URLs from markdown/plain body (first N unique).
    pub fn extract_urls(body: &str, limit: usize) -> Vec<String> {
        static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
        let re = RE.get_or_init(|| Regex::new(r#"https?://[^\s<>\[\]()"'`]+"#).expect("url re"));
        let mut out = Vec::new();
        for m in re.find_iter(body) {
            let u = m
                .as_str()
                .trim_end_matches(['.', ',', ';', ':', ')', ']'])
                .to_string();
            if Self::is_safe_url(&u) && !out.iter().any(|x| x == &u) {
                out.push(u);
            }
            if out.len() >= limit {
                break;
            }
        }
        out
    }
}

struct OgMeta {
    title: Option<String>,
    description: Option<String>,
    image_url: Option<String>,
    site_name: Option<String>,
}

fn parse_og(html: &str) -> OgMeta {
    fn meta_content(html: &str, key: &str) -> Option<String> {
        // property="og:title" content="..."
        let patterns = [
            format!(
                r#"(?is)<meta[^>]+property=["']{}["'][^>]+content=["']([^"']+)["']"#,
                key
            ),
            format!(
                r#"(?is)<meta[^>]+content=["']([^"']+)["'][^>]+property=["']{}["']"#,
                key
            ),
            format!(
                r#"(?is)<meta[^>]+name=["']{}["'][^>]+content=["']([^"']+)["']"#,
                key
            ),
            format!(
                r#"(?is)<meta[^>]+content=["']([^"']+)["'][^>]+name=["']{}["']"#,
                key
            ),
        ];
        for p in patterns {
            if let Ok(re) = Regex::new(&p) {
                if let Some(c) = re.captures(html) {
                    let v = c.get(1)?.as_str().trim();
                    if !v.is_empty() {
                        return Some(html_unescape(v));
                    }
                }
            }
        }
        None
    }

    let title = meta_content(html, "og:title")
        .or_else(|| meta_content(html, "twitter:title"))
        .or_else(|| {
            Regex::new(r"(?is)<title[^>]*>(.*?)</title>")
                .ok()
                .and_then(|re| {
                    re.captures(html).and_then(|c| {
                        let t = c.get(1)?.as_str();
                        let t = html_unescape(&strip_tags(t));
                        let t = t.trim();
                        if t.is_empty() {
                            None
                        } else {
                            Some(t.to_string())
                        }
                    })
                })
        });

    OgMeta {
        title,
        description: meta_content(html, "og:description")
            .or_else(|| meta_content(html, "description"))
            .or_else(|| meta_content(html, "twitter:description")),
        image_url: meta_content(html, "og:image").or_else(|| meta_content(html, "twitter:image")),
        site_name: meta_content(html, "og:site_name"),
    }
}

fn strip_tags(s: &str) -> String {
    let re = Regex::new(r"<[^>]+>").unwrap();
    re.replace_all(s, "").to_string()
}

fn html_unescape(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}
