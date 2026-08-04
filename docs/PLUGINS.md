# Plugin / hooks API (v0)

Not a dynamic plugin loader. **In-process hooks** for operators who fork or embed.

## Events

| Event | When |
|-------|------|
| `UserRegistered` | After successful register |
| `ThreadCreated` | After first-post thread create |
| `PostCreated` | After reply (and first post of a thread) |

## Subscribe (Rust)

```rust
use forum_backend::services::hooks::{subscribe, ForumEvent};

let mut rx = subscribe();
tokio::spawn(async move {
    while let Ok(ev) = rx.recv().await {
        match ev {
            ForumEvent::PostCreated { post_id, .. } => {
                // e.g. push to webhook, index, etc.
                tracing::info!(post_id, "custom handler");
            }
            _ => {}
        }
    }
});
```

Emit is already wired in handlers via `hooks::emit`.

## Roadmap

Dynamic WASM/native plugins and ActivityPub federation are **post-1.0**.
