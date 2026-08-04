use sqlx::SqlitePool;

use crate::dto::SearchHit;
use crate::error::AppResult;
use crate::services::ThreadService;

pub struct SearchService;

#[derive(Debug, Default, Clone)]
pub struct SearchFilters {
    pub author: Option<String>,
    pub category: Option<String>,
    pub since: Option<String>,
}

impl SearchService {
    /// Escape FTS5 user query: wrap tokens in quotes to avoid syntax errors.
    pub fn sanitize_fts_query(raw: &str) -> String {
        raw.split_whitespace()
            .map(|t| {
                let cleaned: String = t
                    .chars()
                    .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == '*')
                    .collect();
                cleaned
            })
            .filter(|t| !t.is_empty() && t != "*")
            .map(|t| format!("\"{t}\""))
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub async fn search(
        db: &SqlitePool,
        q: &str,
        limit: i64,
        offset: i64,
        filters: &SearchFilters,
    ) -> AppResult<(Vec<SearchHit>, i64)> {
        let fts_q = Self::sanitize_fts_query(q);
        if fts_q.is_empty() {
            return Ok((vec![], 0));
        }

        let author = filters
            .author
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let category = filters
            .category
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let since = filters
            .since
            .as_deref()
            .map(str::trim)
            .filter(|s| s.len() >= 10)
            .map(|s| s[..10].to_string());

        // Build filter SQL fragments (bound params only — no string concat of user SQL).
        let mut sql = String::from(
            r#"
            SELECT
                f.thread_id AS thread_id,
                f.post_id AS post_id,
                f.body AS snippet,
                c.slug AS category_slug,
                c.name AS category_name
            FROM forum_fts f
            INNER JOIN threads t ON t.id = f.thread_id
            INNER JOIN categories c ON c.id = t.category_id
            INNER JOIN users u ON u.id = t.author_id
            WHERE forum_fts MATCH ?
            "#,
        );
        if author.is_some() {
            sql.push_str(" AND u.username = ? COLLATE NOCASE ");
        }
        if category.is_some() {
            sql.push_str(" AND c.slug = ? COLLATE NOCASE ");
        }
        if since.is_some() {
            sql.push_str(" AND COALESCE(t.last_post_at, t.created_at) >= ? ");
        }
        sql.push_str(" ORDER BY rank LIMIT 200 ");

        let mut qy = sqlx::query_as::<_, FtsRow>(&sql).bind(&fts_q);
        if let Some(ref a) = author {
            qy = qy.bind(a);
        }
        if let Some(ref c) = category {
            qy = qy.bind(c);
        }
        if let Some(ref s) = since {
            // Compare ISO prefixes lexicographically (stored as ISO8601 UTC).
            qy = qy.bind(format!("{s}T00:00:00"));
        }

        let rows = qy.fetch_all(db).await.unwrap_or_default();

        // Dedupe by thread_id, keep first (best rank) match.
        let mut seen = std::collections::HashSet::new();
        let mut unique: Vec<FtsRow> = Vec::new();
        for row in rows {
            if seen.insert(row.thread_id) {
                unique.push(row);
            }
        }

        let total = unique.len() as i64;
        let page: Vec<FtsRow> = unique
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect();

        let mut results = Vec::with_capacity(page.len());
        for row in page {
            let thread = match ThreadService::get_view_by_id(db, row.thread_id).await {
                Ok(t) => t,
                Err(_) => continue,
            };
            results.push(SearchHit {
                thread,
                category_slug: row.category_slug,
                category_name: row.category_name,
                post_id: row.post_id,
                snippet: truncate_snippet(&row.snippet, 180),
            });
        }

        Ok((results, total))
    }
}

#[derive(Debug, sqlx::FromRow)]
struct FtsRow {
    thread_id: i64,
    post_id: i64,
    snippet: String,
    category_slug: String,
    category_name: String,
}

fn truncate_snippet(s: &str, max: usize) -> String {
    let t = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.chars().count() <= max {
        return t;
    }
    let mut out = String::new();
    for (i, ch) in t.chars().enumerate() {
        if i >= max {
            break;
        }
        out.push(ch);
    }
    out.push('…');
    out
}
