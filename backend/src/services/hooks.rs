//! Lightweight in-process extension hooks (plugin API v0).
//!
//! Not a full plugin system — just named events you can log or forward.
//! Wire custom logic here without forking handlers.

use std::sync::OnceLock;
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub enum ForumEvent {
    ThreadCreated {
        thread_id: i64,
        category_id: i64,
        author_id: i64,
    },
    PostCreated {
        post_id: i64,
        thread_id: i64,
        author_id: i64,
    },
    UserRegistered {
        user_id: i64,
        username: String,
    },
}

static BUS: OnceLock<broadcast::Sender<ForumEvent>> = OnceLock::new();

fn bus() -> &'static broadcast::Sender<ForumEvent> {
    BUS.get_or_init(|| {
        let (tx, _rx) = broadcast::channel(256);
        // Drop default receiver; subscribers call subscribe().
        tx
    })
}

/// Subscribe to forum events (for operators embedding custom handlers).
#[allow(dead_code)]
pub fn subscribe() -> broadcast::Receiver<ForumEvent> {
    bus().subscribe()
}

pub fn emit(event: ForumEvent) {
    match &event {
        ForumEvent::ThreadCreated {
            thread_id,
            category_id,
            author_id,
        } => tracing::debug!(thread_id, category_id, author_id, "hook.thread_created"),
        ForumEvent::PostCreated {
            post_id,
            thread_id,
            author_id,
        } => tracing::debug!(post_id, thread_id, author_id, "hook.post_created"),
        ForumEvent::UserRegistered { user_id, username } => {
            tracing::debug!(user_id, %username, "hook.user_registered")
        }
    }
    // Ignore "no receivers" errors — hooks are best-effort.
    let _ = bus().send(event);
}
