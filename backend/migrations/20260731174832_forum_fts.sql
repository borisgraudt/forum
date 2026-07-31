-- Full-text search over thread titles + post bodies (SQLite FTS5).
CREATE VIRTUAL TABLE forum_fts USING fts5(
    title,
    body,
    thread_id UNINDEXED,
    post_id UNINDEXED,
    category_id UNINDEXED,
    tokenize = 'porter unicode61'
);

-- Backfill existing content.
INSERT INTO forum_fts (title, body, thread_id, post_id, category_id)
SELECT t.title, p.body, t.id, p.id, t.category_id
FROM posts p
INNER JOIN threads t ON t.id = p.thread_id;

-- Keep FTS in sync with posts / thread title changes.
CREATE TRIGGER posts_ai_fts AFTER INSERT ON posts BEGIN
    INSERT INTO forum_fts (title, body, thread_id, post_id, category_id)
    SELECT t.title, NEW.body, t.id, NEW.id, t.category_id
    FROM threads t
    WHERE t.id = NEW.thread_id;
END;

CREATE TRIGGER posts_ad_fts AFTER DELETE ON posts BEGIN
    DELETE FROM forum_fts WHERE post_id = OLD.id;
END;

CREATE TRIGGER posts_au_fts AFTER UPDATE OF body ON posts BEGIN
    DELETE FROM forum_fts WHERE post_id = OLD.id;
    INSERT INTO forum_fts (title, body, thread_id, post_id, category_id)
    SELECT t.title, NEW.body, t.id, NEW.id, t.category_id
    FROM threads t
    WHERE t.id = NEW.thread_id;
END;

CREATE TRIGGER threads_au_title_fts AFTER UPDATE OF title ON threads BEGIN
    UPDATE forum_fts SET title = NEW.title WHERE thread_id = NEW.id;
END;

CREATE TRIGGER threads_ad_fts AFTER DELETE ON threads BEGIN
    DELETE FROM forum_fts WHERE thread_id = OLD.id;
END;
