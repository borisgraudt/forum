-- v0.4: reports, sanctions (ban/mute/timeout), audit log

CREATE TABLE reports (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    reporter_id INTEGER NOT NULL,
    target_type TEXT NOT NULL CHECK (target_type IN ('post', 'thread', 'user')),
    target_id INTEGER NOT NULL,
    reason TEXT NOT NULL,
    details TEXT,
    status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'resolved', 'dismissed')),
    resolved_by INTEGER,
    resolved_at TEXT,
    resolution_note TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    FOREIGN KEY (reporter_id) REFERENCES users (id) ON DELETE CASCADE,
    FOREIGN KEY (resolved_by) REFERENCES users (id) ON DELETE SET NULL
);

CREATE INDEX idx_reports_status ON reports (status, created_at DESC);
CREATE INDEX idx_reports_target ON reports (target_type, target_id);
CREATE INDEX idx_reports_reporter ON reports (reporter_id);

CREATE TABLE user_sanctions (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    user_id INTEGER NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('ban', 'mute', 'timeout')),
    reason TEXT,
    created_by INTEGER NOT NULL,
    starts_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    -- NULL ends_at = permanent (ban) or open-ended mute
    ends_at TEXT,
    is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES users (id) ON DELETE RESTRICT
);

CREATE INDEX idx_sanctions_user ON user_sanctions (user_id, is_active, kind);
CREATE INDEX idx_sanctions_active ON user_sanctions (is_active, ends_at);

CREATE TABLE audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    actor_id INTEGER,
    action TEXT NOT NULL,
    target_type TEXT,
    target_id INTEGER,
    meta TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    FOREIGN KEY (actor_id) REFERENCES users (id) ON DELETE SET NULL
);

CREATE INDEX idx_audit_created ON audit_log (created_at DESC);
CREATE INDEX idx_audit_actor ON audit_log (actor_id, created_at DESC);
CREATE INDEX idx_audit_action ON audit_log (action, created_at DESC);
