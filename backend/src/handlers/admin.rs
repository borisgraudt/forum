use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, patch};
use axum::{Json, Router};
use validator::Validate;

use crate::dto::{
    AdminUpdateCategoryRequest, AdminUpdateUserRequest, CategoryListResponse, CategoryResponse,
    ListQuery, UserListResponse,
};
use crate::error::{AppError, AppResult};
use crate::middleware::AuthUser;
use crate::models::{User, UserPublic};
use crate::services::{CategoryService, UserService};
use crate::state::AppState;
use crate::utils::validation_error;

pub fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/users", get(list_users))
        .route("/users/{user_id}", patch(update_user))
        .route("/categories", get(list_categories))
        .route(
            "/categories/{category_id}",
            patch(update_category).delete(delete_category),
        )
}

async fn list_users(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Query(query): Query<ListQuery>,
) -> AppResult<(StatusCode, Json<UserListResponse>)> {
    require_admin(&user)?;
    let limit = query.limit();
    let offset = query.offset();
    let (users, total) = UserService::list(&state.db, limit, offset).await?;
    Ok((
        StatusCode::OK,
        Json(UserListResponse {
            users,
            total,
            limit,
            offset,
        }),
    ))
}

async fn update_user(
    State(state): State<AppState>,
    AuthUser(admin): AuthUser,
    Path(user_id): Path<i64>,
    Json(body): Json<AdminUpdateUserRequest>,
) -> AppResult<(StatusCode, Json<UserPublic>)> {
    require_admin(&admin)?;
    body.validate().map_err(validation_error)?;

    if admin.id == user_id {
        if body.is_active == Some(false) {
            return Err(AppError::BadRequest(
                "cannot deactivate your own account".into(),
            ));
        }
        if body.role.as_deref() == Some("user") || body.role.as_deref() == Some("moderator") {
            return Err(AppError::BadRequest(
                "cannot demote your own admin role".into(),
            ));
        }
    }

    let user =
        UserService::admin_update(&state.db, user_id, body.role.as_deref(), body.is_active).await?;

    let _ = crate::services::ModerationService::audit(
        &state.db,
        Some(admin.id),
        "admin.user_update",
        Some("user"),
        Some(user_id),
        Some(&format!(
            r#"{{"role":{},"is_active":{}}}"#,
            body.role
                .as_ref()
                .map(|r| format!("\"{r}\""))
                .unwrap_or_else(|| "null".into()),
            body.is_active
                .map(|b| b.to_string())
                .unwrap_or_else(|| "null".into())
        )),
    )
    .await;

    Ok((StatusCode::OK, Json(user)))
}

async fn list_categories(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> AppResult<(StatusCode, Json<CategoryListResponse>)> {
    require_admin(&user)?;
    let categories = CategoryService::list_all(&state.db).await?;
    Ok((StatusCode::OK, Json(CategoryListResponse { categories })))
}

async fn update_category(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(category_id): Path<i64>,
    Json(body): Json<AdminUpdateCategoryRequest>,
) -> AppResult<(StatusCode, Json<CategoryResponse>)> {
    require_admin(&user)?;
    body.validate().map_err(validation_error)?;

    let name = body
        .name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let description = if body.clear_description == Some(true) {
        Some(None)
    } else if let Some(ref d) = body.description {
        let t = d.trim();
        if t.is_empty() {
            Some(None)
        } else {
            Some(Some(t))
        }
    } else {
        None
    };

    let category =
        CategoryService::update(&state.db, category_id, name, description, body.sort_order).await?;

    Ok((
        StatusCode::OK,
        Json(CategoryResponse {
            category,
            children: None,
            parent: None,
        }),
    ))
}

async fn delete_category(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(category_id): Path<i64>,
) -> AppResult<StatusCode> {
    require_admin(&user)?;
    CategoryService::delete_if_empty(&state.db, category_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

fn require_admin(user: &User) -> AppResult<()> {
    match user.role_enum() {
        Ok(role) if role.is_admin() => Ok(()),
        _ => Err(AppError::Forbidden),
    }
}
