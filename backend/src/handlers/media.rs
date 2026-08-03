use axum::body::Body;
use axum::extract::{Multipart, Path, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::dto::{AttachmentResponse, EmbedResponse};
use crate::error::{AppError, AppResult};
use crate::middleware::AuthUser;
use crate::models::AttachmentJson;
use crate::services::{EmbedService, MediaService, StorageService};
use crate::state::AppState;

pub fn media_router() -> Router<AppState> {
    Router::new()
        .route("/uploads", post(upload_image))
        .route("/me/avatar", post(upload_avatar))
        .route("/embeds", post(preview_embed))
}

pub fn media_public_router() -> Router<AppState> {
    Router::new().route("/media/{*key}", get(serve_media))
}

async fn upload_image(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    mut multipart: Multipart,
) -> AppResult<(StatusCode, Json<AttachmentResponse>)> {
    StorageService::ensure_dirs(&state.config.data_dir).await?;
    let (bytes, filename, declared) = read_first_file(&mut multipart).await?;
    let stored = StorageService::store_image(
        &state.config.data_dir,
        &bytes,
        declared.as_deref(),
        true,
        false,
    )
    .await?;
    let att =
        MediaService::insert(&state.db, &stored, user.id, "post", filename.as_deref()).await?;
    Ok((
        StatusCode::CREATED,
        Json(AttachmentResponse {
            attachment: AttachmentJson::from(att),
        }),
    ))
}

async fn upload_avatar(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    mut multipart: Multipart,
) -> AppResult<(StatusCode, Json<AttachmentResponse>)> {
    StorageService::ensure_dirs(&state.config.data_dir).await?;
    let (bytes, _name, declared) = read_first_file(&mut multipart).await?;
    let att = MediaService::set_avatar(
        &state.db,
        &state.config.data_dir,
        user.id,
        &bytes,
        declared.as_deref(),
    )
    .await?;
    Ok((
        StatusCode::OK,
        Json(AttachmentResponse {
            attachment: AttachmentJson::from(att),
        }),
    ))
}

#[derive(serde::Deserialize)]
struct EmbedReq {
    url: String,
}

async fn preview_embed(
    State(state): State<AppState>,
    AuthUser(_user): AuthUser,
    Json(body): Json<EmbedReq>,
) -> AppResult<(StatusCode, Json<EmbedResponse>)> {
    let embed = EmbedService::get_or_fetch(&state.db, body.url.trim()).await?;
    Ok((StatusCode::OK, Json(EmbedResponse { embed })))
}

async fn serve_media(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> AppResult<Response> {
    let path = StorageService::resolve_path(&state.config.data_dir, &key)?;
    let file = File::open(&path).await.map_err(|_| AppError::NotFound)?;
    let meta = file.metadata().await.map_err(|_| AppError::NotFound)?;
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let mime = match path.extension().and_then(|e| e.to_str()) {
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        _ => "application/octet-stream",
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .header(header::CONTENT_LENGTH, meta.len())
        .header(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        )
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .body(body)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("response build: {e}")))
}

async fn read_first_file(
    multipart: &mut Multipart,
) -> AppResult<(Vec<u8>, Option<String>, Option<String>)> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("multipart: {e}")))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name != "file" && name != "image" && name != "avatar" {
            continue;
        }
        let filename = field.file_name().map(|s| s.to_string());
        let content_type = field.content_type().map(|s| s.to_string());
        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(format!("read file: {e}")))?;
        return Ok((data.to_vec(), filename, content_type));
    }
    Err(AppError::BadRequest("file field required".into()))
}
