-- Foundation migration.
-- Domain tables (users, categories, threads, posts) land in a follow-up PR.
-- WAL is enabled at connection time in the application as well.
PRAGMA journal_mode = WAL;
