use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum_extra::extract::CookieJar;

use crate::error::{AppError, AppResult};
use crate::models::User;
use crate::services::AuthService;
use crate::state::AppState;
use crate::utils::cookies::read_auth_token;

/// Authenticated user loaded from the session cookie + DB.
#[derive(Debug, Clone)]
pub struct AuthUser(pub User);

async fn load_user_from_jar(parts: &Parts, state: &AppState) -> AppResult<Option<User>> {
    let jar = CookieJar::from_headers(&parts.headers);
    let Some(token) = read_auth_token(&jar) else {
        return Ok(None);
    };

    let claims = AuthService::decode_token(&token, &state.config.jwt_secret)?;
    let user = AuthService::find_by_id(&state.db, claims.sub)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if !user.is_active {
        return Err(AppError::Unauthorized);
    }

    Ok(Some(user))
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        load_user_from_jar(parts, state)
            .await?
            .map(AuthUser)
            .ok_or(AppError::Unauthorized)
    }
}

/// Optional session: `None` when anonymous (not an error).
#[derive(Debug, Clone)]
pub struct OptionalAuthUser(pub Option<User>);

impl FromRequestParts<AppState> for OptionalAuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(OptionalAuthUser(load_user_from_jar(parts, state).await?))
    }
}
