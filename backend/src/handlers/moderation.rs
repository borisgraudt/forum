use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, patch, post};
use axum::{Json, Router};
use validator::Validate;

use crate::dto::{
    AuditListResponse, CreateReportRequest, CreateSanctionRequest, ListQuery, ReportListQuery,
    ReportListResponse, ReportResponse, ResolveReportRequest, SanctionListResponse,
    SanctionResponse,
};
use crate::error::{AppError, AppResult};
use crate::middleware::AuthUser;
use crate::models::{SanctionKind, User};
use crate::services::ModerationService;
use crate::state::AppState;
use crate::utils::validation_error;

pub fn moderation_router() -> Router<AppState> {
    Router::new()
        .route("/reports", post(create_report).get(list_reports))
        .route("/reports/{report_id}", patch(resolve_report))
        .route("/sanctions", get(list_sanctions))
        .route("/users/{user_id}/sanctions", post(create_sanction))
        .route("/sanctions/{sanction_id}", delete(lift_sanction))
        .route("/audit", get(list_audit))
}

async fn create_report(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(body): Json<CreateReportRequest>,
) -> AppResult<(StatusCode, Json<ReportResponse>)> {
    body.validate().map_err(validation_error)?;
    let report = ModerationService::create_report(
        &state.db,
        user.id,
        body.target_type.trim(),
        body.target_id,
        body.reason.trim(),
        body.details
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty()),
    )
    .await?;

    let _ = ModerationService::audit(
        &state.db,
        Some(user.id),
        "report.create",
        Some(report.target_type.as_str()),
        Some(report.target_id),
        Some(&format!(
            r#"{{"report_id":{},"reason":{}}}"#,
            report.id,
            serde_json::to_string(&report.reason).unwrap_or_default()
        )),
    )
    .await;

    Ok((StatusCode::CREATED, Json(ReportResponse { report })))
}

async fn list_reports(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Query(query): Query<ReportListQuery>,
) -> AppResult<(StatusCode, Json<ReportListResponse>)> {
    require_staff(&user)?;
    let limit = query.limit();
    let offset = query.offset();
    let (reports, total) =
        ModerationService::list_reports(&state.db, query.status.as_deref(), limit, offset).await?;
    Ok((
        StatusCode::OK,
        Json(ReportListResponse {
            reports,
            total,
            limit,
            offset,
        }),
    ))
}

async fn resolve_report(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(report_id): Path<i64>,
    Json(body): Json<ResolveReportRequest>,
) -> AppResult<(StatusCode, Json<ReportResponse>)> {
    require_staff(&user)?;
    body.validate().map_err(validation_error)?;
    let report = ModerationService::resolve_report(
        &state.db,
        report_id,
        user.id,
        body.status.trim(),
        body.note
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty()),
    )
    .await?;

    let _ = ModerationService::audit(
        &state.db,
        Some(user.id),
        "report.resolve",
        Some("report"),
        Some(report.id),
        Some(&format!(r#"{{"status":"{}"}}"#, report.status)),
    )
    .await;

    Ok((StatusCode::OK, Json(ReportResponse { report })))
}

async fn create_sanction(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(user_id): Path<i64>,
    Json(body): Json<CreateSanctionRequest>,
) -> AppResult<(StatusCode, Json<SanctionResponse>)> {
    require_staff(&user)?;
    body.validate().map_err(validation_error)?;

    let kind = SanctionKind::parse(body.kind.trim())
        .ok_or_else(|| AppError::BadRequest("kind must be ban, mute, or timeout".into()))?;

    if kind == SanctionKind::Timeout && body.ends_at.is_none() {
        return Err(AppError::BadRequest("timeout requires ends_at".into()));
    }

    // Only admins can ban.
    if kind == SanctionKind::Ban {
        require_admin(&user)?;
    }

    let ends_at = body
        .ends_at
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let sanction = ModerationService::create_sanction(
        &state.db,
        user_id,
        kind,
        body.reason
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty()),
        user.id,
        ends_at,
    )
    .await?;

    let _ = ModerationService::audit(
        &state.db,
        Some(user.id),
        &format!("sanction.{}", kind.as_str()),
        Some("user"),
        Some(user_id),
        Some(&format!(
            r#"{{"sanction_id":{},"ends_at":{}}}"#,
            sanction.id,
            ends_at
                .map(|s| format!("\"{s}\""))
                .unwrap_or_else(|| "null".into())
        )),
    )
    .await;

    Ok((StatusCode::CREATED, Json(SanctionResponse { sanction })))
}

async fn lift_sanction(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(sanction_id): Path<i64>,
) -> AppResult<(StatusCode, Json<SanctionResponse>)> {
    require_staff(&user)?;
    let sanction = ModerationService::lift_sanction(&state.db, sanction_id).await?;

    let _ = ModerationService::audit(
        &state.db,
        Some(user.id),
        "sanction.lift",
        Some("user"),
        Some(sanction.user_id),
        Some(&format!(r#"{{"sanction_id":{}}}"#, sanction.id)),
    )
    .await;

    Ok((StatusCode::OK, Json(SanctionResponse { sanction })))
}

async fn list_sanctions(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Query(query): Query<ListQuery>,
) -> AppResult<(StatusCode, Json<SanctionListResponse>)> {
    require_staff(&user)?;
    let limit = query.limit();
    let offset = query.offset();
    let (sanctions, total) =
        ModerationService::list_sanctions(&state.db, true, limit, offset).await?;
    Ok((
        StatusCode::OK,
        Json(SanctionListResponse {
            sanctions,
            total,
            limit,
            offset,
        }),
    ))
}

async fn list_audit(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Query(query): Query<ListQuery>,
) -> AppResult<(StatusCode, Json<AuditListResponse>)> {
    // Audit is admin-only.
    require_admin(&user)?;
    let limit = query.limit();
    let offset = query.offset();
    let (entries, total) = ModerationService::list_audit(&state.db, limit, offset).await?;
    Ok((
        StatusCode::OK,
        Json(AuditListResponse {
            entries,
            total,
            limit,
            offset,
        }),
    ))
}

fn require_staff(user: &User) -> AppResult<()> {
    match user.role_enum() {
        Ok(r) if r.is_staff() => Ok(()),
        _ => Err(AppError::Forbidden),
    }
}

fn require_admin(user: &User) -> AppResult<()> {
    match user.role_enum() {
        Ok(r) if r.is_admin() => Ok(()),
        _ => Err(AppError::Forbidden),
    }
}
