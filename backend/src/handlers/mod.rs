pub mod auth;
pub mod categories;
pub mod posts;
pub mod threads;

pub use auth::auth_router;
pub use categories::categories_router;
pub use posts::posts_router;
pub use threads::threads_router;
