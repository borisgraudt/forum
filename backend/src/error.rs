use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    // Reserved for CRUD handlers; keep variants stable for clients.
    #[allow(dead_code)]
    #[error("not found")]
    NotFound,
    #[error("unauthorized")]
    Unauthorized,
    #[allow(dead_code)]
    #[error("forbidden")]
    Forbidden,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized".into()),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "forbidden".into()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Internal(err) => {
                tracing::error!(error = %err, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".into(),
                )
            }
            AppError::Sqlx(err) => {
                if let sqlx::Error::Database(db_err) = err {
                    if db_err.is_unique_violation() {
                        return (
                            StatusCode::CONFLICT,
                            Json(ErrorBody {
                                error: "resource already exists".into(),
                            }),
                        )
                            .into_response();
                    }
                }
                tracing::error!(error = %err, "database error");
                (StatusCode::INTERNAL_SERVER_ERROR, "database error".into())
            }
        };

        (status, Json(ErrorBody { error: message })).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
