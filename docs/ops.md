# Ops runbook (v0.8)

## Production env checklist

Backend (`backend/.env`):

| Variable | Prod value |
|----------|------------|
| `FORUM_ENV` | `production` (refuses weak `JWT_SECRET`) |
| `JWT_SECRET` | 32+ random chars |
| `COOKIE_SECURE` | `true` (HTTPS only cookies) |
| `ENABLE_HSTS` | `true` (or leave default when cookie_secure) |
| `CORS_ORIGIN` | `https://forum.example.com` |
| `PUBLIC_ORIGIN` | same as public UI origin |
| `DATABASE_URL` | `sqlite:/var/lib/forum/forum.db?mode=rwc` |
| `DATA_DIR` | `/var/lib/forum/data` |
| `RUST_LOG` | `info,forum_backend=info` |

Frontend:

| Variable | Prod value |
|----------|------------|
| `PUBLIC_API_URL` | `https://forum.example.com/api/v1` (same-origin via reverse proxy) |

Put API + UI behind **Caddy** (`deploy/Caddyfile`) or **nginx** (`deploy/nginx.conf`).

## Backup

```bash
# DB (+ optional media tarball under backups/)
./scripts/backup-sqlite.sh /var/lib/forum/forum.db /var/backups/forum

# Cron example (daily 03:15 UTC)
# 15 3 * * * DATA_DIR=/var/lib/forum/data /opt/forum/scripts/backup-sqlite.sh /var/lib/forum/forum.db /var/backups/forum
```

## Restore

1. Stop API (and UI if needed).
2. `./scripts/restore-sqlite.sh /var/backups/forum/forum-YYYYMMDD.db /var/lib/forum/forum.db`
3. Restore media: `tar -C /var/lib/forum -xzf forum-data-….tar.gz` if used.
4. Start API; migrations are applied on boot.

## Seed demo data

```bash
cd backend && cargo run -- seed
# or: ./forum-backend seed
```

Creates `admin` / `password123`, community **General → Introductions**, and a welcome thread. **Change the password** before exposing production.

## Health & metrics

| URL | Purpose |
|-----|---------|
| `GET /health` / `/ready` | Liveness (DB ping) |
| `GET /metrics` | Prometheus text (`forum_users_total`, …) |

Load baseline:

```bash
./scripts/load-smoke.sh http://127.0.0.1:3000 100
```

## Smoke e2e

```bash
# API :3000 + UI :4321 running
make test-smoke
```
