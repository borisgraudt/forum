use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get};
use axum::{Json, Router};
use validator::Validate;

use crate::dto::{CreatePostRequest, ListQuery, PostListResponse, PostResponse};
use crate::error::{AppError, AppResult};
use crate::middleware::AuthUser;
use crate::models::{PostViewJson, UserRole};
use crate::services::{CategoryService, PostService, ThreadService};
use crate::state::AppState;
use crate::utils::validation_error;

pub fn posts_router() -> Router<AppState> {
    Router::new()
        .route(
            "/categories/{category_slug}/threads/{thread_slug}/posts",
            get(list_posts).post(create_post),
        )
        .route(
            "/categories/{category_slug}/threads/{thread_slug}/posts/{post_id}",
            delete(delete_post),
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
    let limit = query.limit();
    let offset = query.offset();
    let total = PostService::count_by_thread(&state.db, thread.id).await?;
    // OP + replies: allow larger default page for thread view convenience.
    let posts = PostService::list_by_thread(&state.db, thread.id, limit, offset).await?;
    let posts = posts.into_iter().map(PostViewJson::from).collect();
    Ok((
        StatusCode::OK,
        Json(PostListResponse {
            posts,
            total,
            limit,
            offset,
        }),
    ))
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
        return Err(AppError::BadRequest("body is required".into()));
    }

    let post = PostService::create_reply(&state.db, thread.id, user.id, &body_text).await?;
    Ok((
        StatusCode::CREATED,
        Json(PostResponse {
            post: PostViewJson::from(post),
        }),
    ))
}

async fn delete_post(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((category_slug, thread_slug, post_id)): Path<(String, String, i64)>,
) -> AppResult<StatusCode> {
    let category = CategoryService::get_by_slug(&state.db, &category_slug).await?;
    let thread =
        ThreadService::get_by_category_and_slug(&state.db, category.id, &thread_slug).await?;

    let post = PostService::get_view(&state.db, post_id).await?;
    if post.thread_id != thread.id {
        return Err(AppError::NotFound);
    }

    let is_mod = matches!(
        user.role_enum(),
        Ok(UserRole::Moderator) | Ok(UserRole::Admin)
    );
    if post.author_id != user.id && !is_mod {
        return Err(AppError::Forbidden);
    }

    // Don't allow deleting the only remaining OP if it's the sole post? Allow for mods.
    PostService::delete(&state.db, post_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
