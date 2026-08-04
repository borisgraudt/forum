#!/usr/bin/env bash
# Online-safe SQLite backup (uses sqlite3 .backup API when available).
# Usage: scripts/backup-sqlite.sh [db_path] [out_dir]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DB="${1:-${DATABASE_FILE:-$ROOT/backend/forum.db}}"
OUT_DIR="${2:-${BACKUP_DIR:-$ROOT/backups}}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "$OUT_DIR"

if [[ ! -f "$DB" ]]; then
  echo "error: database not found: $DB" >&2
  exit 1
fi

DEST="$OUT_DIR/forum-$STAMP.db"

if command -v sqlite3 >/dev/null 2>&1; then
  # Consistent snapshot even under WAL writers.
  sqlite3 "$DB" ".backup '$DEST'"
else
  echo "warn: sqlite3 CLI missing — using file copy (stop writers for consistency)" >&2
  cp -p "$DB" "$DEST"
  # Best-effort WAL companions
  [[ -f "${DB}-wal" ]] && cp -p "${DB}-wal" "${DEST}-wal" || true
  [[ -f "${DB}-shm" ]] && cp -p "${DB}-shm" "${DEST}-shm" || true
fi

# Optional media tree
DATA_DIR="${DATA_DIR:-$ROOT/backend/data}"
if [[ -d "$DATA_DIR" ]]; then
  tar -C "$(dirname "$DATA_DIR")" -czf "$OUT_DIR/forum-data-$STAMP.tar.gz" "$(basename "$DATA_DIR")"
  echo "media: $OUT_DIR/forum-data-$STAMP.tar.gz"
fi

if command -v shasum >/dev/null 2>&1; then
  (cd "$OUT_DIR" && shasum -a 256 "$(basename "$DEST")" >"$(basename "$DEST").sha256")
elif command -v sha256sum >/dev/null 2>&1; then
  (cd "$OUT_DIR" && sha256sum "$(basename "$DEST")" >"$(basename "$DEST").sha256")
fi

echo "backup: $DEST"
ls -lh "$DEST"
