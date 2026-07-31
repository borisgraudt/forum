# Forum

High-performance, security-focused forum engine built with **Rust + Astro**.

Designed for niche communities that value speed, low resource usage, and strong security guarantees.

> Single responsibility: be extremely fast and hard to break.

---

## Tech Stack

### Backend
| Technology | Purpose |
|----------|--------|
| **Rust** | Memory safety + maximum performance |
| **Axum** | Modern, ergonomic web framework |
| **SQLx** | Async SQL with compile-time checked queries |
| **SQLite** (WAL mode) | Extremely fast, zero-config, single-file database |
| **Tower** | Middleware (CORS, trace, rate limiting) |
| **jsonwebtoken** + **bcrypt** | Authentication |
| **pulldown-cmark** + **ammonia** | Markdown rendering + HTML sanitization |
| **validator** + **serde** | Input validation |

### Frontend
| Technology | Purpose |
|----------|--------|
| **Astro** | Server-rendered UI, minimal JS |
| **TypeScript** | Type safety |

### Infrastructure
- Docker + Docker Compose
- sqlx-cli for migrations
- Makefile for common tasks

---

## Project Structure

```text
forum/
├── backend/                 # Rust + Axum API
│   ├── src/
│   │   ├── main.rs
│   │   ├── config.rs
│   │   ├── db.rs
│   │   ├── error.rs
│   │   ├── models/          # users, categories, threads, posts
│   │   └── state.rs
│   ├── migrations/
│   └── Cargo.toml
│
├── frontend/                # Astro
│   ├── src/
│   └── package.json
│
├── .github/workflows/ci.yml
├── docker-compose.yml
├── Makefile
├── CONTRIBUTING.md
└── README.md
```

---

## Getting Started

### Prerequisites

- Rust (latest stable) — install via `rustup`
- Node.js 20+
- sqlx-cli: `cargo install sqlx-cli --no-default-features --features sqlite`
- Docker (optional)

### 1. Clone & setup

```bash
git clone <your-repo-url> ultraforum
cd ultraforum
```

### 2. Backend

```bash
cd backend
cp .env.example .env
# edit .env if needed

# Create database and run migrations
sqlx database create
sqlx migrate run

# Run in development
cargo watch -x run
```

Backend will be available at `http://localhost:3000`

### 3. Frontend

```bash
cd frontend
npm install
cp .env.example .env

npm run dev
```

Frontend will be available at `http://localhost:4321`

UI is a minimal community shell (Apple Discussions–inspired): topics, threads, posts, sign-in/join. Start the backend first so SSR can reach the API.

### 4. Development with Makefile (recommended)

```bash
make setup        # deps + .env
make migrate      # apply SQLx migrations
make dev          # cargo-watch backend + Astro HMR
make lint         # rustfmt + clippy + frontend lint
make test         # backend tests
make build        # production build
make audit        # cargo audit + npm audit
```

See [CONTRIBUTING.md](./CONTRIBUTING.md) for Git flow and PR rules.

---

## Workflows

| Workflow | Command / place | Notes |
|----------|-----------------|-------|
| Local development | `make dev` | Backend `:3000`, frontend `:4321` |
| Migrations | `make migrate` | SQLx + files in `backend/migrations/` |
| Lint & format | `make lint` / `make fmt` | rustfmt, clippy `-D warnings`, Prettier, `astro check` |
| Tests | `make test` | Unit + HTTP smoke tests on backend |
| Package (tarball) | `make package` | Portable archive under `dist/` / Release **Assets** |
| GitHub Packages | one package **`forum`** on GHCR | tags `backend-*` / `frontend-*` |
| Docker | `make docker-up` | Compose profile `app` |
| Git flow | `main` ← `develop` ← `feature/*` | Details in CONTRIBUTING |
| CI | `.github/workflows/ci.yml` | fmt · clippy · test · frontend build · audit on every PR |
| Release | tag `v*` → release workflow | Tarballs + GHCR + GitHub Release |
| Security | `make audit` | `cargo audit` + `npm audit --audit-level=high` |

See **[docs/DEPLOY.md](./docs/DEPLOY.md)** for production env, Docker, **GitHub Packages**, and pre-releases.

**Current version:** `0.1.0-alpha.1`

---

## Environment Variables

### Backend (`.env`)

```env
HOST=0.0.0.0
PORT=3000
DATABASE_URL=sqlite:forum.db?mode=rwc
JWT_SECRET=change-me-to-a-long-random-string
JWT_TTL_SECS=604800
COOKIE_SECURE=false
RUST_LOG=info
CORS_ORIGIN=http://localhost:4321
```

### Auth API (`/api/v1/auth`)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/register` | Create account, set `session` httpOnly cookie |
| POST | `/login` | Login (username or email), set cookie |
| POST | `/logout` | Clear session cookie |
| GET | `/me` | Current user (requires cookie) |

Cookie: `session` — httpOnly, SameSite=Lax, Secure when `COOKIE_SECURE=true`.

### Forum hierarchy

```text
Community (root category)  →  Subcategory  →  Topic (thread)  →  Posts
```

UI matches Apple Discussions: **Browse** grid → subcategory list → topics → thread.

### Forum API

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/v1/categories` | no | List **root** communities |
| GET | `/api/v1/categories/{slug}` | no | Get category (+ `children` if root) |
| GET | `/api/v1/categories/{slug}/children` | no | List subcategories |
| POST | `/api/v1/categories` | yes | Create community or subcategory (`parent_slug`) |
| GET | `/api/v1/categories/{slug}/threads` | no | List topics in a category |
| GET | `/api/v1/categories/{c}/threads/{t}` | no | Get topic |
| POST | `/api/v1/categories/{slug}/threads` | yes | Create topic + first post |
| GET/POST | `.../posts` | reply needs auth | List / reply |

### Frontend (`.env`)

```env
PUBLIC_API_URL=http://localhost:3000/api/v1
```

---

## Security Highlights

- Memory-safe backend (Rust)
- httpOnly + Secure + SameSite cookies for sessions
- CSRF double-submit cookie (`csrf` + `X-CSRF-Token`)
- Strict CORS (`CORS_ORIGIN`)
- Security headers (CSP, X-Content-Type-Options, X-Frame-Options, Referrer-Policy)
- Input validation on every endpoint
- In-memory IP rate limiting
- Prepared statements only (SQLx)
- Markdown sanitized with ammonia (no raw HTML scripts)

---

## Performance Goals

- TTFB < 30ms on modest hardware
- Very low memory footprint
- Excellent Lighthouse scores (especially Performance & Best Practices)
- Single binary backend possible

---

## Roadmap

### Done (v0.2)
1. Core CRUD + Auth
2. Category hierarchy + Apple-style UI
3. Markdown + sanitization
4. Pagination (threads / posts / search)
5. Search (SQLite FTS5)
6. CSRF, security headers, rate limiting
7. Real view counters
8. Basic moderation (lock / pin / delete post)

### Next (v0.3+)
- Admin panel
- Me too / Helpful votes
- Attachments / images
- Email / password reset
- Production HSTS + reverse-proxy guides

---

## License

MIT
