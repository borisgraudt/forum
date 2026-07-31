-- Foundation migration.
-- Domain tables live in later migrations; WAL is also set per-connection in app code.
PRAGMA journal_mode = WAL;
