use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::models::{Category, Post, Thread};

#[derive(Debug, Deserialize, Validate)]
pub struct CreateCategoryRequest {
    #[validate(length(min = 1, max = 80, message = "name must be 1-80 characters"))]
    pub name: String,

    /// Optional; auto-generated from name when omitted.
    #[validate(length(min = 1, max = 80, message = "slug must be 1-80 characters"))]
    pub slug: Option<String>,

    #[validate(length(max = 500, message = "description is too long"))]
    pub description: Option<String>,

    pub sort_order: Option<i64>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateThreadRequest {
    #[validate(length(min = 1, max = 200, message = "title must be 1-200 characters"))]
    pub title: String,

    /// Optional; auto-generated from title when omitted.
    #[validate(length(min = 1, max = 80, message = "slug must be 1-80 characters"))]
    pub slug: Option<String>,

    /// First post body (required when creating a thread).
    #[validate(length(min = 1, max = 50_000, message = "body must be 1-50000 characters"))]
    pub body: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreatePostRequest {
    #[validate(length(min = 1, max = 50_000, message = "body must be 1-50000 characters"))]
    pub body: String,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl ListQuery {
    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(20).clamp(1, 100)
    }

    pub fn offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }
}

#[derive(Debug, Serialize)]
pub struct CategoryResponse {
    pub category: Category,
}

#[derive(Debug, Serialize)]
pub struct CategoryListResponse {
    pub categories: Vec<Category>,
}

#[derive(Debug, Serialize)]
pub struct ThreadResponse {
    pub thread: Thread,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_post: Option<Post>,
}

#[derive(Debug, Serialize)]
pub struct ThreadListResponse {
    pub threads: Vec<Thread>,
}

#[derive(Debug, Serialize)]
pub struct PostResponse {
    pub post: Post,
}

#[derive(Debug, Serialize)]
pub struct PostListResponse {
    pub posts: Vec<Post>,
}
