use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::fmt;
use std::str::FromStr;

/// Application role stored as lowercase text in SQLite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    User,
    Moderator,
    Admin,
}

impl UserRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Moderator => "moderator",
            Self::Admin => "admin",
        }
    }

    pub fn is_staff(self) -> bool {
        matches!(self, Self::Moderator | Self::Admin)
    }

    pub fn is_admin(self) -> bool {
        matches!(self, Self::Admin)
    }
}

impl fmt::Display for UserRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for UserRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "user" => Ok(Self::User),
            "moderator" => Ok(Self::Moderator),
            "admin" => Ok(Self::Admin),
            other => Err(format!("unknown user role: {other}")),
        }
    }
}

/// Full user row including credentials. Prefer [`UserPublic`] for API responses.
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_key: Option<String>,
    pub email_verified: bool,
    pub role: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl User {
    pub fn role_enum(&self) -> Result<UserRole, String> {
        UserRole::from_str(&self.role)
    }

    pub fn avatar_url(&self) -> Option<String> {
        self.avatar_key.as_ref().map(|k| format!("/media/{k}"))
    }

    pub fn into_public(self) -> UserPublic {
        let avatar_url = self.avatar_url();
        UserPublic {
            id: self.id,
            username: self.username,
            display_name: self.display_name,
            bio: self.bio,
            avatar_url,
            email_verified: self.email_verified,
            role: self.role,
            is_active: self.is_active,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

/// Safe user projection for JSON responses (no email / password).
#[derive(Debug, Clone, Serialize)]
pub struct UserPublic {
    pub id: i64,
    pub username: String,
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    pub email_verified: bool,
    pub role: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Recent topic authored by the user (for profile activity).
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ProfileThreadItem {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub category_slug: String,
    pub category_name: String,
    pub created_at: String,
    pub post_count: i64,
}

/// Recent non-OP post by the user.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ProfilePostItem {
    pub id: i64,
    pub thread_id: i64,
    pub thread_title: String,
    pub thread_slug: String,
    pub category_slug: String,
    pub body_preview: String,
    pub created_at: String,
}

/// Public profile with light activity stats.
#[derive(Debug, Clone, Serialize)]
pub struct UserProfile {
    pub user: UserPublic,
    pub post_count: i64,
    pub thread_count: i64,
    pub helpful_received: i64,
    pub reputation: i64,
    pub recent_threads: Vec<ProfileThreadItem>,
    pub recent_posts: Vec<ProfilePostItem>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_roundtrip() {
        for role in [UserRole::User, UserRole::Moderator, UserRole::Admin] {
            assert_eq!(UserRole::from_str(role.as_str()).unwrap(), role);
        }
    }

    #[test]
    fn password_hash_is_not_serialized() {
        let user = User {
            id: 1,
            username: "alice".into(),
            email: "alice@example.com".into(),
            password_hash: "secret-hash".into(),
            display_name: None,
            bio: None,
            avatar_key: None,
            email_verified: false,
            role: "user".into(),
            is_active: true,
            created_at: "2026-01-01T00:00:00.000Z".into(),
            updated_at: "2026-01-01T00:00:00.000Z".into(),
        };
        let json = serde_json::to_string(&user).unwrap();
        assert!(!json.contains("secret-hash"));
        assert!(!json.contains("password"));
    }
}
