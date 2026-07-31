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
| **Tower** | Middleware (timeouts, rate limiting, etc.) |
| **jsonwebtoken** + **bcrypt** | Authentication |
| **pulldown-cmark** | Safe & fast Markdown rendering |
| **validator** + **serde** | Input validation |

### Frontend
| Technology | Purpose |
|----------|--------|
| **Astro** | Zero-JS by default, excellent performance |
| **TypeScript** | Type safety |
| **Tailwind CSS** | Utility-first styling |
| **HTMX** (optional) | Progressive enhancement without heavy JS |

### Infrastructure
- Docker + Docker Compose
- sqlx-cli for migrations
- Makefile for common tasks

---

## Project Structure

```text
ultraforum/
├── backend/                 # Rust + Axum API
│   ├── src/
│   │   ├── main.rs
│   │   ├── config.rs
│   │   ├── error.rs
│   │   ├── state.rs
│   │   ├── db/
│   │   ├── models/
│   │   ├── dto/
│   │   ├── handlers/
│   │   ├── middleware/
│   │   ├── services/
│   │   └── utils/
│   ├── migrations/
│   └── Cargo.toml
│
├── frontend/                # Astro
│   ├── src/
│   │   ├── pages/
│   │   ├── components/
│   │   ├── layouts/
│   │   ├── lib/
│   │   └── styles/
│   └── package.json
│
├── shared/
├── docker-compose.yml
├── Makefile
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

### 4. Development with Makefile (recommended)

```bash
make dev          # starts both backend and frontend
make migrate      # run migrations
make build        # production build
```

---

## Environment Variables

### Backend (`.env`)

```env
HOST=0.0.0.0
PORT=3000
DATABASE_URL=sqlite:forum.db?mode=rwc
JWT_SECRET=change-me-to-a-long-random-string
RUST_LOG=info
CORS_ORIGIN=http://localhost:4321
```

### Frontend (`.env`)

```env
PUBLIC_API_URL=http://localhost:3000/api/v1
```

---

## Security Highlights

- Memory-safe backend (Rust)
- httpOnly + Secure + SameSite cookies
- CSRF protection
- Strict CORS
- Security headers (CSP, HSTS, X-Content-Type-Options, etc.)
- Input validation on every endpoint
- Rate limiting (Tower layer)
- Prepared statements only (SQLx)

---

## Performance Goals

- TTFB < 30ms on modest hardware
- Very low memory footprint
- Excellent Lighthouse scores (especially Performance & Best Practices)
- Single binary backend possible

---

## Roadmap

1. Core CRUD + Auth
2. Markdown + sanitization
3. Pagination & sorting
4. Search (FTS5)
5. Admin panel
6. Rate limiting & moderation tools
7. Production hardening + deployment guides

---

## License

MIT
