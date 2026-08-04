# Forum

Fast, low-resource forum engine: **Rust (Axum + SQLite)** API + **Astro** SSR UI.

Hierarchy: **Community → subcategory → topic → posts**. UI inspired by Apple Discussions.

**Current version:** `0.7.0`

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

Goal for **v1.0**: not a MVP — a **complete, fast, phone-first community product** that feels as polished as Apple Discussions and can run a real niche forum in production (including stuff we used to call “optional”).

### Done (through v0.2) — foundation
- Auth, CRUD, category hierarchy
- Apple-style Browse / Ask / Thread UI
- Markdown, pagination, FTS search
- CSRF, headers, rate limit, view counts
- Basic mod (lock / pin / delete)
- Basic responsive CSS (not yet a full mobile product)

### v0.3 — Community product core
- [x] **Admin panel** (`/admin`): users, roles, deactivate + **categories** (create/edit/delete empty)
- [x] **Categories** admin-only (communities + subcategories)
- [x] **Me too / Helpful** real votes + rankings
- [x] **Quote / reply-to** specific posts
- [x] **Edit / delete** own content (+ soft-delete, edit history)
- [x] **User profiles** (`/u/:user`, bio, reputation, recent activity; avatars later)
- [x] **Drafts** for ask + reply

### v0.4 — Trust, safety, identity
- [x] **Report** + mod queue (`/mod`)
- [x] **Ban / mute / timeout** users
- [x] **Audit log** of mod/admin actions
- [x] Per-route **rate limits** (auth / write / search / default)
- [x] **Mentions** (`@user` → profile link)
- ~~Password reset + email verification~~ → v0.5 (needs mail)

### v0.5 — Media & composition (fast path)
- [x] **Image attachments** (content-addressed store, MIME sniff, size limits, thumbs)
- [x] **Avatars** upload + server resize
- [x] **Markdown toolbar** (tiny progressive JS)
- [x] **Embed previews** (OG scrape, SQLite cache, SSR cards, SSRF-safe)
- [x] Clipboard **paste images** into composer
- [x] **Password reset** + email verify tokens (dev log transport; SMTP later)

### v0.6 — Discovery, engagement, realtime
- [x] **Notifications** in-app (`/notifications`, deep links, mark read, unread badge)
- [x] **Watch / subscribe** thread + category (auto-watch on reply + create)
- [x] **Sort / filters** (activity, newest, unanswered, solved)
- [x] **Solved** mark for OP/staff
- [x] **Live counts** pulse (~1.2s) + SSE endpoint; Me too / Helpful / Watch without full page reload (optimistic UI)
- [x] **Unreads** for watched threads
- [x] **Photo gallery** + lightbox viewer
- [ ] Email digests (needs SMTP — later)

### v0.7 — Mobile + performance (first-class)
- [x] **Phone-first layout**: stacked toolbars, reply box, safe-area padding
- [x] Touch targets ≥ 44px, sticky reply composer on mobile
- [x] No horizontal scroll; `content-visibility` on topic rows; system fonts only
- [x] PWA install shell (`manifest.webmanifest`)
- [x] **Lighthouse-oriented**: skip link, meta description/canonical, contrast AA, reduced motion
- [x] Cache-Control for static assets; security headers on HTML via Astro middleware
- [x] Progressive JS only on pages that need it (composer / live) — zero on browse
- [x] SQLite WAL + page cache / mmap (from 0.6)
- [ ] Optional: `srcset` variants for large media; offline SW

### v0.8 — Ops & multi-community scale
- [ ] Production defaults: HSTS, Secure cookies, reverse-proxy recipes
- [ ] **Backups** (SQLite snapshot + restore runbook)
- [ ] Structured logs + metrics (Prometheus/OpenTelemetry)
- [ ] E2E CI (Playwright) + load test baseline
- [ ] **Multi-tenant / multi-site** mode (one binary, many communities)
- [ ] OpenAPI for `/api/v1` + seed/demo command
- [ ] i18n (RU/EN minimum)

### v0.9 — Polish & “cooler than NodeBB” bar
- [ ] Design system tokens + dark mode
- [ ] Full keyboard a11y + WCAG AA pass
- [ ] Rich search (filters by author/category/date)
- [ ] Import from Discourse / NodeBB (CSV/JSON tools)
- [ ] Federation hooks *or* clean plugin API (pick one path)
- [ ] Public status / health dashboard for ops

### v1.0.0 — Release bar (all of the above shipped)
- [ ] Feature freeze of the full list above
- [ ] Stable `/api/v1` compatibility promise
- [ ] Security review (no critical/high open issues)
- [ ] Mobile + desktop Lighthouse + real-device QA
- [ ] Load test: documented numbers for N concurrent users on modest VPS
- [ ] Tagged `v1.0.0` + release notes + upgrade path from 0.x

**Post-v1.0 (nice later, not blocking 1.0):** full multi-region SaaS billing, ActivityPub federation if not chosen in 0.9, native apps.

---

## Mobile & speed — current status (honest)

### Mobile (today)
| Area | Status |
|------|--------|
| Viewport meta | yes |
| Fluid layout / page rail | yes (`max-width` + padding) |
| Some breakpoints | yes (~480 / 600 / 700 / 800px) — header, lists, thread |
| Full phone UX | **partial** — works, not “app-like” |
| Nav on small screens | may crowd (Ask / Browse / Search + avatar) |
| Thread actions (pills) | wrap, but dense |
| Forms (Ask/Search) | usable; not optimized for thumb |
| Safe areas / bottom nav | **no** |
| Touch target audit | **no** |
| PWA | **no** |

**Verdict:** desktop-first with responsive CSS. Fine for reading on phone; not yet a polished mobile product. That’s a first-class **v0.7** track for v1.0.

### Speed (today)
| Area | Status |
|------|--------|
| Backend | Rust + SQLite — inherently fast for small/medium communities |
| SSR API hops | fixed `127.0.0.1` + parallel `Promise.all` on key pages |
| Measured earlier | API ~0.5–1 ms; full SSR page ~10–15 ms local (dev) |
| Production build | `cargo build --release` + Astro build — much better than `astro dev` |
| Client JS | minimal by design (Astro) — good for mobile CPU |
| Caching | almost none beyond browser defaults |
| Images/CDN | N/A until attachments |
| FTS search | local SQLite — fast for typical forum size |
| Rate limit | in-memory (fine single-node) |

**Verdict:** architecture is already **speed-friendly**. Bottlenecks later will be: N+1 if we get sloppy, big attachments, uncached SSR, and `astro dev` feeling “slow” in development. v1.0 needs a real **perf budget + load test** (v0.7–v0.8), not a rewrite.

---

## License

No license file is shipped in this repo. Add one if you open-source or redistribute.
