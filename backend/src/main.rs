mod config;
mod db;
mod dto;
mod error;
mod handlers;
mod middleware;
mod models;
mod services;
mod state;
mod utils;

use axum::extract::State;
use axum::http::{header, HeaderValue, Method, StatusCode};
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::error::AppResult;
use crate::handlers::{
    admin_router, auth_router, categories_router, drafts_router, media_public_router, media_router,
    moderation_router, posts_router, search_router, threads_router, users_router,
};
use crate::middleware::{CsrfLayer, RateLimitLayer, SecurityHeadersLayer};
use crate::services::StorageService;
use crate::state::AppState;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_env()?;

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(&config.rust_log))
        .init();

    let pool = db::connect(&config.database_url).await?;
    db::migrate(&pool).await?;
    StorageService::ensure_dirs(&config.data_dir).await?;

    let addr = config.socket_addr()?;
    let state = AppState::new(config.clone(), pool);
    let app = build_router(state, &config.cors_origin)?;

    tracing::info!(%addr, "listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

fn build_router(state: AppState, cors_origin: &str) -> anyhow::Result<Router> {
    let origin = HeaderValue::from_str(cors_origin)
        .map_err(|e| anyhow::anyhow!("invalid CORS_ORIGIN: {e}"))?;

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::exact(origin))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::COOKIE,
            header::HeaderName::from_static("x-csrf-token"),
        ])
        .allow_credentials(true);

    // Layer order: outermost runs first on request.
    // security headers ← rate limit ← csrf ← trace ← routes
    Ok(Router::new()
        .route("/health", get(health))
        .route("/api/v1/health", get(health))
        // Immutable media: long-cache, no CSRF (GET only).
        .merge(media_public_router())
        .nest("/api/v1/auth", auth_router())
        .nest("/api/v1/categories", categories_router())
        .nest(
            "/api/v1",
            Router::new()
                .merge(threads_router())
                .merge(posts_router())
                .merge(search_router())
                .merge(users_router())
                .merge(drafts_router())
                .merge(media_router())
                .nest("/mod", moderation_router())
                .nest("/admin", admin_router()),
        )
        .layer(TraceLayer::new_for_http())
        .layer(CsrfLayer)
        .layer(RateLimitLayer::default())
        .layer(SecurityHeadersLayer)
        .layer(cors)
        .with_state(state))
}

async fn health(State(state): State<AppState>) -> AppResult<(StatusCode, Json<HealthResponse>)> {
    // Cheap liveness probe that also verifies the pool can serve a query.
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.db)
        .await?;

    Ok((
        StatusCode::OK,
        Json(HealthResponse {
            status: "ok",
            service: "forum-backend",
        }),
    ))
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("shutdown signal received");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    use axum::body::Body;
    use axum::http::{header, Request};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn test_state() -> AppState {
        let config = Config {
            host: "127.0.0.1".into(),
            port: 0,
            database_url: "sqlite::memory:".into(),
            jwt_secret: "test-secret-at-least-16".into(),
            jwt_ttl: Duration::from_secs(3600),
            cookie_secure: false,
            cors_origin: "http://localhost:4321".into(),
            rust_log: "error".into(),
            data_dir: std::env::temp_dir().join(format!("forum-test-{}", std::process::id())),
            public_origin: "http://localhost:4321".into(),
        };
        let pool = db::connect(&config.database_url)
            .await
            .expect("connect memory db");
        db::migrate(&pool).await.expect("migrate");
        AppState::new(config, pool)
    }

    fn json_body(value: serde_json::Value) -> Body {
        Body::from(serde_json::to_vec(&value).unwrap())
    }

    fn cookie_from(response: &axum::http::Response<Body>) -> Option<String> {
        response
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .find(|c| c.starts_with("session="))
            .map(str::to_string)
    }

    fn set_cookies(response: &axum::http::Response<Body>) -> Vec<String> {
        response
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .map(|c| c.split(';').next().unwrap_or(c).to_string())
            .collect()
    }

    /// Fetch a CSRF token + Cookie header value for mutating requests.
    async fn csrf_pair(app: &Router) -> (String, String) {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/csrf")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let cookies = set_cookies(&response);
        let csrf_cookie = cookies
            .iter()
            .find(|c| c.starts_with("csrf="))
            .cloned()
            .expect("csrf cookie");
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let token = json["csrf_token"].as_str().unwrap().to_string();
        (csrf_cookie, token)
    }

    fn merge_cookies(parts: &[&str]) -> String {
        parts.join("; ")
    }

    #[tokio::test]
    async fn health_returns_ok() {
        let state = test_state().await;
        let app = build_router(state, "http://localhost:4321").expect("router");

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
        assert_eq!(json["service"], "forum-backend");
    }

    #[tokio::test]
    async fn register_login_me_logout_flow() {
        let state = test_state().await;
        let app = build_router(state, "http://localhost:4321").expect("router");
        let (csrf_cookie, csrf_token) = csrf_pair(&app).await;

        // Register
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/register")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, &csrf_cookie)
                    .header("x-csrf-token", &csrf_token)
                    .body(json_body(serde_json::json!({
                        "username": "alice",
                        "email": "alice@example.com",
                        "password": "password123",
                        "display_name": "Alice"
                    })))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let session = cookie_from(&response).expect("session cookie on register");
        assert!(session.starts_with("session="));
        assert!(session.to_ascii_lowercase().contains("httponly"));

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["user"]["username"], "alice");
        assert!(json["user"].get("password_hash").is_none());
        assert!(json["user"].get("email").is_none());

        // /me with cookie
        let session_pair = session.split(';').next().unwrap();
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/me")
                    .header(header::COOKIE, session_pair)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Login (fresh CSRF)
        let (csrf_cookie, csrf_token) = csrf_pair(&app).await;
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, &csrf_cookie)
                    .header("x-csrf-token", &csrf_token)
                    .body(json_body(serde_json::json!({
                        "login": "alice@example.com",
                        "password": "password123"
                    })))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let session = cookie_from(&response).expect("session cookie on login");
        let cookies = set_cookies(&response);
        let csrf_after = cookies
            .iter()
            .find(|c| c.starts_with("csrf="))
            .cloned()
            .unwrap_or(csrf_cookie);
        let token_after = csrf_after.strip_prefix("csrf=").unwrap_or("").to_string();
        let cookie_header = merge_cookies(&[session.split(';').next().unwrap(), &csrf_after]);

        // Logout
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/logout")
                    .header(header::COOKIE, &cookie_header)
                    .header("x-csrf-token", &token_after)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let cleared = cookie_from(&response).expect("clear cookie");
        assert!(
            cleared.contains("Max-Age=0") || cleared.to_ascii_lowercase().contains("max-age=0")
        );

        // /me without valid cookie
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/me")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn login_rejects_bad_password() {
        let state = test_state().await;
        let app = build_router(state, "http://localhost:4321").expect("router");
        let _ = register_cookie(&app, "bob").await;

        let (csrf_cookie, csrf_token) = csrf_pair(&app).await;
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, &csrf_cookie)
                    .header("x-csrf-token", &csrf_token)
                    .body(json_body(serde_json::json!({
                        "login": "bob",
                        "password": "wrong-password"
                    })))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn register_rejects_duplicate_username() {
        let state = test_state().await;
        let app = build_router(state, "http://localhost:4321").expect("router");

        let payload = serde_json::json!({
            "username": "carol",
            "email": "carol@example.com",
            "password": "password123"
        });

        let (csrf_cookie, csrf_token) = csrf_pair(&app).await;
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/register")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, &csrf_cookie)
                    .header("x-csrf-token", &csrf_token)
                    .body(json_body(payload.clone()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        let (csrf_cookie, csrf_token) = csrf_pair(&app).await;
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/register")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, &csrf_cookie)
                    .header("x-csrf-token", &csrf_token)
                    .body(json_body(serde_json::json!({
                        "username": "Carol",
                        "email": "other@example.com",
                        "password": "password123"
                    })))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn register_validates_short_password() {
        let state = test_state().await;
        let app = build_router(state, "http://localhost:4321").expect("router");
        let (csrf_cookie, csrf_token) = csrf_pair(&app).await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/register")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, &csrf_cookie)
                    .header("x-csrf-token", &csrf_token)
                    .body(json_body(serde_json::json!({
                        "username": "dave",
                        "email": "dave@example.com",
                        "password": "short"
                    })))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    /// Returns `session=...; csrf=...` cookie header and the csrf token value.
    async fn register_cookie(app: &Router, username: &str) -> (String, String) {
        let (csrf_cookie, csrf_token) = csrf_pair(app).await;
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/register")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, &csrf_cookie)
                    .header("x-csrf-token", &csrf_token)
                    .body(json_body(serde_json::json!({
                        "username": username,
                        "email": format!("{username}@example.com"),
                        "password": "password123"
                    })))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let session = cookie_from(&response).expect("cookie");
        let session = session.split(';').next().unwrap().to_string();
        let cookies = set_cookies(&response);
        let csrf = cookies
            .iter()
            .find(|c| c.starts_with("csrf="))
            .cloned()
            .unwrap_or(csrf_cookie);
        let token = csrf.strip_prefix("csrf=").unwrap_or("").to_string();
        (merge_cookies(&[&session, &csrf]), token)
    }

    #[tokio::test]
    async fn forum_crud_category_thread_post_flow() {
        let state = test_state().await;
        let app = build_router(state.clone(), "http://localhost:4321").expect("router");
        let (cookie, csrf) = register_cookie(&app, "erin").await;

        // Categories are admin-only.
        sqlx::query("UPDATE users SET role = 'admin' WHERE username = 'erin'")
            .execute(&state.db)
            .await
            .expect("promote admin");

        // Non-admin cannot create categories
        let (user_cookie, user_csrf) = register_cookie(&app, "erin_user").await;
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/categories")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, &user_cookie)
                    .header("x-csrf-token", &user_csrf)
                    .body(json_body(serde_json::json!({ "name": "Nope" })))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        // Create category (admin)
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/categories")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, &cookie)
                    .header("x-csrf-token", &csrf)
                    .body(json_body(serde_json::json!({
                        "name": "General",
                        "description": "General chat"
                    })))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["category"]["slug"], "general");

        // Create thread + first post
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/categories/general/threads")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, &cookie)
                    .header("x-csrf-token", &csrf)
                    .body(json_body(serde_json::json!({
                        "title": "Hello World",
                        "body": "Opening **post** content"
                    })))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["thread"]["slug"], "hello-world");
        assert_eq!(json["thread"]["post_count"], 1);
        assert_eq!(json["first_post"]["body"], "Opening **post** content");
        assert!(json["first_post"]["body_html"]
            .as_str()
            .unwrap()
            .contains("<strong>post</strong>"));

        // Reply
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/categories/general/threads/hello-world/posts")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, &cookie)
                    .header("x-csrf-token", &csrf)
                    .body(json_body(serde_json::json!({
                        "body": "A reply"
                    })))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        // List posts
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/categories/general/threads/hello-world/posts?limit=50")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["posts"].as_array().unwrap().len(), 2);
        assert_eq!(json["total"], 2);

        // Thread shows updated post_count + view increment
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/categories/general/threads/hello-world")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["thread"]["post_count"], 2);
        assert!(json["thread"]["view_count"].as_i64().unwrap() >= 1);
    }

    #[tokio::test]
    async fn create_thread_requires_auth() {
        let state = test_state().await;
        let app = build_router(state.clone(), "http://localhost:4321").expect("router");
        let (cookie, csrf) = register_cookie(&app, "frank").await;
        sqlx::query("UPDATE users SET role = 'admin' WHERE username = 'frank'")
            .execute(&state.db)
            .await
            .expect("promote admin");

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/categories")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, &cookie)
                    .header("x-csrf-token", &csrf)
                    .body(json_body(serde_json::json!({ "name": "Offtopic" })))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        let (csrf_cookie, csrf_token) = csrf_pair(&app).await;
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/categories/offtopic/threads")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, &csrf_cookie)
                    .header("x-csrf-token", &csrf_token)
                    .body(json_body(serde_json::json!({
                        "title": "Nope",
                        "body": "should fail"
                    })))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn security_headers_present() {
        let state = test_state().await;
        let app = build_router(state, "http://localhost:4321").expect("router");
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("x-content-type-options").unwrap(),
            "nosniff"
        );
        assert_eq!(response.headers().get("x-frame-options").unwrap(), "DENY");
    }
}
