//! Simple in-memory IP rate limiter for API abuse resistance.

use axum::body::Body;
use axum::http::{Request, Response, StatusCode};
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
    max_requests: u32,
    window: Duration,
}

struct RateLimitState {
    hits: HashMap<String, Vec<Instant>>,
}

impl RateLimitLayer {
    pub fn new(max_requests: u32, window: Duration) -> Self {
        Self {
            state: Arc::new(Mutex::new(RateLimitState {
                hits: HashMap::new(),
            })),
            max_requests,
            window,
        }
    }
}

impl Default for RateLimitLayer {
    fn default() -> Self {
        // 120 requests / minute / IP is plenty for a small community.
        Self::new(120, Duration::from_secs(60))
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitService {
            inner,
            state: self.state.clone(),
            max_requests: self.max_requests,
            window: self.window,
        }
    }
}

#[derive(Clone)]
pub struct RateLimitService<S> {
    inner: S,
    state: Arc<Mutex<RateLimitState>>,
    max_requests: u32,
    window: Duration,
}

fn client_key<B>(req: &Request<B>) -> String {
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

impl<S, B> Service<Request<B>> for RateLimitService<S>
where
    S: Service<Request<B>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    B: Send + 'static,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let key = client_key(&req);
        let now = Instant::now();
        let allowed = {
            let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
            let window = self.window;
            let max = self.max_requests;
            let entry = guard.hits.entry(key).or_default();
            entry.retain(|t| now.duration_since(*t) < window);
            if entry.len() as u32 >= max {
                false
            } else {
                entry.push(now);
                true
            }
        };

        if !allowed {
            return Box::pin(async move {
                Ok((
                    StatusCode::TOO_MANY_REQUESTS,
                    axum::Json(serde_json::json!({ "error": "rate limit exceeded" })),
                )
                    .into_response())
            });
        }

        let mut inner = self.inner.clone();
        Box::pin(async move { inner.call(req).await })
    }
}
