mod config;
mod db;
mod error;
mod models;
mod state;

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
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, header::COOKIE])
        .allow_credentials(true);

    Ok(Router::new()
        .route("/health", get(health))
        .route("/api/v1/health", get(health))
        .layer(TraceLayer::new_for_http())
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
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn test_state() -> AppState {
        let config = Config {
            host: "127.0.0.1".into(),
            port: 0,
            database_url: "sqlite::memory:".into(),
            jwt_secret: "test-secret-at-least-16".into(),
            cors_origin: "http://localhost:4321".into(),
            rust_log: "error".into(),
        };
        let pool = db::connect(&config.database_url)
            .await
            .expect("connect memory db");
        db::migrate(&pool).await.expect("migrate");
        AppState::new(config, pool)
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
}
