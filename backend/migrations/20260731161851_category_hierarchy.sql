-- Categories can nest: root community → subcategories → threads live on any leaf (or any node).
-- parent_id NULL = top-level community (e.g. iPhone, Mac).

ALTER TABLE categories ADD COLUMN parent_id INTEGER REFERENCES categories (id) ON DELETE CASCADE;

CREATE INDEX idx_categories_parent_id ON categories (parent_id);
