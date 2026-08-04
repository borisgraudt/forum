use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::stream::{self, Stream};
use std::convert::Infallible;
use std::time::Duration;
use validator::Validate;

use crate::dto::{
    ListQuery, MarkReadRequest, NotificationListResponse, SolveThreadRequest, ThreadPulseResponse,
    ThreadResponse, UnreadCountResponse, WatchRequest, WatchStatusResponse,
};
use crate::error::{AppError, AppResult};
use crate::middleware::AuthUser;
use crate::services::{CategoryService, EngagementService, ThreadService};
use crate::state::AppState;
use crate::utils::validation_error;

pub fn engagement_router() -> Router<AppState> {
    Router::new()
        .route("/watches", post(add_watch).delete(remove_watch))
        .route(
            "/categories/{category_slug}/threads/{thread_slug}/watch",
            get(thread_watch_status)
                .post(watch_thread)
                .delete(unwatch_thread),
        )
        .route(
            "/categories/{category_slug}/threads/{thread_slug}/solve",
            post(solve_thread),
        )
        .route(
            "/categories/{category_slug}/threads/{thread_slug}/pulse",
            get(thread_pulse),
        )
        .route(
            "/categories/{category_slug}/threads/{thread_slug}/events",
            get(thread_events_sse),
        )
        .route("/notifications", get(list_notifications).post(mark_read))
        .route("/notifications/unread-count", get(unread_count))
}

async fn add_watch(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(body): Json<WatchRequest>,
) -> AppResult<(StatusCode, Json<WatchStatusResponse>)> {
    body.validate().map_err(validation_error)?;
    EngagementService::watch(&state.db, user.id, body.target_type.trim(), body.target_id).await?;
    Ok((StatusCode::OK, Json(WatchStatusResponse { watching: true })))
}

async fn remove_watch(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(body): Json<WatchRequest>,
) -> AppResult<(StatusCode, Json<WatchStatusResponse>)> {
    body.validate().map_err(validation_error)?;
    EngagementService::unwatch(&state.db, user.id, body.target_type.trim(), body.target_id).await?;
    Ok((
        StatusCode::OK,
        Json(WatchStatusResponse { watching: false }),
    ))
}

async fn watch_thread(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((category_slug, thread_slug)): Path<(String, String)>,
) -> AppResult<(StatusCode, Json<WatchStatusResponse>)> {
    let category = CategoryService::get_by_slug(&state.db, &category_slug).await?;
    let thread =
        ThreadService::get_by_category_and_slug(&state.db, category.id, &thread_slug).await?;
    EngagementService::watch(&state.db, user.id, "thread", thread.id).await?;
    Ok((StatusCode::OK, Json(WatchStatusResponse { watching: true })))
}

async fn unwatch_thread(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((category_slug, thread_slug)): Path<(String, String)>,
) -> AppResult<(StatusCode, Json<WatchStatusResponse>)> {
    let category = CategoryService::get_by_slug(&state.db, &category_slug).await?;
    let thread =
        ThreadService::get_by_category_and_slug(&state.db, category.id, &thread_slug).await?;
    EngagementService::unwatch(&state.db, user.id, "thread", thread.id).await?;
    Ok((
        StatusCode::OK,
        Json(WatchStatusResponse { watching: false }),
    ))
}

async fn thread_watch_status(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((category_slug, thread_slug)): Path<(String, String)>,
) -> AppResult<(StatusCode, Json<WatchStatusResponse>)> {
    let category = CategoryService::get_by_slug(&state.db, &category_slug).await?;
    let thread =
        ThreadService::get_by_category_and_slug(&state.db, category.id, &thread_slug).await?;
    let watching = EngagementService::is_watching(&state.db, user.id, "thread", thread.id).await?;
    Ok((StatusCode::OK, Json(WatchStatusResponse { watching })))
}

async fn solve_thread(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((category_slug, thread_slug)): Path<(String, String)>,
    Json(body): Json<SolveThreadRequest>,
) -> AppResult<(StatusCode, Json<ThreadResponse>)> {
    let category = CategoryService::get_by_slug(&state.db, &category_slug).await?;
    let thread =
        ThreadService::get_by_category_and_slug(&state.db, category.id, &thread_slug).await?;

    let is_staff = user.role_enum().map(|r| r.is_staff()).unwrap_or(false);
    if thread.author_id != user.id && !is_staff {
        return Err(AppError::Forbidden);
    }

    let thread = ThreadService::set_solved(
        &state.db,
        thread.id,
        body.is_solved,
        if body.is_solved {
            body.accepted_post_id
        } else {
            None
        },
    )
    .await?;

    Ok((
        StatusCode::OK,
        Json(ThreadResponse {
            thread,
            viewer_me_too: false,
            first_post: None,
        }),
    ))
}

/// Ultra-light poll endpoint for live counts (no HTML, ~200B JSON).
async fn thread_pulse(
    State(state): State<AppState>,
    Path((category_slug, thread_slug)): Path<(String, String)>,
) -> AppResult<axum::response::Response> {
    use axum::response::IntoResponse;
    let category = CategoryService::get_by_slug(&state.db, &category_slug).await?;
    let thread =
        ThreadService::get_by_category_and_slug(&state.db, category.id, &thread_slug).await?;
    let body = ThreadPulseResponse {
        post_count: thread.post_count,
        me_too_count: thread.me_too_count,
        view_count: thread.view_count,
        is_solved: thread.is_solved,
        last_post_at: thread.last_post_at,
    };
    // Short private cache so parallel tabs coalesce; still feels live.
    Ok((
        StatusCode::OK,
        [
            (
                axum::http::header::CACHE_CONTROL,
                "private, max-age=1, stale-while-revalidate=2",
            ),
            (axum::http::header::CONTENT_TYPE, "application/json"),
        ],
        axum::Json(body),
    )
        .into_response())
}

/// SSE stream: emits pulse JSON when post_count / me_too / solved change.
async fn thread_events_sse(
    State(state): State<AppState>,
    Path((category_slug, thread_slug)): Path<(String, String)>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    let category = CategoryService::get_by_slug(&state.db, &category_slug).await?;
    let thread =
        ThreadService::get_by_category_and_slug(&state.db, category.id, &thread_slug).await?;
    let thread_id = thread.id;
    let db = state.db.clone();

    let stream = stream::unfold(
        (
            db,
            thread_id,
            thread.post_count,
            thread.me_too_count,
            thread.is_solved,
        ),
        |(db, thread_id, prev_posts, prev_me, prev_solved)| async move {
            tokio::time::sleep(Duration::from_millis(1200)).await;
            let Ok(t) = ThreadService::get_view_by_id(&db, thread_id).await else {
                return None;
            };
            let changed = t.post_count != prev_posts
                || t.me_too_count != prev_me
                || t.is_solved != prev_solved;
            let payload = ThreadPulseResponse {
                post_count: t.post_count,
                me_too_count: t.me_too_count,
                view_count: t.view_count,
                is_solved: t.is_solved,
                last_post_at: t.last_post_at.clone(),
            };
            let data = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".into());
            let event = if changed {
                Event::default().event("pulse").data(data)
            } else {
                Event::default().event("ping").data("1")
            };
            Some((
                Ok(event),
                (db, thread_id, t.post_count, t.me_too_count, t.is_solved),
            ))
        },
    );

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

async fn list_notifications(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Query(query): Query<ListQuery>,
) -> AppResult<(StatusCode, Json<NotificationListResponse>)> {
    let limit = query.limit();
    let offset = query.offset();
    let unread_only = query.sort() == "unread";
    let (notifications, total) =
        EngagementService::list_notifications(&state.db, user.id, limit, offset, unread_only)
            .await?;
    let unread = EngagementService::unread_count(&state.db, user.id).await?;
    Ok((
        StatusCode::OK,
        Json(NotificationListResponse {
            notifications,
            total,
            unread,
            limit,
            offset,
        }),
    ))
}

async fn mark_read(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(body): Json<MarkReadRequest>,
) -> AppResult<(StatusCode, Json<UnreadCountResponse>)> {
    let ids = body.ids.unwrap_or_default();
    EngagementService::mark_read(&state.db, user.id, &ids).await?;
    let count = EngagementService::unread_count(&state.db, user.id).await?;
    Ok((StatusCode::OK, Json(UnreadCountResponse { count })))
}

async fn unread_count(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> AppResult<(StatusCode, Json<UnreadCountResponse>)> {
    let count = EngagementService::unread_count(&state.db, user.id).await?;
    Ok((StatusCode::OK, Json(UnreadCountResponse { count })))
}
