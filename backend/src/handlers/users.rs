use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, patch};
use axum::{Json, Router};
use validator::Validate;

use crate::dto::{ProfileResponse, UpdateProfileRequest};
use crate::error::AppResult;
use crate::middleware::AuthUser;
use crate::models::UserPublic;
use crate::services::UserService;
use crate::state::AppState;
use crate::utils::validation_error;

pub fn users_router() -> Router<AppState> {
    Router::new()
        .route("/users/me", patch(update_me))
        .route("/users/{username}", get(get_profile))
}

async fn get_profile(
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> AppResult<(StatusCode, Json<ProfileResponse>)> {
    let profile = UserService::profile(&state.db, &username).await?;
    Ok((StatusCode::OK, Json(ProfileResponse { profile })))
}

async fn update_me(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(body): Json<UpdateProfileRequest>,
) -> AppResult<(StatusCode, Json<UserPublic>)> {
    body.validate().map_err(validation_error)?;

    let display_name = body.display_name.map(|s| {
        let t = s.trim().to_string();
        if t.is_empty() {
            None
        } else {
            Some(t)
        }
    });
    let bio = body.bio.map(|s| {
        let t = s.trim().to_string();
        if t.is_empty() {
            None
        } else {
            Some(t)
        }
    });

    let user = UserService::update_profile(&state.db, user.id, display_name, bio).await?;
    Ok((StatusCode::OK, Json(user)))
}
