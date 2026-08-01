pub mod auth;
pub mod forum;

pub use auth::{AuthResponse, LoginRequest, RegisterRequest};
pub use forum::{
    CategoryListResponse, CategoryResponse, CreateCategoryRequest, CreatePostRequest,
    CreateThreadRequest, CsrfResponse, ListQuery, PostListResponse, PostResponse, SearchHit,
    SearchQuery, SearchResponse, ThreadListResponse, ThreadResponse, UpdateThreadRequest,
};
