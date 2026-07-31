use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use time::Duration as TimeDuration;

use crate::config::{Config, AUTH_COOKIE_NAME};

pub fn set_auth_cookie(jar: CookieJar, token: String, config: &Config) -> CookieJar {
    let max_age = TimeDuration::seconds(config.jwt_ttl.as_secs() as i64);

    let mut cookie = Cookie::build((AUTH_COOKIE_NAME, token))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(max_age)
        .build();

    if config.cookie_secure {
        cookie.set_secure(true);
    }

    jar.add(cookie)
}

pub fn clear_auth_cookie(jar: CookieJar, config: &Config) -> CookieJar {
    let mut cookie = Cookie::build((AUTH_COOKIE_NAME, ""))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(TimeDuration::seconds(0))
        .build();

    if config.cookie_secure {
        cookie.set_secure(true);
    }

    jar.remove(cookie)
}

pub fn read_auth_token(jar: &CookieJar) -> Option<String> {
    jar.get(AUTH_COOKIE_NAME)
        .map(|c| c.value().to_string())
        .filter(|v| !v.is_empty())
}
