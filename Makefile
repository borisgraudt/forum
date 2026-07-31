.PHONY: help dev dev-backend dev-frontend build lint lint-backend lint-frontend \
	fmt fmt-check test test-backend migrate migrate-info db-create db-reset \
	audit audit-backend audit-frontend clean setup ci \
	package docker-build docker-up docker-down release-dry

ROOT := $(abspath $(dir $(lastword $(MAKEFILE_LIST))))
BACKEND := $(ROOT)/backend
FRONTEND := $(ROOT)/frontend

export DATABASE_URL ?= sqlite:$(BACKEND)/forum.db?mode=rwc
export VERSION ?= 0.1.0-alpha.1

help: ## Show available targets
	@grep -E '^[a-zA-Z_-]+:.*?##' $(MAKEFILE_LIST) | sort | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-18s\033[0m %s\n", $$1, $$2}'

setup: ## Install local tooling hints + deps
	@command -v cargo >/dev/null || (echo "Install Rust via rustup" && exit 1)
	@command -v node >/dev/null || (echo "Install Node.js 22+" && exit 1)
	@command -v sqlx >/dev/null || echo "Optional: cargo install sqlx-cli --no-default-features --features sqlite"
	@command -v cargo-watch >/dev/null || echo "Optional: cargo install cargo-watch"
	@command -v cargo-audit >/dev/null || echo "Optional: cargo install cargo-audit"
	cd $(FRONTEND) && npm install
	@test -f $(BACKEND)/.env || cp $(BACKEND)/.env.example $(BACKEND)/.env
	@echo "Setup complete. Run: make migrate && make dev"

dev: ## Start backend + frontend (requires cargo-watch)
	@command -v cargo-watch >/dev/null || (echo "Install cargo-watch: cargo install cargo-watch" && exit 1)
	@echo "Starting backend (:3000) and frontend (:4321)..."
	@$(MAKE) -j2 dev-backend dev-frontend

dev-backend: ## Backend only with auto-reload
	cd $(BACKEND) && cargo watch -x run

dev-frontend: ## Frontend only (Astro HMR)
	cd $(FRONTEND) && npm run dev

build: ## Production build (backend + frontend)
	cd $(BACKEND) && cargo build --release
	cd $(FRONTEND) && npm run build

package: ## Build portable tarball under dist/ (binary + UI + migrations)
	chmod +x $(ROOT)/scripts/package.sh
	VERSION=$(VERSION) $(ROOT)/scripts/package.sh

docker-build: ## Build backend + frontend images
	docker compose --profile app build

docker-up: ## Run stack (profile app) on :3000 + :4321
	docker compose --profile app up -d --build
	@echo "API  http://localhost:3000/health"
	@echo "UI   http://localhost:4321"

docker-down: ## Stop compose app stack
	docker compose --profile app down

release-dry: ## Show how to cut a pre-release tag
	@echo "1. Merge to develop/main"
	@echo "2. git tag -a v$(VERSION) -m \"Forum v$(VERSION)\""
	@echo "3. git push origin v$(VERSION)"
	@echo "   → GitHub Actions builds packages + creates release"
	@echo "Or local: make package && gh release create v$(VERSION) --prerelease dist/*.tar.gz dist/*.sha256"

lint: lint-backend lint-frontend ## Run all linters

lint-backend: ## rustfmt check + clippy (-D warnings)
	cd $(BACKEND) && cargo fmt --all -- --check
	cd $(BACKEND) && cargo clippy --all-targets --all-features -- -D warnings

lint-frontend: ## Prettier check + Astro/TS check + build smoke
	cd $(FRONTEND) && npm run lint

fmt: ## Auto-format backend + frontend
	cd $(BACKEND) && cargo fmt --all
	cd $(FRONTEND) && npm run format

fmt-check: ## Check formatting only
	cd $(BACKEND) && cargo fmt --all -- --check
	cd $(FRONTEND) && npm run format:check

test: test-backend ## Run all tests

test-backend: ## Backend unit/integration tests
	cd $(BACKEND) && cargo test

migrate: ## Apply sqlx migrations
	cd $(BACKEND) && sqlx database create || true
	cd $(BACKEND) && sqlx migrate run

migrate-info: ## Show migration status
	cd $(BACKEND) && sqlx migrate info

db-create: ## Create sqlite database file
	cd $(BACKEND) && sqlx database create

db-reset: ## Drop + recreate DB and re-run migrations (DESTRUCTIVE)
	@echo "Resetting database at $(DATABASE_URL)"
	cd $(BACKEND) && sqlx database drop -y || true
	cd $(BACKEND) && sqlx database create
	cd $(BACKEND) && sqlx migrate run

audit: audit-backend audit-frontend ## Security audits

audit-backend: ## cargo audit
	@command -v cargo-audit >/dev/null || cargo install cargo-audit --locked
	cd $(BACKEND) && cargo audit

audit-frontend: ## npm audit (high+)
	cd $(FRONTEND) && npm audit --audit-level=high

clean: ## Remove build artifacts
	cd $(BACKEND) && cargo clean
	cd $(FRONTEND) && rm -rf dist .astro
	rm -rf $(ROOT)/dist
	rm -f $(BACKEND)/forum.db $(BACKEND)/forum.db-*

ci: fmt-check lint-backend test-backend audit-backend ## Local approximation of CI (backend-heavy)
	cd $(FRONTEND) && npm ci
	cd $(FRONTEND) && npm run lint
	cd $(FRONTEND) && npm run build
