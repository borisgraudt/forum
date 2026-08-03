-- v0.5: media, avatars, link embeds cache, email tokens

ALTER TABLE users ADD COLUMN avatar_key TEXT;
ALTER TABLE users ADD COLUMN email_verified INTEGER NOT NULL DEFAULT 0;

CREATE TABLE attachments (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    -- Content-addressed key: "ab/abcdef...64.ext" under data_dir/uploads
    storage_key TEXT NOT NULL UNIQUE,
    thumb_key TEXT,
    original_name TEXT,
    mime TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    width INTEGER,
    height INTEGER,
    uploader_id INTEGER NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('avatar', 'post', 'other')),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    FOREIGN KEY (uploader_id) REFERENCES users (id) ON DELETE CASCADE
);

CREATE INDEX idx_attachments_uploader ON attachments (uploader_id, created_at DESC);

CREATE TABLE post_attachments (
    post_id INTEGER NOT NULL,
    attachment_id INTEGER NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (post_id, attachment_id),
    FOREIGN KEY (post_id) REFERENCES posts (id) ON DELETE CASCADE,
    FOREIGN KEY (attachment_id) REFERENCES attachments (id) ON DELETE CASCADE
);

CREATE INDEX idx_post_attachments_post ON post_attachments (post_id, sort_order);

-- Aggressive cache for link previews (OG). Failures cached briefly to avoid hammering.
CREATE TABLE link_embeds (
    url_hash TEXT PRIMARY KEY NOT NULL,
    url TEXT NOT NULL,
    title TEXT,
    description TEXT,
    image_url TEXT,
    site_name TEXT,
    status TEXT NOT NULL CHECK (status IN ('ok', 'failed', 'blocked')),
    fetched_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    expires_at TEXT NOT NULL
);

CREATE INDEX idx_link_embeds_expires ON link_embeds (expires_at);

CREATE TABLE email_tokens (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    user_id INTEGER NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('verify', 'reset')),
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TEXT NOT NULL,
    used_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
);

CREATE INDEX idx_email_tokens_user ON email_tokens (user_id, kind);
