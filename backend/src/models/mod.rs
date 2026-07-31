//! Domain models mapped to SQLite tables via SQLx.
//!
//! Handlers/services in follow-up PRs will consume these types; schema tests
//! already exercise them end-to-end against migrations.

mod category;
mod post;
mod thread;
mod user;

pub use category::Category;
pub use post::{Post, PostView};
pub use thread::{Thread, ThreadView};
pub use user::{User, UserPublic, UserRole};

// re-export for handlers that only need table rows


#[cfg(test)]
mod schema_tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn pool() -> SqlitePool {
        let pool = crate::db::connect("sqlite::memory:")
            .await
            .expect("connect");
        crate::db::migrate(&pool).await.expect("migrate");
        pool
    }

    #[tokio::test]
    async fn migrations_create_core_tables() {
        let pool = pool().await;
        for table in ["users", "categories", "threads", "posts"] {
            let exists: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?",
            )
            .bind(table)
            .fetch_one(&pool)
            .await
            .unwrap();
            assert_eq!(exists, 1, "expected table {table}");
        }
    }

    #[tokio::test]
    async fn can_insert_and_load_user_category_thread_post() {
        let pool = pool().await;

        let user_id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO users (username, email, password_hash, display_name, role)
            VALUES (?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind("alice")
        .bind("alice@example.com")
        .bind("$2b$dummyhash")
        .bind("Alice")
        .bind(UserRole::User.as_str())
        .fetch_one(&pool)
        .await
        .unwrap();

        let category_id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO categories (name, slug, description, sort_order)
            VALUES (?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind("General")
        .bind("general")
        .bind("General discussion")
        .bind(0_i64)
        .fetch_one(&pool)
        .await
        .unwrap();

        let thread_id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO threads (category_id, author_id, title, slug, post_count, last_post_at)
            VALUES (?, ?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(category_id)
        .bind(user_id)
        .bind("Hello world")
        .bind("hello-world")
        .bind(1_i64)
        .bind("2026-07-31T12:00:00.000Z")
        .fetch_one(&pool)
        .await
        .unwrap();

        let post_id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO posts (thread_id, author_id, body)
            VALUES (?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(thread_id)
        .bind(user_id)
        .bind("First post body")
        .fetch_one(&pool)
        .await
        .unwrap();

        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(user.username, "alice");
        assert_eq!(user.role_enum().unwrap(), UserRole::User);
        assert!(user.is_active);
        // Ensure password stays out of public projection / JSON.
        let public = user.into_public();
        let json = serde_json::to_string(&public).unwrap();
        assert!(!json.contains("password"));
        assert!(!json.contains("@"));

        let category = sqlx::query_as::<_, Category>("SELECT * FROM categories WHERE id = ?")
            .bind(category_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(category.slug, "general");

        let thread = sqlx::query_as::<_, Thread>("SELECT * FROM threads WHERE id = ?")
            .bind(thread_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(thread.title, "Hello world");
        assert!(!thread.is_locked);
        assert_eq!(thread.post_count, 1);

        let post = sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE id = ?")
            .bind(post_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(post.body, "First post body");
        assert_eq!(post.author_id, user_id);
    }

    #[tokio::test]
    async fn foreign_keys_reject_orphan_thread() {
        let pool = pool().await;

        let err = sqlx::query(
            r#"
            INSERT INTO threads (category_id, author_id, title, slug)
            VALUES (999, 999, 'orphan', 'orphan')
            "#,
        )
        .execute(&pool)
        .await
        .expect_err("FK should fail");

        let msg = err.to_string().to_lowercase();
        assert!(
            msg.contains("foreign key") || msg.contains("constraint"),
            "unexpected error: {msg}"
        );
    }

    #[tokio::test]
    async fn username_is_unique_case_insensitive() {
        let pool = pool().await;

        sqlx::query(
            r#"
            INSERT INTO users (username, email, password_hash)
            VALUES ('Bob', 'bob@example.com', 'hash1')
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        let err = sqlx::query(
            r#"
            INSERT INTO users (username, email, password_hash)
            VALUES ('bob', 'bob2@example.com', 'hash2')
            "#,
        )
        .execute(&pool)
        .await
        .expect_err("duplicate username");

        assert!(err.to_string().to_lowercase().contains("unique"));
    }
}
