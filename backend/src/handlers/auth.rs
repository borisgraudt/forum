use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::Json;
use axum::Router;
use axum_extra::extract::CookieJar;
use validator::Validate;

use crate::dto::{
    AuthResponse, CsrfResponse, EmailVerifyConfirm, LoginRequest, PasswordResetConfirm,
    PasswordResetRequest, RegisterRequest,
};
use crate::error::AppResult;
use crate::middleware::AuthUser;
use crate::services::{AuthService, EmailService};
use crate::state::AppState;
use crate::utils::csrf::{generate_csrf_token, read_csrf_cookie, set_csrf_cookie};
use crate::utils::{clear_auth_cookie, set_auth_cookie, validation_error};

pub fn auth_router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/me", get(me))
        .route("/csrf", get(csrf))
        .route("/password-reset", post(password_reset_request))
        .route("/password-reset/confirm", post(password_reset_confirm))
        .route("/verify-email", post(verify_email))
        .route("/verify-email/request", post(request_verify_email))
}

async fn csrf(
    State(state): State<AppState>,
    jar: CookieJar,
) -> AppResult<(StatusCode, CookieJar, Json<CsrfResponse>)> {
    let token = read_csrf_cookie(&jar).unwrap_or_else(generate_csrf_token);
    let jar = set_csrf_cookie(jar, token.clone(), &state.config);
    Ok((
        StatusCode::OK,
        jar,
        Json(CsrfResponse { csrf_token: token }),
    ))
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

    // Issue email verification token (dev: logged).
    if let Ok(raw) = EmailService::issue_token(&state.db, user.id, "verify", 48).await {
        let link = format!(
            "{}/verify-email?token={}",
            state.config.public_origin.trim_end_matches('/'),
            raw
        );
        EmailService::send_log(
            "verify",
            &email,
            "Verify your Forum email",
            &format!("Open: {link}"),
        );
    }

    let token = AuthService::issue_token(&user, &state.config.jwt_secret, state.config.jwt_ttl)?;
    let jar = set_auth_cookie(jar, token, &state.config);
    let csrf = generate_csrf_token();
    let jar = set_csrf_cookie(jar, csrf, &state.config);

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
    let csrf = generate_csrf_token();
    let jar = set_csrf_cookie(jar, csrf, &state.config);

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

/// Always 200 to avoid account enumeration.
async fn password_reset_request(
    State(state): State<AppState>,
    Json(body): Json<PasswordResetRequest>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    body.validate().map_err(validation_error)?;
    let login = body.login.trim();
    if let Ok(Some(user)) = AuthService::find_by_login(&state.db, login).await {
        if let Ok(raw) = EmailService::issue_token(&state.db, user.id, "reset", 2).await {
            let link = format!(
                "{}/reset-password?token={}",
                state.config.public_origin.trim_end_matches('/'),
                raw
            );
            EmailService::send_log(
                "reset",
                &user.email,
                "Reset your Forum password",
                &format!("Open: {link}"),
            );
        }
    }
    Ok((
        StatusCode::OK,
        Json(
            serde_json::json!({ "ok": true, "message": "if the account exists, a reset link was sent" }),
        ),
    ))
}

async fn password_reset_confirm(
    State(state): State<AppState>,
    Json(body): Json<PasswordResetConfirm>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    body.validate().map_err(validation_error)?;
    let user_id = EmailService::consume_token(&state.db, "reset", body.token.trim()).await?;
    let hash = AuthService::hash_password(body.password).await?;
    sqlx::query(
        r#"
        UPDATE users
        SET password_hash = ?,
            updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
        WHERE id = ?
        "#,
    )
    .bind(&hash)
    .bind(user_id)
    .execute(&state.db)
    .await?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "ok": true }))))
}

async fn verify_email(
    State(state): State<AppState>,
    Json(body): Json<EmailVerifyConfirm>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    body.validate().map_err(validation_error)?;
    let user_id = EmailService::consume_token(&state.db, "verify", body.token.trim()).await?;
    sqlx::query(
        r#"
        UPDATE users
        SET email_verified = 1,
            updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
        WHERE id = ?
        "#,
    )
    .bind(user_id)
    .execute(&state.db)
    .await?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "ok": true }))))
}

async fn request_verify_email(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    if user.email_verified {
        return Ok((
            StatusCode::OK,
            Json(serde_json::json!({ "ok": true, "message": "already verified" })),
        ));
    }
    let raw = EmailService::issue_token(&state.db, user.id, "verify", 48).await?;
    let link = format!(
        "{}/verify-email?token={}",
        state.config.public_origin.trim_end_matches('/'),
        raw
    );
    EmailService::send_log(
        "verify",
        &user.email,
        "Verify your Forum email",
        &format!("Open: {link}"),
    );
    Ok((StatusCode::OK, Json(serde_json::json!({ "ok": true }))))
}
