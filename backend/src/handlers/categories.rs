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
    // Register `/children` before `/{slug}` so the path is not swallowed.
    Router::new()
        .route("/", get(list_root_categories).post(create_category))
        .route("/{slug}/children", get(list_children))
        .route("/{slug}", get(get_category))
}

/// Top-level communities only (for Browse).
async fn list_root_categories(
    State(state): State<AppState>,
) -> AppResult<(StatusCode, Json<CategoryListResponse>)> {
    let categories = CategoryService::list_roots(&state.db).await?;
    Ok((StatusCode::OK, Json(CategoryListResponse { categories })))
}

async fn get_category(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> AppResult<(StatusCode, Json<CategoryResponse>)> {
    let category = CategoryService::get_by_slug(&state.db, &slug).await?;
    let children = if category.parent_id.is_none() {
        Some(CategoryService::list_children(&state.db, category.id).await?)
    } else {
        None
    };
    let parent = if let Some(parent_id) = category.parent_id {
        Some(CategoryService::get_by_id(&state.db, parent_id).await?)
    } else {
        None
    };
    Ok((
        StatusCode::OK,
        Json(CategoryResponse {
            category,
            children,
            parent,
        }),
    ))
}

async fn list_children(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> AppResult<(StatusCode, Json<CategoryListResponse>)> {
    let parent = CategoryService::get_by_slug(&state.db, &slug).await?;
    let categories = CategoryService::list_children(&state.db, parent.id).await?;
    Ok((StatusCode::OK, Json(CategoryListResponse { categories })))
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

    let parent_id = match body
        .parent_slug
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(parent_slug) => {
            let parent = CategoryService::get_by_slug(&state.db, parent_slug).await?;
            Some(parent.id)
        }
        None => None,
    };

    let category = CategoryService::create(
        &state.db,
        &name,
        &slug,
        description.as_deref(),
        sort_order,
        parent_id,
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(CategoryResponse {
            category,
            children: None,
            parent: None,
        }),
    ))
}
