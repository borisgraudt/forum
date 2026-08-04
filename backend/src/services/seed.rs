//! Demo data for local/dev and smoke environments.

use sqlx::SqlitePool;

use crate::error::AppResult;
use crate::services::{AuthService, CategoryService, ThreadService};
use crate::utils::slugify;

pub struct SeedService;

impl SeedService {
    /// Idempotent seed: admin user + sample community tree + one thread.
    /// Returns a short human summary.
    pub async fn run(db: &SqlitePool) -> AppResult<String> {
        let mut lines = Vec::new();

        // Admin: admin / password123 (change in production!)
        let admin = match AuthService::find_by_login(db, "admin").await? {
            Some(u) => {
                lines.push(format!("admin user exists (id={})", u.id));
                u
            }
            None => {
                let hash = AuthService::hash_password("password123".into()).await?;
                let u = AuthService::create_user(
                    db,
                    "admin",
                    "admin@example.com",
                    &hash,
                    Some("Admin"),
                )
                .await?;
                sqlx::query("UPDATE users SET role = 'admin' WHERE id = ?")
                    .bind(u.id)
                    .execute(db)
                    .await?;
                lines.push(format!(
                    "created admin user id={} (username=admin password=password123)",
                    u.id
                ));
                AuthService::find_by_login(db, "admin")
                    .await?
                    .expect("admin just created")
            }
        };

        let root = match CategoryService::get_by_slug(db, "general").await {
            Ok(c) => {
                lines.push(format!("community 'general' exists (id={})", c.id));
                c
            }
            Err(_) => {
                let c = CategoryService::create(
                    db,
                    "General",
                    "general",
                    Some("Default community for demos and tests."),
                    0,
                    None,
                )
                .await?;
                lines.push(format!("created community general id={}", c.id));
                c
            }
        };

        let sub = match CategoryService::get_by_slug(db, "introductions").await {
            Ok(c) => {
                lines.push(format!("subcategory 'introductions' exists (id={})", c.id));
                c
            }
            Err(_) => {
                let c = CategoryService::create(
                    db,
                    "Introductions",
                    "introductions",
                    Some("Say hello."),
                    0,
                    Some(root.id),
                )
                .await?;
                lines.push(format!("created subcategory introductions id={}", c.id));
                c
            }
        };

        let thread_slug = slugify("Welcome to the forum");
        let has_thread = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM threads WHERE category_id = ? AND slug = ? COLLATE NOCASE",
        )
        .bind(sub.id)
        .bind(&thread_slug)
        .fetch_one(db)
        .await?
            > 0;

        if has_thread {
            lines.push(format!("welcome thread already exists ({thread_slug})"));
        } else {
            let (thread, _) = ThreadService::create_with_first_post(
                db,
                sub.id,
                admin.id,
                "Welcome to the forum",
                &thread_slug,
                "This is a **seeded** topic. Edit or delete it anytime.\n\nTry Me too, Watch, and Helpful.",
            )
            .await?;
            // Auto-watch for OP is done async on create path; do it here too.
            let _ =
                crate::services::EngagementService::watch(db, admin.id, "thread", thread.id).await;
            lines.push(format!(
                "created welcome thread id={} slug={}",
                thread.id, thread.slug
            ));
        }

        Ok(lines.join("\n"))
    }
}
