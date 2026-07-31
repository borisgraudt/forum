# Contributing

## Git flow

```text
main          → production-ready
└── develop   → integration branch for active development
    └── feature/xxx
    └── fix/xxx
    └── chore/xxx
```

### Rules

1. **Never commit directly to `main`.** Prefer PRs into `develop`, then release PRs `develop → main`.
2. **One concern per branch.** Models, auth, CI, UI — separate branches/PRs.
3. **Conventional Commits** (roughly):
   - `feat:` new user-facing capability
   - `fix:` bug fix
   - `chore:` tooling, deps, workflows
   - `docs:` documentation only
   - `test:` tests only
   - `refactor:` no behavior change
4. **Every PR must pass CI** (see below).

### Branch naming

| Prefix | Use |
|--------|-----|
| `feature/` | New functionality (`feature/db-models`) |
| `fix/` | Bug fixes |
| `chore/` | Tooling / CI / DX |
| `docs/` | Docs-only |

### Example

```bash
git checkout develop
git pull origin develop
git checkout -b feature/auth

# ... work ...
make lint
make test

git push -u origin HEAD
# open PR → develop
```

## Local workflows

| Command | Purpose |
|---------|---------|
| `make setup` | Install frontend deps, copy `.env` |
| `make dev` | Backend (`cargo watch`) + Astro HMR |
| `make migrate` | Apply SQLx migrations |
| `make lint` | rustfmt + clippy + frontend lint |
| `make test` | Backend tests |
| `make build` | Release backend + frontend build |
| `make audit` | `cargo audit` + `npm audit` |
| `make ci` | Local approximation of GitHub Actions |

### Required PR checks (CI)

- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test`
- frontend lint + build
- `cargo audit` / `npm audit` (high+)

## Database migrations

- Migrations live in `backend/migrations/`.
- Create with: `cd backend && sqlx migrate add <name>`
- Never edit a migration that already landed on `develop`/`main` — add a new one.
- Schema changes ship in the same PR as the Rust models that use them.

## Security baseline (product goals)

- httpOnly + Secure + SameSite cookies for auth
- CSRF on mutating requests
- Strict CORS (`CORS_ORIGIN`)
- Validate all input
- Prefer fewer dependencies

## What “better than NodeBB” means here

Not more features — **faster, lighter, safer, simpler to deploy**:

- Low memory (Rust + SQLite)
- One binary + one DB file
- Astro with minimal JS
- Compile-time safety + opinionated structure
