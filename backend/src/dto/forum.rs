use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::models::{
    AttachmentJson, AuditEntry, Category, Draft, NotificationView, PostEdit, PostViewJson, Report,
    ReportView, ThreadView, UserProfile, UserPublic, UserSanction, UserSanctionView,
};
use crate::services::embed::LinkEmbed;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateCategoryRequest {
    #[validate(length(min = 1, max = 80, message = "name must be 1-80 characters"))]
    pub name: String,

    #[validate(length(min = 1, max = 80, message = "slug must be 1-80 characters"))]
    pub slug: Option<String>,

    #[validate(length(max = 500, message = "description is too long"))]
    pub description: Option<String>,

    pub sort_order: Option<i64>,

    /// When set, creates a subcategory under this parent slug.
    pub parent_slug: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateThreadRequest {
    #[validate(length(min = 1, max = 200, message = "title must be 1-200 characters"))]
    pub title: String,

    #[validate(length(min = 1, max = 80, message = "slug must be 1-80 characters"))]
    pub slug: Option<String>,

    #[validate(length(min = 1, max = 50_000, message = "body must be 1-50000 characters"))]
    pub body: String,

    pub attachment_ids: Option<Vec<i64>>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreatePostRequest {
    #[validate(length(min = 1, max = 50_000, message = "body must be 1-50000 characters"))]
    pub body: String,
    pub reply_to_post_id: Option<i64>,
    /// Attachment ids previously uploaded by this user.
    pub attachment_ids: Option<Vec<i64>>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdatePostRequest {
    #[validate(length(min = 1, max = 50_000, message = "body must be 1-50000 characters"))]
    pub body: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateThreadRequest {
    pub is_locked: Option<bool>,
    pub is_pinned: Option<bool>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProfileRequest {
    #[validate(length(max = 64, message = "display_name is too long"))]
    pub display_name: Option<String>,
    /// Pass empty string to clear.
    #[validate(length(max = 500, message = "bio is too long"))]
    pub bio: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct AdminUpdateUserRequest {
    pub role: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct AdminUpdateCategoryRequest {
    #[validate(length(min = 1, max = 80, message = "name must be 1-80 characters"))]
    pub name: Option<String>,
    #[validate(length(max = 500, message = "description is too long"))]
    pub description: Option<String>,
    pub sort_order: Option<i64>,
    /// When true, clear description.
    pub clear_description: Option<bool>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpsertDraftRequest {
    #[validate(length(min = 1, max = 16))]
    pub kind: String,
    pub category_slug: Option<String>,
    pub thread_id: Option<i64>,
    #[validate(length(max = 200))]
    pub title: Option<String>,
    #[validate(length(max = 50_000))]
    pub body: String,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    /// activity | newest | unanswered | solved
    pub sort: Option<String>,
}

impl ListQuery {
    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(20).clamp(1, 100)
    }

    pub fn offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }

    pub fn sort(&self) -> &str {
        self.sort.as_deref().unwrap_or("activity")
    }
}

#[derive(Debug, Deserialize, Validate)]
pub struct SearchQuery {
    #[validate(length(min = 1, max = 200, message = "q must be 1-200 characters"))]
    pub q: String,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    /// Filter by author username (case-insensitive).
    pub author: Option<String>,
    /// Filter by category slug.
    pub category: Option<String>,
    /// ISO date `YYYY-MM-DD` — only threads with activity on/after this day.
    pub since: Option<String>,
}

impl SearchQuery {
    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(20).clamp(1, 50)
    }

    pub fn offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }
}

#[derive(Debug, Serialize)]
pub struct CategoryResponse {
    pub category: Category,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<Category>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<Category>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub viewer_watching: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct CategoryListResponse {
    pub categories: Vec<Category>,
}

#[derive(Debug, Serialize)]
pub struct ThreadResponse {
    pub thread: ThreadView,
    pub viewer_me_too: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_post: Option<PostViewJson>,
}

#[derive(Debug, Serialize)]
pub struct ThreadListResponse {
    pub threads: Vec<ThreadView>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Serialize)]
pub struct PostResponse {
    pub post: PostViewJson,
}

#[derive(Debug, Serialize)]
pub struct PostListResponse {
    pub posts: Vec<PostViewJson>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Serialize)]
pub struct VoteCountResponse {
    pub count: i64,
    pub viewer_voted: bool,
}

#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub thread: ThreadView,
    pub category_slug: String,
    pub category_name: String,
    pub post_id: i64,
    pub snippet: String,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchHit>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    pub q: String,
}

#[derive(Debug, Serialize)]
pub struct CsrfResponse {
    pub csrf_token: String,
}

#[derive(Debug, Serialize)]
pub struct UserListResponse {
    pub users: Vec<UserPublic>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Serialize)]
pub struct ProfileResponse {
    pub profile: UserProfile,
}

#[derive(Debug, Serialize)]
pub struct DraftListResponse {
    pub drafts: Vec<Draft>,
}

#[derive(Debug, Serialize)]
pub struct DraftResponse {
    pub draft: Draft,
}

#[derive(Debug, Serialize)]
pub struct PostEditListResponse {
    pub edits: Vec<PostEdit>,
}

#[derive(Debug, Serialize)]
pub struct AttachmentResponse {
    pub attachment: AttachmentJson,
}

#[derive(Debug, Serialize)]
pub struct EmbedResponse {
    pub embed: LinkEmbed,
}

#[derive(Debug, Deserialize, Validate)]
pub struct WatchRequest {
    #[validate(length(min = 1, max = 16))]
    pub target_type: String,
    pub target_id: i64,
}

#[derive(Debug, Serialize)]
pub struct WatchStatusResponse {
    pub watching: bool,
}

#[derive(Debug, Deserialize)]
pub struct SolveThreadRequest {
    pub is_solved: bool,
    pub accepted_post_id: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct NotificationListResponse {
    pub notifications: Vec<NotificationView>,
    pub total: i64,
    pub unread: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Deserialize)]
pub struct MarkReadRequest {
    /// Empty / omitted = mark all.
    pub ids: Option<Vec<i64>>,
}

#[derive(Debug, Serialize)]
pub struct UnreadCountResponse {
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct ThreadPulseResponse {
    pub post_count: i64,
    pub me_too_count: i64,
    pub view_count: i64,
    pub is_solved: bool,
    pub last_post_at: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct PasswordResetRequest {
    #[validate(length(min = 3, max = 254))]
    pub login: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct PasswordResetConfirm {
    #[validate(length(min = 16, max = 128))]
    pub token: String,
    #[validate(length(min = 8, max = 128))]
    pub password: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct EmailVerifyConfirm {
    #[validate(length(min = 16, max = 128))]
    pub token: String,
}

// ── v0.4 moderation ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate)]
pub struct CreateReportRequest {
    #[validate(length(min = 1, max = 16))]
    pub target_type: String,
    pub target_id: i64,
    #[validate(length(min = 1, max = 200))]
    pub reason: String,
    #[validate(length(max = 2000))]
    pub details: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ResolveReportRequest {
    #[validate(length(min = 1, max = 16))]
    pub status: String,
    #[validate(length(max = 1000))]
    pub note: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateSanctionRequest {
    #[validate(length(min = 1, max = 16))]
    pub kind: String,
    #[validate(length(max = 500))]
    pub reason: Option<String>,
    /// ISO-8601 UTC end time. Required for timeout; optional for mute/ban.
    pub ends_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReportListQuery {
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl ReportListQuery {
    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(30).clamp(1, 100)
    }
    pub fn offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }
}

#[derive(Debug, Serialize)]
pub struct ReportResponse {
    pub report: Report,
}

#[derive(Debug, Serialize)]
pub struct ReportListResponse {
    pub reports: Vec<ReportView>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Serialize)]
pub struct SanctionResponse {
    pub sanction: UserSanction,
}

#[derive(Debug, Serialize)]
pub struct SanctionListResponse {
    pub sanctions: Vec<UserSanctionView>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Serialize)]
pub struct AuditListResponse {
    pub entries: Vec<AuditEntry>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}
