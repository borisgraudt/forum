//! Double-submit CSRF cookie helpers.

use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use rand::RngCore;
use time::Duration as TimeDuration;

use crate::config::Config;

pub const CSRF_COOKIE_NAME: &str = "csrf";
pub const CSRF_HEADER_NAME: &str = "x-csrf-token";

/// Generate a URL-safe random CSRF token.
pub fn generate_csrf_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn set_csrf_cookie(jar: CookieJar, token: String, config: &Config) -> CookieJar {
    // Readable by the frontend so forms can copy it into X-CSRF-Token (double-submit).
    let mut cookie = Cookie::build((CSRF_COOKIE_NAME, token))
        .http_only(false)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(TimeDuration::days(7))
        .build();

    if config.cookie_secure {
        cookie.set_secure(true);
    }

    jar.add(cookie)
}

pub fn read_csrf_cookie(jar: &CookieJar) -> Option<String> {
    jar.get(CSRF_COOKIE_NAME)
        .map(|c| c.value().to_string())
        .filter(|v| !v.is_empty())
}

/// Constant-time-ish equality for CSRF tokens.
pub fn tokens_match(a: &str, b: &str) -> bool {
    if a.len() != b.len() || a.is_empty() {
        return false;
    }
    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}
