use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use validator::Validate;

use crate::dto::{CategoryListResponse, CategoryResponse, CreateCategoryRequest};
use crate::error::AppResult;
use crate::middleware::AuthUser;
use crate::services::CategoryService;
use crate::state::AppState;
use crate::utils::{slugify, validation_error};

pub fn categories_router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_categories).post(create_category))
        .route("/{slug}", get(get_category))
}

async fn list_categories(
    State(state): State<AppState>,
) -> AppResult<(StatusCode, Json<CategoryListResponse>)> {
    let categories = CategoryService::list(&state.db).await?;
    Ok((StatusCode::OK, Json(CategoryListResponse { categories })))
}

async fn get_category(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> AppResult<(StatusCode, Json<CategoryResponse>)> {
    let category = CategoryService::get_by_slug(&state.db, &slug).await?;
    Ok((StatusCode::OK, Json(CategoryResponse { category })))
}

async fn create_category(
    State(state): State<AppState>,
    AuthUser(_user): AuthUser,
    Json(body): Json<CreateCategoryRequest>,
) -> AppResult<(StatusCode, Json<CategoryResponse>)> {
    body.validate().map_err(validation_error)?;

    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Err(crate::error::AppError::BadRequest(
            "name is required".into(),
        ));
    }

    let slug = body
        .slug
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(slugify)
        .unwrap_or_else(|| slugify(&name));

    let description = body
        .description
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let sort_order = body.sort_order.unwrap_or(0);

    let category =
        CategoryService::create(&state.db, &name, &slug, description.as_deref(), sort_order)
            .await?;

    Ok((StatusCode::CREATED, Json(CategoryResponse { category })))
}
