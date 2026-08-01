-- Real view counters for threads (replaces UI-only placeholders).
ALTER TABLE threads ADD COLUMN view_count INTEGER NOT NULL DEFAULT 0 CHECK (view_count >= 0);
