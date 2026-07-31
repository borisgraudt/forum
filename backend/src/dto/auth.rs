use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::models::UserPublic;

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(length(min = 3, max = 32, message = "username must be 3-32 characters"))]
    #[validate(regex(
        path = "*crate::dto::auth::USERNAME_RE",
        message = "username may only contain letters, numbers, and underscores"
    ))]
    pub username: String,

    #[validate(email(message = "invalid email address"))]
    #[validate(length(max = 254, message = "email is too long"))]
    pub email: String,

    #[validate(length(min = 8, max = 128, message = "password must be 8-128 characters"))]
    pub password: String,

    #[validate(length(max = 64, message = "display_name is too long"))]
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    /// Username or email.
    #[validate(length(min = 1, max = 254, message = "login is required"))]
    pub login: String,

    #[validate(length(min = 1, max = 128, message = "password is required"))]
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user: UserPublic,
}

/// ASCII letters, digits, underscore; must start with a letter or underscore.
pub static USERNAME_RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
    regex::Regex::new(r"^[A-Za-z_][A-Za-z0-9_]{2,31}$").expect("username regex")
});
