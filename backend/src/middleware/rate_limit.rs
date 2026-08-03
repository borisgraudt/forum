//! Path-aware in-memory IP rate limiter.
//!
//! Buckets (per IP / window):
//! - `auth`   — register / login: 10 / min
//! - `write`  — create/edit posts & threads: 40 / min
//! - `search` — search: 60 / min
//! - `default`— everything else: 180 / min (SSR-friendly)

use axum::body::Body;
use axum::http::{Method, Request, Response, StatusCode};
use axum::response::IntoResponse;
use futures_util::future::BoxFuture;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use tower::{Layer, Service};

#[derive(Clone)]
pub struct RateLimitLayer {
    state: Arc<Mutex<RateLimitState>>,
}

struct RateLimitState {
    /// key = "{ip}:{bucket}"
    hits: HashMap<String, Vec<Instant>>,
}

#[derive(Clone, Copy)]
struct Bucket {
    name: &'static str,
    max: u32,
    window: Duration,
}

const AUTH: Bucket = Bucket {
    name: "auth",
    max: 10,
    window: Duration::from_secs(60),
};
const WRITE: Bucket = Bucket {
    name: "write",
    max: 40,
    window: Duration::from_secs(60),
};
const SEARCH: Bucket = Bucket {
    name: "search",
    max: 60,
    window: Duration::from_secs(60),
};
const DEFAULT: Bucket = Bucket {
    name: "default",
    max: 180,
    window: Duration::from_secs(60),
};

impl Default for RateLimitLayer {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(RateLimitState {
                hits: HashMap::new(),
            })),
        }
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitService {
            inner,
            state: self.state.clone(),
        }
    }
}

#[derive(Clone)]
pub struct RateLimitService<S> {
    inner: S,
    state: Arc<Mutex<RateLimitState>>,
}

fn client_ip<B>(req: &Request<B>) -> String {
    if let Some(forwarded) = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
    {
        if let Some(first) = forwarded.split(',').next() {
            return first.trim().to_string();
        }
    }
    req.extensions()
        .get::<axum::extract::ConnectInfo<SocketAddr>>()
        .map(|c| c.0.ip().to_string())
        .unwrap_or_else(|| "unknown".into())
}

fn bucket_for(method: &Method, path: &str) -> Bucket {
    // Auth (mutating only — CSRF GET is cheap)
    if matches!(
        *method,
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    ) {
        if path.starts_with("/api/v1/auth/login") || path.starts_with("/api/v1/auth/register") {
            return AUTH;
        }
        // Writes: threads, posts, votes, drafts, reports, profile updates
        if path.contains("/threads")
            || path.contains("/posts")
            || path.starts_with("/api/v1/drafts")
            || path.starts_with("/api/v1/reports")
            || path.starts_with("/api/v1/users/me")
            || path.contains("/me-too")
            || path.contains("/helpful")
        {
            return WRITE;
        }
    }

    if path.starts_with("/api/v1/search") {
        return SEARCH;
    }

    DEFAULT
}

impl<S, B> Service<Request<B>> for RateLimitService<S>
where
    S: Service<Request<B>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    B: Send + 'static,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, S::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let ip = client_ip(&req);
        let bucket = bucket_for(req.method(), req.uri().path());
        let key = format!("{}:{}", ip, bucket.name);
        let now = Instant::now();

        let allowed = {
            let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
            let entry = guard.hits.entry(key).or_default();
            entry.retain(|t| now.duration_since(*t) < bucket.window);
            if entry.len() as u32 >= bucket.max {
                false
            } else {
                entry.push(now);
                true
            }
        };

        if !allowed {
            let msg = format!(
                "rate limit exceeded ({}/min for {})",
                bucket.max, bucket.name
            );
            return Box::pin(async move {
                Ok((
                    StatusCode::TOO_MANY_REQUESTS,
                    axum::Json(serde_json::json!({ "error": msg })),
                )
                    .into_response())
            });
        }

        let mut inner = self.inner.clone();
        Box::pin(async move { inner.call(req).await })
    }
}
