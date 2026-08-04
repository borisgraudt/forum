#!/usr/bin/env bash
# Restore a SQLite backup. Stops the app first (you should).
# Usage: scripts/restore-sqlite.sh <backup.db> [target.db]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="${1:-}"
DEST="${2:-${DATABASE_FILE:-$ROOT/backend/forum.db}}"

if [[ -z "$SRC" || ! -f "$SRC" ]]; then
  echo "usage: $0 <backup.db> [target.db]" >&2
  exit 1
fi

mkdir -p "$(dirname "$DEST")"
# Drop live WAL so restore is clean
rm -f "${DEST}-wal" "${DEST}-shm"
cp -p "$SRC" "$DEST"
echo "restored $SRC → $DEST"
echo "restart the API (and re-run migrations only if needed)."
