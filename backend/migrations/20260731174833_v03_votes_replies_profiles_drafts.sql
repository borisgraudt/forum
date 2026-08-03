-- v0.3: votes, quote/reply-to, edits, soft-delete, profiles, drafts

ALTER TABLE users ADD COLUMN bio TEXT;

ALTER TABLE posts ADD COLUMN reply_to_post_id INTEGER
    REFERENCES posts (id) ON DELETE SET NULL;
ALTER TABLE posts ADD COLUMN deleted_at TEXT;
ALTER TABLE posts ADD COLUMN edited_at TEXT;

CREATE INDEX idx_posts_reply_to ON posts (reply_to_post_id);
CREATE INDEX idx_posts_deleted_at ON posts (thread_id, deleted_at);

CREATE TABLE post_edits (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    post_id INTEGER NOT NULL,
    editor_id INTEGER NOT NULL,
    body_before TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    FOREIGN KEY (post_id) REFERENCES posts (id) ON DELETE CASCADE,
    FOREIGN KEY (editor_id) REFERENCES users (id) ON DELETE RESTRICT
);

CREATE INDEX idx_post_edits_post_id ON post_edits (post_id, created_at DESC);

-- "Me too" on the original question (thread-level).
CREATE TABLE thread_me_too (
    thread_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (thread_id, user_id),
    FOREIGN KEY (thread_id) REFERENCES threads (id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
);

CREATE INDEX idx_thread_me_too_user ON thread_me_too (user_id);

-- "Helpful" on replies / posts.
CREATE TABLE post_helpful (
    post_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (post_id, user_id),
    FOREIGN KEY (post_id) REFERENCES posts (id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
);

CREATE INDEX idx_post_helpful_user ON post_helpful (user_id);

CREATE TABLE drafts (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    user_id INTEGER NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('thread', 'reply')),
    category_slug TEXT,
    thread_id INTEGER,
    title TEXT,
    body TEXT NOT NULL DEFAULT '',
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    FOREIGN KEY (thread_id) REFERENCES threads (id) ON DELETE CASCADE
);

CREATE INDEX idx_drafts_user ON drafts (user_id, updated_at DESC);
