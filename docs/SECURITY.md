# Security review checklist (v1.0 bar)

Run before tagging a production release.

## Secrets & config

- [ ] `FORUM_ENV=production`
- [ ] `JWT_SECRET` is 32+ random bytes (not example values)
- [ ] `COOKIE_SECURE=true` and HTTPS only
- [ ] `ENABLE_HSTS=true` (or TLS terminator sets HSTS)
- [ ] `CORS_ORIGIN` / `PUBLIC_ORIGIN` match real site origin
- [ ] Default seed password changed or seed not used in prod
- [ ] `/metrics` not public without network ACL

## App surface

- [ ] CSRF required on all state-changing API methods
- [ ] Auth cookies `HttpOnly` + `Secure` + `SameSite=Lax`
- [ ] Upload size / MIME sniff limits still enforced
- [ ] Markdown sanitization (ammonia) still on post HTML
- [ ] Rate limits active (auth / write / search / default)
- [ ] Soft-deleted posts not exposing private body to strangers

## Ops

- [ ] Automated SQLite backups (`scripts/backup-sqlite.sh`) tested restore
- [ ] Reverse proxy terminates TLS (Caddy/nginx examples in `deploy/`)
- [ ] Logs do not print JWT secrets or passwords
- [ ] Dependency audits clean: `cargo audit`, `npm audit --audit-level=high`

## Process

- [ ] No open critical/high GitHub security advisories on pinned deps
- [ ] Smoke e2e passes against staging (`make test-smoke`)
