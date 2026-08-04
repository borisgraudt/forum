use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use validator::Validate;

use crate::dto::{
    CreatePostRequest, ListQuery, PostEditListResponse, PostListResponse, PostResponse,
    UpdatePostRequest, VoteCountResponse,
};
use crate::error::{AppError, AppResult};
use crate::middleware::{AuthUser, OptionalAuthUser};
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
            delete(delete_post).patch(update_post),
        )
        .route(
            "/categories/{category_slug}/threads/{thread_slug}/posts/{post_id}/helpful",
            post(add_helpful).delete(remove_helpful),
        )
        .route(
            "/categories/{category_slug}/threads/{thread_slug}/posts/{post_id}/edits",
            get(list_edits),
        )
}

async fn list_posts(
    State(state): State<AppState>,
    OptionalAuthUser(viewer): OptionalAuthUser,
    Path((category_slug, thread_slug)): Path<(String, String)>,
    Query(query): Query<ListQuery>,
) -> AppResult<(StatusCode, Json<PostListResponse>)> {
    let category = CategoryService::get_by_slug(&state.db, &category_slug).await?;
    let thread =
        ThreadService::get_by_category_and_slug(&state.db, category.id, &thread_slug).await?;
    let limit = query.limit();
    let offset = query.offset();
    // Keep soft-deleted placeholders so reply_to chain stays readable for staff? Show all with flag.
    let total = PostService::count_by_thread(&state.db, thread.id, true).await?;
    let posts = PostService::list_by_thread(&state.db, thread.id, limit, offset, true).await?;

    let mut out = Vec::with_capacity(posts.len());
    for p in posts {
        let voted = if let Some(ref u) = viewer {
            PostService::viewer_helpful(&state.db, p.id, u.id).await?
        } else {
            false
        };
        let mut json = PostViewJson::from_view(p, voted);
        enrich_post(&state, &mut json).await?;
        out.push(json);
    }

    Ok((
        StatusCode::OK,
        Json(PostListResponse {
            posts: out,
            total,
            limit,
            offset,
        }),
    ))
}

async fn enrich_post(state: &AppState, json: &mut PostViewJson) -> AppResult<()> {
    if json.is_deleted {
        return Ok(());
    }
    let atts = crate::services::MediaService::list_for_post(&state.db, json.id).await?;
    json.attachments = atts
        .into_iter()
        .map(crate::models::AttachmentJson::from)
        .collect();

    // Only cached embeds — never block list on network (fast path).
    for url in crate::services::EmbedService::extract_urls(&json.body, 3) {
        if let Ok(Some(e)) = crate::services::EmbedService::get_cached(&state.db, &url).await {
            if e.status == "ok" {
                json.embeds.push(e);
            }
        }
    }
    Ok(())
}

async fn create_post(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((category_slug, thread_slug)): Path<(String, String)>,
    Json(body): Json<CreatePostRequest>,
) -> AppResult<(StatusCode, Json<PostResponse>)> {
    body.validate().map_err(validation_error)?;
    crate::services::ModerationService::ensure_can_post(&state.db, user.id).await?;

    let category = CategoryService::get_by_slug(&state.db, &category_slug).await?;
    let thread =
        ThreadService::get_by_category_and_slug(&state.db, category.id, &thread_slug).await?;

    let body_text = body.body.trim().to_string();
    if body_text.is_empty() {
        return Err(AppError::BadRequest("body is required".into()));
    }

    let post = PostService::create_reply(
        &state.db,
        thread.id,
        user.id,
        &body_text,
        body.reply_to_post_id,
    )
    .await?;

    crate::services::hooks::emit(crate::services::hooks::ForumEvent::PostCreated {
        post_id: post.id,
        thread_id: thread.id,
        author_id: user.id,
    });

    if let Some(ids) = body.attachment_ids.as_ref() {
        if !ids.is_empty() {
            crate::services::MediaService::link_to_post(&state.db, post.id, ids, user.id).await?;
        }
    }

    // Prefetch embeds + notify watchers off the hot path (no latency on reply).
    let db = state.db.clone();
    let body_for_embed = body_text.clone();
    let thread_id = thread.id;
    let category_id = category.id;
    let actor_id = user.id;
    let post_id = post.id;
    let actor_name = user.username.clone();
    tokio::spawn(async move {
        for url in crate::services::EmbedService::extract_urls(&body_for_embed, 3) {
            let _ = crate::services::EmbedService::get_or_fetch(&db, &url).await;
        }
        let _ = crate::services::EngagementService::notify_watchers(
            &db,
            thread_id,
            category_id,
            actor_id,
            "reply",
            Some(post_id),
            &format!("@{actor_name} replied"),
        )
        .await;
        let _ = crate::services::EngagementService::watch(&db, actor_id, "thread", thread_id).await;
    });

    let mut json = PostViewJson::from(post);
    enrich_post(&state, &mut json).await?;
    Ok((StatusCode::CREATED, Json(PostResponse { post: json })))
}

async fn update_post(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((category_slug, thread_slug, post_id)): Path<(String, String, i64)>,
    Json(body): Json<UpdatePostRequest>,
) -> AppResult<(StatusCode, Json<PostResponse>)> {
    body.validate().map_err(validation_error)?;
    let category = CategoryService::get_by_slug(&state.db, &category_slug).await?;
    let thread =
        ThreadService::get_by_category_and_slug(&state.db, category.id, &thread_slug).await?;
    let post = PostService::get_view(&state.db, post_id).await?;
    if post.thread_id != thread.id {
        return Err(AppError::NotFound);
    }

    let is_staff = user.role_enum().map(|r| r.is_staff()).unwrap_or(false);
    if post.author_id != user.id && !is_staff {
        return Err(AppError::Forbidden);
    }

    let body_text = body.body.trim().to_string();
    if body_text.is_empty() {
        return Err(AppError::BadRequest("body is required".into()));
    }

    let post = PostService::update_body(&state.db, post_id, user.id, &body_text).await?;
    Ok((
        StatusCode::OK,
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

    PostService::soft_delete(&state.db, post_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_edits(
    State(state): State<AppState>,
    Path((category_slug, thread_slug, post_id)): Path<(String, String, i64)>,
) -> AppResult<(StatusCode, Json<PostEditListResponse>)> {
    let category = CategoryService::get_by_slug(&state.db, &category_slug).await?;
    let thread =
        ThreadService::get_by_category_and_slug(&state.db, category.id, &thread_slug).await?;
    let post = PostService::get_view(&state.db, post_id).await?;
    if post.thread_id != thread.id {
        return Err(AppError::NotFound);
    }
    let edits = PostService::list_edits(&state.db, post_id).await?;
    Ok((StatusCode::OK, Json(PostEditListResponse { edits })))
}

async fn add_helpful(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((category_slug, thread_slug, post_id)): Path<(String, String, i64)>,
) -> AppResult<(StatusCode, Json<VoteCountResponse>)> {
    let category = CategoryService::get_by_slug(&state.db, &category_slug).await?;
    let thread =
        ThreadService::get_by_category_and_slug(&state.db, category.id, &thread_slug).await?;
    let post = PostService::get_view(&state.db, post_id).await?;
    if post.thread_id != thread.id {
        return Err(AppError::NotFound);
    }
    if post.author_id == user.id {
        return Err(AppError::BadRequest(
            "cannot mark your own post as helpful".into(),
        ));
    }
    let count = PostService::add_helpful(&state.db, post_id, user.id).await?;
    Ok((
        StatusCode::OK,
        Json(VoteCountResponse {
            count,
            viewer_voted: true,
        }),
    ))
}

async fn remove_helpful(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((category_slug, thread_slug, post_id)): Path<(String, String, i64)>,
) -> AppResult<(StatusCode, Json<VoteCountResponse>)> {
    let category = CategoryService::get_by_slug(&state.db, &category_slug).await?;
    let thread =
        ThreadService::get_by_category_and_slug(&state.db, category.id, &thread_slug).await?;
    let post = PostService::get_view(&state.db, post_id).await?;
    if post.thread_id != thread.id {
        return Err(AppError::NotFound);
    }
    let count = PostService::remove_helpful(&state.db, post_id, user.id).await?;
    Ok((
        StatusCode::OK,
        Json(VoteCountResponse {
            count,
            viewer_voted: false,
        }),
    ))
}
