use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use validator::Validate;

use crate::dto::{CreatePostRequest, ListQuery, PostListResponse, PostResponse};
use crate::error::AppResult;
use crate::middleware::AuthUser;
use crate::services::{CategoryService, PostService, ThreadService};
use crate::state::AppState;
use crate::utils::validation_error;

pub fn posts_router() -> Router<AppState> {
    Router::new().route(
        "/categories/{category_slug}/threads/{thread_slug}/posts",
        get(list_posts).post(create_post),
    )
}

async fn list_posts(
    State(state): State<AppState>,
    Path((category_slug, thread_slug)): Path<(String, String)>,
    Query(query): Query<ListQuery>,
) -> AppResult<(StatusCode, Json<PostListResponse>)> {
    let category = CategoryService::get_by_slug(&state.db, &category_slug).await?;
    let thread =
        ThreadService::get_by_category_and_slug(&state.db, category.id, &thread_slug).await?;
    let posts =
        PostService::list_by_thread(&state.db, thread.id, query.limit(), query.offset()).await?;
    Ok((StatusCode::OK, Json(PostListResponse { posts })))
}

async fn create_post(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((category_slug, thread_slug)): Path<(String, String)>,
    Json(body): Json<CreatePostRequest>,
) -> AppResult<(StatusCode, Json<PostResponse>)> {
    body.validate().map_err(validation_error)?;

    let category = CategoryService::get_by_slug(&state.db, &category_slug).await?;
    let thread =
        ThreadService::get_by_category_and_slug(&state.db, category.id, &thread_slug).await?;

    let body_text = body.body.trim().to_string();
    if body_text.is_empty() {
        return Err(crate::error::AppError::BadRequest(
            "body is required".into(),
        ));
    }

    let post = PostService::create_reply(&state.db, thread.id, user.id, &body_text).await?;
    Ok((StatusCode::CREATED, Json(PostResponse { post })))
}
