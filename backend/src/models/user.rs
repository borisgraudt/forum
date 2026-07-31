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
    pub role: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl User {
    pub fn role_enum(&self) -> Result<UserRole, String> {
        UserRole::from_str(&self.role)
    }

    pub fn into_public(self) -> UserPublic {
        UserPublic {
            id: self.id,
            username: self.username,
            display_name: self.display_name,
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
    pub role: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
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
