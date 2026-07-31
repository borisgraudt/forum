use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::Json;
use axum::Router;
use axum_extra::extract::CookieJar;
use validator::Validate;

use crate::dto::{AuthResponse, LoginRequest, RegisterRequest};
use crate::error::AppResult;
use crate::middleware::AuthUser;
use crate::services::AuthService;
use crate::state::AppState;
use crate::utils::{clear_auth_cookie, set_auth_cookie, validation_error};

pub fn auth_router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/me", get(me))
}

async fn register(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<RegisterRequest>,
) -> AppResult<(StatusCode, CookieJar, Json<AuthResponse>)> {
    body.validate().map_err(validation_error)?;

    let username = body.username.trim().to_string();
    let email = body.email.trim().to_lowercase();
    let display_name = body
        .display_name
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let password_hash = AuthService::hash_password(body.password).await?;
    let user = AuthService::create_user(
        &state.db,
        &username,
        &email,
        &password_hash,
        display_name.as_deref(),
    )
    .await?;

    let token = AuthService::issue_token(&user, &state.config.jwt_secret, state.config.jwt_ttl)?;
    let jar = set_auth_cookie(jar, token, &state.config);

    Ok((
        StatusCode::CREATED,
        jar,
        Json(AuthResponse {
            user: user.into_public(),
        }),
    ))
}

async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<LoginRequest>,
) -> AppResult<(StatusCode, CookieJar, Json<AuthResponse>)> {
    body.validate().map_err(validation_error)?;

    let login = body.login.trim().to_string();
    let user = AuthService::authenticate(&state.db, &login, body.password).await?;

    let token = AuthService::issue_token(&user, &state.config.jwt_secret, state.config.jwt_ttl)?;
    let jar = set_auth_cookie(jar, token, &state.config);

    Ok((
        StatusCode::OK,
        jar,
        Json(AuthResponse {
            user: user.into_public(),
        }),
    ))
}

async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> AppResult<(StatusCode, CookieJar, Json<serde_json::Value>)> {
    let jar = clear_auth_cookie(jar, &state.config);
    Ok((StatusCode::OK, jar, Json(serde_json::json!({ "ok": true }))))
}

async fn me(AuthUser(user): AuthUser) -> AppResult<(StatusCode, Json<AuthResponse>)> {
    Ok((
        StatusCode::OK,
        Json(AuthResponse {
            user: user.into_public(),
        }),
    ))
}
