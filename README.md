# Forum

Fast, low-resource forum engine: **Rust (Axum + SQLite)** API + **Astro** SSR UI.

Hierarchy: **Community → subcategory → topic → posts**. UI inspired by Apple Discussions.

**Current version:** `0.2.0`

---

## Stack

| Layer | Tech |
|-------|------|
| API | Rust, Axum, SQLx, SQLite (WAL), JWT cookie auth |
| UI | Astro (Node adapter), TypeScript |
| Content | Markdown (pulldown-cmark) + ammonia sanitize |
| Search | SQLite FTS5 |
| Ops | Docker Compose, GHCR, Makefile, GitHub Actions |

---

## Layout

```text
forum/
├── backend/          # API + migrations
├── frontend/         # Astro UI
├── .github/workflows/
├── docker-compose.yml
├── Makefile
└── README.md
```

---

## Quick start

**Prereqs:** Rust stable, Node 22+, optional `sqlx-cli` / `cargo-watch`.

```bash
# Backend
cd backend
cp .env.example .env
sqlx database create && sqlx migrate run
cargo run          # :3000

# Frontend (other terminal)
cd frontend
npm install
cp .env.example .env   # PUBLIC_API_URL=http://127.0.0.1:3000/api/v1
npm run dev            # :4321
```

Or:

```bash
make setup && make migrate && make dev
```

### Useful make targets

| Target | What |
|--------|------|
| `make dev` | API + UI (needs cargo-watch) |
| `make test` | Backend tests |
| `make lint` | fmt, clippy, frontend check |
| `make build` | Release backend + frontend build |
| `make package` | Portable tarball under `dist/` |
| `make docker-up` | Compose stack |

### Moderator / admin

No separate admin UI yet. Promote a user, then use Lock / Pin / Delete on a thread page:

```bash
sqlite3 backend/forum.db "UPDATE users SET role = 'moderator' WHERE username = 'you';"
```

---

## API sketch (`/api/v1`)

| Area | Paths |
|------|--------|
| Auth | `POST /auth/register\|login\|logout`, `GET /auth/me`, `GET /auth/csrf` |
| Categories | `GET/POST /categories`, `GET …/{slug}`, `GET …/{slug}/children` |
| Threads | `GET/POST …/threads`, `GET/PATCH …/threads/{t}` |
| Posts | `GET/POST …/posts`, `DELETE …/posts/{id}` |
| Search | `GET /search?q=` |

Mutating requests need CSRF: cookie `csrf` + header `X-CSRF-Token`.

---

## Security (current)

- httpOnly session cookie, optional Secure
- CSRF double-submit
- Security headers + IP rate limit
- Input validation, prepared statements
- Sanitized Markdown HTML

---

## Roadmap to v1.0.0

### Done (through v0.2)
- Auth, CRUD, category hierarchy
- Apple-style Browse / Ask / Thread UI
- Markdown, pagination, FTS search
- CSRF, headers, rate limit, view counts
- Basic mod actions (lock / pin / delete)

### v0.3 — Product depth
- [ ] **Admin panel** (`/admin`): users, roles, reports, global settings
- [ ] **Me too / Helpful** votes (real counts, not placeholders)
- [ ] **Quote / reply-to** a specific post
- [ ] **Edit own posts** (with history optional)
- [ ] **User profiles** (public page, activity)

### v0.4 — Trust & safety
- [ ] **Report** content + mod queue
- [ ] **Ban / mute** users
- [ ] **Audit log** of mod actions
- [ ] **Stricter rate limits** per endpoint (auth / post)
- [ ] **Password reset** (email or admin-issued)

### v0.5 — Content & media
- [ ] **Attachments / images** (upload + size limits + scan)
- [ ] **Avatars**
- [ ] **Better markdown toolbar** (real formatting, not decorative)
- [ ] **Drafts** (optional)

### v0.6 — Discovery & UX
- [ ] **Notifications** (replies, mentions)
- [ ] **Watch / subscribe** thread or category
- [ ] **Sort threads** (activity, newest, unanswered)
- [ ] **“Solved” / accepted answer**
- [ ] Mobile polish + empty/error states

### v0.7 — Ops / production
- [ ] Reverse-proxy ready (HSTS, secure cookies by default in prod)
- [ ] Backups story for SQLite (docs + script in Makefile or cron example)
- [ ] Metrics / health depth (optional Prometheus)
- [ ] Structured logging
- [ ] E2E smoke tests in CI

### v0.8–0.9 — Hardening
- [ ] Full API docs (OpenAPI)
- [ ] Seed / demo data command
- [ ] Permission matrix review
- [ ] Load test notes + defaults for small communities
- [ ] i18n-ready strings (if you care about RU/EN)

### v1.0.0 — “Ship it”
- [ ] Feature freeze of the above core
- [ ] Stable API versioning commitment for `/api/v1`
- [ ] Release checklist + tagged `v1.0.0`
- [ ] Known limitations documented in README
- [ ] No critical open security issues

**Not required for v1.0** (later): multi-tenant SaaS, realtime websockets, email digests, federation, full SPA rewrite.

---

## License

No license file is shipped in this repo. Add one if you open-source or redistribute.
