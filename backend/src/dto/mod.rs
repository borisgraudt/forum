pub mod auth;
pub mod forum;

pub use auth::{AuthResponse, LoginRequest, RegisterRequest};
pub use forum::{
    CategoryListResponse, CategoryResponse, CreateCategoryRequest, CreatePostRequest,
    CreateThreadRequest, ListQuery, PostListResponse, PostResponse, ThreadListResponse,
    ThreadResponse,
};
