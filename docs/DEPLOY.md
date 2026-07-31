# Deploy & Ops

UltraForum is designed for simple self-hosting: **one Rust binary + SQLite file** for the API, plus an **Astro Node server** for the UI.

## Quick paths

| Mode | Command | When |
|------|---------|------|
| Local dev | `make dev` | day-to-day coding |
| Release **Assets** (tar.gz) | `make package` / GitHub Release | downloadable archive on a Release |
| **GitHub Packages** (GHCR) | tag `v*` or push to `develop` | container images next to the repo |
| Docker (local build) | `make docker-up` | compose profile `app` |
| GitHub pre-release | tag `vX.Y.Z-alpha.N` | Assets + Packages |

### Releases vs Packages vs Tags (GitHub UI)

| Thing | What it is |
|-------|------------|
| **Tag** | Git pointer (`v0.1.0-alpha.1`). Repo shows “1 tag” after the first one. |
| **Release** | Notes + optional **Assets** (our `.tar.gz`). Always tied to **one tag**. |
| **GitHub Packages** | Separate registry (here: **GHCR** Docker images). Shown under the repo **Packages** tab, not as a “tag”. |

So “1 tag” after the first pre-release is **normal** — not a bug.

## Environment

### Backend (`backend/.env`)

| Variable | Example | Notes |
|----------|---------|--------|
| `HOST` | `0.0.0.0` | bind address |
| `PORT` | `3000` | API port |
| `DATABASE_URL` | `sqlite:/data/forum.db?mode=rwc` | single file DB |
| `JWT_SECRET` | long random string | **required in prod** (≥16 chars) |
| `JWT_TTL_SECS` | `604800` | 7 days default |
| `COOKIE_SECURE` | `true` | set `true` behind HTTPS |
| `CORS_ORIGIN` | `https://forum.example.com` | exact UI origin |
| `RUST_LOG` | `info,forum_backend=info` | tracing filter |

### Frontend (`frontend/.env` / build arg)

| Variable | Example |
|----------|---------|
| `PUBLIC_API_URL` | `https://api.example.com/api/v1` |
| `HOST` / `PORT` | `0.0.0.0` / `4321` |

Migrations run **automatically** on backend start (embedded via SQLx).

## Release package (tarball)

```bash
make package
# → dist/forum-0.1.0-alpha.1-<target>.tar.gz
# → dist/forum-0.1.0-alpha.1-<target>.tar.gz.sha256
```

Unpack and run:

```bash
tar -xzf forum-0.1.0-alpha.1-*.tar.gz
cd forum-0.1.0-alpha.1-*
# edit backend/.env (created on first run)
./run-backend.sh   # :3000
./run-frontend.sh  # :4321
```

## GitHub Packages (one package: **forum**)

There is a **single** package named **`forum`**:

```text
ghcr.io/borisgraudt/forum
```

Backend and UI are **tags** of that package (not two packages):

| Role | Tags |
|------|------|
| API | `backend`, `backend-edge`, `backend-0.1.0-alpha.1`, `backend-sha-…` |
| UI | `frontend`, `frontend-edge`, `frontend-0.1.0-alpha.1`, `frontend-sha-…` |

```bash
# public pull (if package visibility is public)
docker pull ghcr.io/borisgraudt/forum:backend-edge
docker pull ghcr.io/borisgraudt/forum:frontend-edge

# version from release tag v0.1.0-alpha.1
docker pull ghcr.io/borisgraudt/forum:backend-0.1.0-alpha.1
docker pull ghcr.io/borisgraudt/forum:frontend-0.1.0-alpha.1

# private packages: authenticate first
echo $GITHUB_TOKEN | docker login ghcr.io -u USERNAME --password-stdin
```

After the first push: repo → **Packages** → **forum**  
or `https://github.com/borisgraudt/forum/pkgs/container/forum`

### Run from GHCR with Compose

```bash
export JWT_SECRET="$(openssl rand -hex 32)"
export BACKEND_IMAGE=ghcr.io/borisgraudt/forum:backend-edge
export FRONTEND_IMAGE=ghcr.io/borisgraudt/forum:frontend-edge
docker compose --profile app up -d
```

## Docker Compose (local build)

```bash
export JWT_SECRET="$(openssl rand -hex 32)"
export CORS_ORIGIN=http://localhost:4321
export PUBLIC_API_URL=http://localhost:3000/api/v1

make docker-up
# backend http://localhost:3000/health
# frontend http://localhost:4321
```

Stop / wipe volume:

```bash
make docker-down
docker volume rm forum_forum-data   # optional: destroy SQLite data
```

## Production checklist

1. Strong `JWT_SECRET` (never commit it)
2. `COOKIE_SECURE=true` only with HTTPS
3. `CORS_ORIGIN` matches the public UI origin exactly
4. Persist `/data` (or the SQLite path) with backups
5. Reverse proxy (Caddy/nginx) for TLS; optional single domain with path routing
6. Do not expose debug `RUST_LOG=debug` in production

## Pre-release versioning

- Format: `v0.1.0-alpha.1`, `v0.1.0-beta.1`, `v0.1.0`
- Tags starting with `v` trigger `.github/workflows/release.yml`
- GitHub **prerelease** until `vX.Y.Z` without pre suffix

## Health

```bash
curl -sS http://localhost:3000/health
# {"status":"ok","service":"forum-backend"}
```
