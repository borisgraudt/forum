//! CSRF double-submit middleware for mutating requests.

use axum::body::Body;
use axum::http::{Method, Request, Response, StatusCode};
use axum::response::IntoResponse;
use axum_extra::extract::CookieJar;
use futures_util::future::BoxFuture;
use std::task::{Context, Poll};
use tower::{Layer, Service};

use crate::utils::csrf::{read_csrf_cookie, tokens_match, CSRF_HEADER_NAME};

#[derive(Clone, Copy, Default)]
pub struct CsrfLayer;

impl<S> Layer<S> for CsrfLayer {
    type Service = CsrfService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CsrfService { inner }
    }
}

#[derive(Clone)]
pub struct CsrfService<S> {
    inner: S,
}

fn is_safe(method: &Method) -> bool {
    matches!(
        *method,
        Method::GET | Method::HEAD | Method::OPTIONS | Method::TRACE
    )
}

fn path_exempt(path: &str) -> bool {
    // CSRF issue + logout (session already required; form is in every layout).
    path == "/health"
        || path == "/api/v1/health"
        || path == "/api/v1/auth/csrf"
        || path == "/api/v1/auth/logout"
}

impl<S, B> Service<Request<B>> for CsrfService<S>
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
        let method = req.method().clone();
        let path = req.uri().path().to_string();

        if is_safe(&method) || path_exempt(&path) {
            let mut inner = self.inner.clone();
            return Box::pin(async move { inner.call(req).await });
        }

        let jar = CookieJar::from_headers(req.headers());
        let cookie_token = read_csrf_cookie(&jar);
        let header_token = req
            .headers()
            .get(CSRF_HEADER_NAME)
            .or_else(|| req.headers().get("X-CSRF-Token"))
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);

        let ok = match (cookie_token.as_deref(), header_token.as_deref()) {
            (Some(c), Some(h)) => tokens_match(c, h),
            _ => false,
        };

        if !ok {
            return Box::pin(async move {
                Ok((
                    StatusCode::FORBIDDEN,
                    axum::Json(serde_json::json!({ "error": "csrf token missing or invalid" })),
                )
                    .into_response())
            });
        }

        let mut inner = self.inner.clone();
        Box::pin(async move { inner.call(req).await })
    }
}
