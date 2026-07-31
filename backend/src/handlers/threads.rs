use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use validator::Validate;

use crate::dto::{CreateThreadRequest, ListQuery, ThreadListResponse, ThreadResponse};
use crate::error::AppResult;
use crate::middleware::AuthUser;
use crate::services::{CategoryService, ThreadService};
use crate::state::AppState;
use crate::utils::{slugify, validation_error};

pub fn threads_router() -> Router<AppState> {
    Router::new()
        .route(
            "/categories/{category_slug}/threads",
            get(list_threads).post(create_thread),
        )
        .route(
            "/categories/{category_slug}/threads/{thread_slug}",
            get(get_thread),
        )
}

async fn list_threads(
    State(state): State<AppState>,
    Path(category_slug): Path<String>,
    Query(query): Query<ListQuery>,
) -> AppResult<(StatusCode, Json<ThreadListResponse>)> {
    let category = CategoryService::get_by_slug(&state.db, &category_slug).await?;
    let threads =
        ThreadService::list_by_category(&state.db, category.id, query.limit(), query.offset())
            .await?;
    Ok((StatusCode::OK, Json(ThreadListResponse { threads })))
}

async fn get_thread(
    State(state): State<AppState>,
    Path((category_slug, thread_slug)): Path<(String, String)>,
) -> AppResult<(StatusCode, Json<ThreadResponse>)> {
    let category = CategoryService::get_by_slug(&state.db, &category_slug).await?;
    let thread =
        ThreadService::get_by_category_and_slug(&state.db, category.id, &thread_slug).await?;
    Ok((
        StatusCode::OK,
        Json(ThreadResponse {
            thread,
            first_post: None,
        }),
    ))
}

async fn create_thread(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(category_slug): Path<String>,
    Json(body): Json<CreateThreadRequest>,
) -> AppResult<(StatusCode, Json<ThreadResponse>)> {
    body.validate().map_err(validation_error)?;

    let category = CategoryService::get_by_slug(&state.db, &category_slug).await?;
    let title = body.title.trim().to_string();
    if title.is_empty() {
        return Err(crate::error::AppError::BadRequest(
            "title is required".into(),
        ));
    }

    let slug = body
        .slug
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(slugify)
        .unwrap_or_else(|| slugify(&title));

    let body_text = body.body.trim().to_string();
    if body_text.is_empty() {
        return Err(crate::error::AppError::BadRequest(
            "body is required".into(),
        ));
    }

    let (thread, first_post) = ThreadService::create_with_first_post(
        &state.db,
        category.id,
        user.id,
        &title,
        &slug,
        &body_text,
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(ThreadResponse {
            thread,
            first_post: Some(first_post),
        }),
    ))
}
