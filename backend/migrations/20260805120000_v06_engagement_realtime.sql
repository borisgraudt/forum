-- v0.6: watch, notifications, unreads, solved

ALTER TABLE threads ADD COLUMN is_solved INTEGER NOT NULL DEFAULT 0
    CHECK (is_solved IN (0, 1));
ALTER TABLE threads ADD COLUMN accepted_post_id INTEGER
    REFERENCES posts (id) ON DELETE SET NULL;

CREATE INDEX idx_threads_solved ON threads (category_id, is_solved, last_post_at DESC);

CREATE TABLE watches (
    user_id INTEGER NOT NULL,
    target_type TEXT NOT NULL CHECK (target_type IN ('thread', 'category')),
    target_id INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (user_id, target_type, target_id),
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
);

CREATE INDEX idx_watches_target ON watches (target_type, target_id);

CREATE TABLE thread_reads (
    user_id INTEGER NOT NULL,
    thread_id INTEGER NOT NULL,
    last_read_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (user_id, thread_id),
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    FOREIGN KEY (thread_id) REFERENCES threads (id) ON DELETE CASCADE
);

CREATE TABLE notifications (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    user_id INTEGER NOT NULL,
    kind TEXT NOT NULL,
    actor_id INTEGER,
    thread_id INTEGER,
    post_id INTEGER,
    category_id INTEGER,
    body TEXT NOT NULL DEFAULT '',
    is_read INTEGER NOT NULL DEFAULT 0 CHECK (is_read IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    FOREIGN KEY (actor_id) REFERENCES users (id) ON DELETE SET NULL,
    FOREIGN KEY (thread_id) REFERENCES threads (id) ON DELETE CASCADE,
    FOREIGN KEY (post_id) REFERENCES posts (id) ON DELETE CASCADE
);

CREATE INDEX idx_notifications_user ON notifications (user_id, is_read, created_at DESC);
