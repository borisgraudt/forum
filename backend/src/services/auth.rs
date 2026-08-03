use std::time::Duration;

use anyhow::anyhow;
use chrono::{Duration as ChronoDuration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::{User, UserRole};

/// Lower cost in tests for speed; production uses a stronger work factor.
#[cfg(test)]
const BCRYPT_COST: u32 = 4;
#[cfg(not(test))]
const BCRYPT_COST: u32 = 12;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject: user id.
    pub sub: i64,
    pub username: String,
    pub role: String,
    pub exp: i64,
    pub iat: i64,
}

pub struct AuthService;

impl AuthService {
    pub async fn hash_password(password: String) -> AppResult<String> {
        tokio::task::spawn_blocking(move || bcrypt::hash(password, BCRYPT_COST))
            .await
            .map_err(|e| AppError::Internal(anyhow!("hash join error: {e}")))?
            .map_err(|e| AppError::Internal(anyhow!("bcrypt hash failed: {e}")))
    }

    pub async fn verify_password(password: String, hash: String) -> AppResult<bool> {
        tokio::task::spawn_blocking(move || bcrypt::verify(password, &hash))
            .await
            .map_err(|e| AppError::Internal(anyhow!("verify join error: {e}")))?
            .map_err(|e| AppError::Internal(anyhow!("bcrypt verify failed: {e}")))
    }

    pub fn issue_token(user: &User, secret: &str, ttl: Duration) -> AppResult<String> {
        let now = Utc::now();
        let exp = now
            + ChronoDuration::from_std(ttl)
                .map_err(|e| AppError::Internal(anyhow!("invalid jwt ttl: {e}")))?;

        let claims = Claims {
            sub: user.id,
            username: user.username.clone(),
            role: user.role.clone(),
            iat: now.timestamp(),
            exp: exp.timestamp(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .map_err(|e| AppError::Internal(anyhow!("jwt encode failed: {e}")))
    }

    pub fn decode_token(token: &str, secret: &str) -> AppResult<Claims> {
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        )
        .map(|data| data.claims)
        .map_err(|_| AppError::Unauthorized)
    }

    pub async fn create_user(
        db: &SqlitePool,
        username: &str,
        email: &str,
        password_hash: &str,
        display_name: Option<&str>,
    ) -> AppResult<User> {
        let role = UserRole::User.as_str();

        let result = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (username, email, password_hash, display_name, role)
            VALUES (?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(username)
        .bind(email)
        .bind(password_hash)
        .bind(display_name)
        .bind(role)
        .fetch_one(db)
        .await;

        match result {
            Ok(user) => Ok(user),
            Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
                Err(AppError::Conflict("username or email already taken".into()))
            }
            Err(err) => Err(AppError::Sqlx(err)),
        }
    }

    pub async fn find_by_login(db: &SqlitePool, login: &str) -> AppResult<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT * FROM users
            WHERE username = ? COLLATE NOCASE OR email = ? COLLATE NOCASE
            LIMIT 1
            "#,
        )
        .bind(login)
        .bind(login)
        .fetch_optional(db)
        .await?;

        Ok(user)
    }

    pub async fn find_by_id(db: &SqlitePool, id: i64) -> AppResult<Option<User>> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
            .bind(id)
            .fetch_optional(db)
            .await?;
        Ok(user)
    }

    /// Authenticate and return the user, or a generic unauthorized error.
    pub async fn authenticate(db: &SqlitePool, login: &str, password: String) -> AppResult<User> {
        use crate::services::ModerationService;

        let user = Self::find_by_login(db, login)
            .await?
            .ok_or(AppError::Unauthorized)?;

        if !user.is_active {
            return Err(AppError::Unauthorized);
        }

        if ModerationService::is_banned(db, user.id).await? {
            return Err(AppError::Forbidden);
        }

        let ok = Self::verify_password(password, user.password_hash.clone()).await?;
        if !ok {
            return Err(AppError::Unauthorized);
        }

        Ok(user)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn password_hash_roundtrip() {
        let hash = AuthService::hash_password("correct horse battery".into())
            .await
            .unwrap();
        assert!(
            AuthService::verify_password("correct horse battery".into(), hash.clone())
                .await
                .unwrap()
        );
        assert!(!AuthService::verify_password("wrong".into(), hash)
            .await
            .unwrap());
    }

    #[test]
    fn jwt_roundtrip() {
        let user = User {
            id: 42,
            username: "alice".into(),
            email: "a@b.c".into(),
            password_hash: "x".into(),
            display_name: None,
            bio: None,
            role: "user".into(),
            is_active: true,
            created_at: "t".into(),
            updated_at: "t".into(),
        };
        let token =
            AuthService::issue_token(&user, "test-secret-at-least-16", Duration::from_secs(60))
                .unwrap();
        let claims = AuthService::decode_token(&token, "test-secret-at-least-16").unwrap();
        assert_eq!(claims.sub, 42);
        assert_eq!(claims.username, "alice");
    }
}
