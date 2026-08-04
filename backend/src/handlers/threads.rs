use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use validator::Validate;

use crate::dto::{
    CreateThreadRequest, ListQuery, ThreadListResponse, ThreadResponse, UpdateThreadRequest,
    VoteCountResponse,
};
use crate::error::{AppError, AppResult};
use crate::middleware::{AuthUser, OptionalAuthUser};
use crate::models::{PostViewJson, UserRole};
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
            get(get_thread).patch(update_thread),
        )
        .route(
            "/categories/{category_slug}/threads/{thread_slug}/me-too",
            post(add_me_too).delete(remove_me_too),
        )
}

async fn list_threads(
    State(state): State<AppState>,
    OptionalAuthUser(viewer): OptionalAuthUser,
    Path(category_slug): Path<String>,
    Query(query): Query<ListQuery>,
) -> AppResult<(StatusCode, Json<ThreadListResponse>)> {
    let category = CategoryService::get_by_slug(&state.db, &category_slug).await?;
    let limit = query.limit();
    let offset = query.offset();
    let sort = query.sort();
    let total = ThreadService::count_by_category_sorted(&state.db, category.id, sort).await?;
    let mut threads =
        ThreadService::list_by_category(&state.db, category.id, limit, offset, sort).await?;

    // Batch-annotate watching/unread (one query each — no N+1).
    if let Some(u) = viewer {
        use crate::services::EngagementService;
        let ids: Vec<i64> = threads.iter().map(|t| t.id).collect();
        let watching = EngagementService::watching_set(&state.db, u.id, "thread", &ids).await?;
        let reads = EngagementService::read_cursors(&state.db, u.id, &ids).await?;
        for t in &mut threads {
            let is_w = watching.contains(&t.id);
            t.viewer_watching = Some(is_w);
            if is_w {
                let unread = match (t.last_post_at.as_deref(), reads.get(&t.id)) {
                    (Some(last), Some(read_at)) => last > read_at.as_str(),
                    (Some(_), None) => true,
                    _ => false,
                };
                t.is_unread = Some(unread);
            }
        }
    }

    Ok((
        StatusCode::OK,
        Json(ThreadListResponse {
            threads,
            total,
            limit,
            offset,
        }),
    ))
}

async fn get_thread(
    State(state): State<AppState>,
    OptionalAuthUser(viewer): OptionalAuthUser,
    Path((category_slug, thread_slug)): Path<(String, String)>,
) -> AppResult<(StatusCode, Json<ThreadResponse>)> {
    let category = CategoryService::get_by_slug(&state.db, &category_slug).await?;
    let thread =
        ThreadService::get_by_category_and_slug(&state.db, category.id, &thread_slug).await?;
    ThreadService::increment_views(&state.db, thread.id).await?;
    let mut thread =
        ThreadService::get_by_category_and_slug(&state.db, category.id, &thread_slug).await?;
    let viewer_me_too = if let Some(ref u) = viewer {
        // Mark read — keeps unreads accurate without extra round-trip.
        let _ =
            crate::services::EngagementService::mark_thread_read(&state.db, u.id, thread.id).await;
        let watching =
            crate::services::EngagementService::is_watching(&state.db, u.id, "thread", thread.id)
                .await?;
        thread.viewer_watching = Some(watching);
        ThreadService::viewer_me_too(&state.db, thread.id, u.id).await?
    } else {
        false
    };
    Ok((
        StatusCode::OK,
        Json(ThreadResponse {
            thread,
            viewer_me_too,
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
    crate::services::ModerationService::ensure_can_post(&state.db, user.id).await?;

    let category = CategoryService::get_by_slug(&state.db, &category_slug).await?;
    let title = body.title.trim().to_string();
    if title.is_empty() {
        return Err(AppError::BadRequest("title is required".into()));
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
        return Err(AppError::BadRequest("body is required".into()));
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

    crate::services::hooks::emit(crate::services::hooks::ForumEvent::ThreadCreated {
        thread_id: thread.id,
        category_id: category.id,
        author_id: user.id,
    });
    crate::services::hooks::emit(crate::services::hooks::ForumEvent::PostCreated {
        post_id: first_post.id,
        thread_id: thread.id,
        author_id: user.id,
    });

    if let Some(ids) = body.attachment_ids.as_ref() {
        if !ids.is_empty() {
            crate::services::MediaService::link_to_post(&state.db, first_post.id, ids, user.id)
                .await?;
        }
    }

    // Auto-watch own thread so OP gets reply alerts (off hot path).
    let db = state.db.clone();
    let body_for_embed = body_text.clone();
    let thread_id = thread.id;
    let author_id = user.id;
    tokio::spawn(async move {
        for url in crate::services::EmbedService::extract_urls(&body_for_embed, 3) {
            let _ = crate::services::EmbedService::get_or_fetch(&db, &url).await;
        }
        let _ =
            crate::services::EngagementService::watch(&db, author_id, "thread", thread_id).await;
    });

    Ok((
        StatusCode::CREATED,
        Json(ThreadResponse {
            thread,
            viewer_me_too: false,
            first_post: Some(PostViewJson::from(first_post)),
        }),
    ))
}

async fn update_thread(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((category_slug, thread_slug)): Path<(String, String)>,
    Json(body): Json<UpdateThreadRequest>,
) -> AppResult<(StatusCode, Json<ThreadResponse>)> {
    require_moderator(&user)?;

    let category = CategoryService::get_by_slug(&state.db, &category_slug).await?;
    let thread =
        ThreadService::get_by_category_and_slug(&state.db, category.id, &thread_slug).await?;

    let thread =
        ThreadService::set_flags(&state.db, thread.id, body.is_locked, body.is_pinned).await?;

    Ok((
        StatusCode::OK,
        Json(ThreadResponse {
            thread,
            viewer_me_too: false,
            first_post: None,
        }),
    ))
}

async fn add_me_too(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((category_slug, thread_slug)): Path<(String, String)>,
) -> AppResult<(StatusCode, Json<VoteCountResponse>)> {
    let category = CategoryService::get_by_slug(&state.db, &category_slug).await?;
    let thread =
        ThreadService::get_by_category_and_slug(&state.db, category.id, &thread_slug).await?;
    if thread.author_id == user.id {
        return Err(AppError::BadRequest(
            "cannot Me too your own question".into(),
        ));
    }
    let count = ThreadService::add_me_too(&state.db, thread.id, user.id).await?;
    Ok((
        StatusCode::OK,
        Json(VoteCountResponse {
            count,
            viewer_voted: true,
        }),
    ))
}

async fn remove_me_too(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((category_slug, thread_slug)): Path<(String, String)>,
) -> AppResult<(StatusCode, Json<VoteCountResponse>)> {
    let category = CategoryService::get_by_slug(&state.db, &category_slug).await?;
    let thread =
        ThreadService::get_by_category_and_slug(&state.db, category.id, &thread_slug).await?;
    let count = ThreadService::remove_me_too(&state.db, thread.id, user.id).await?;
    Ok((
        StatusCode::OK,
        Json(VoteCountResponse {
            count,
            viewer_voted: false,
        }),
    ))
}

fn require_moderator(user: &crate::models::User) -> AppResult<()> {
    match user.role_enum() {
        Ok(UserRole::Moderator) | Ok(UserRole::Admin) => Ok(()),
        _ => Err(AppError::Forbidden),
    }
}
