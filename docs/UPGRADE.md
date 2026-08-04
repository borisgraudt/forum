# Upgrade path (0.x → 1.0)

## Always

1. **Backup** first: `make backup` or `scripts/backup-sqlite.sh`.
2. Pull release / rebuild images or `cargo build --release` + `npm run build`.
3. Start backend — **migrations run automatically** on boot (`sqlx migrate`).
4. Restart frontend (Astro Node).
5. Smoke: `GET /health`, login, open a thread, Me too / reply.

## Version notes

| From → To | Notes |
|-----------|--------|
| 0.5 → 0.6 | Engagement migration (watches, notifications, solved) |
| 0.6 → 0.7 | Frontend-only mobile/Lighthouse; no DB changes |
| 0.7 → 0.8 | Ops env vars (`FORUM_ENV`, HSTS); no required DB changes |
| 0.8 → 0.9 | Search filters, hooks, import CLI; no required DB changes |
| 0.9 → 1.0 | Freeze + docs; no schema break intended |

## Rollback

1. Stop services.
2. Restore DB from backup: `scripts/restore-sqlite.sh <file> <dest>`.
3. Deploy previous binary/UI.
4. Start services (do not re-apply newer migrations if rolling past a schema change — restore pre-migrate backup).

## Data import

```bash
forum-backend seed
forum-backend import docs/examples/import-sample.json
```

Author usernames in import JSON must already exist (e.g. after seed).
