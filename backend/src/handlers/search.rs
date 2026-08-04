use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use validator::Validate;

use crate::dto::{SearchQuery, SearchResponse};
use crate::error::AppResult;
use crate::services::SearchService;
use crate::state::AppState;
use crate::utils::validation_error;

pub fn search_router() -> Router<AppState> {
    Router::new().route("/search", get(search))
}

async fn search(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> AppResult<(StatusCode, Json<SearchResponse>)> {
    query.validate().map_err(validation_error)?;
    let q = query.q.trim().to_string();
    let limit = query.limit();
    let offset = query.offset();
    let filters = crate::services::search::SearchFilters {
        author: query.author.clone(),
        category: query.category.clone(),
        since: query.since.clone(),
    };
    let (results, total) = SearchService::search(&state.db, &q, limit, offset, &filters).await?;
    Ok((
        StatusCode::OK,
        Json(SearchResponse {
            results,
            total,
            limit,
            offset,
            q,
        }),
    ))
}
