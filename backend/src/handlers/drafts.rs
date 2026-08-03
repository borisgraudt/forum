use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, get};
use axum::{Json, Router};
use validator::Validate;

use crate::dto::{DraftListResponse, DraftResponse, UpsertDraftRequest};
use crate::error::AppResult;
use crate::middleware::AuthUser;
use crate::services::DraftService;
use crate::state::AppState;
use crate::utils::validation_error;

pub fn drafts_router() -> Router<AppState> {
    Router::new()
        .route("/drafts", get(list_drafts).put(upsert_draft))
        .route("/drafts/{id}", delete(delete_draft))
}

async fn list_drafts(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> AppResult<(StatusCode, Json<DraftListResponse>)> {
    let drafts = DraftService::list_for_user(&state.db, user.id).await?;
    Ok((StatusCode::OK, Json(DraftListResponse { drafts })))
}

async fn upsert_draft(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(body): Json<UpsertDraftRequest>,
) -> AppResult<(StatusCode, Json<DraftResponse>)> {
    body.validate().map_err(validation_error)?;
    let draft = DraftService::upsert(
        &state.db,
        user.id,
        body.kind.trim(),
        body.category_slug.as_deref().map(str::trim).filter(|s| !s.is_empty()),
        body.thread_id,
        body.title.as_deref().map(str::trim).filter(|s| !s.is_empty()),
        body.body.trim(),
    )
    .await?;
    Ok((StatusCode::OK, Json(DraftResponse { draft })))
}

async fn delete_draft(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    DraftService::delete(&state.db, id, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}
