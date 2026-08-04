# API stability promise (`/api/v1`)

As of **v0.9 / v1.0**, the following applies:

## Guarantees

1. **Prefix** `/api/v1` remains the stable surface until a future `/api/v2`.
2. **Additive changes only** within v1: new optional JSON fields and new routes are OK.
3. **No breaking renames/removals** of existing request/response fields without a major version bump.
4. **Auth model** stays: httpOnly `session` cookie + CSRF double-submit (`csrf` cookie + `X-CSRF-Token`) on mutating methods.
5. **Error shape** remains `{ "error": "…" }` with appropriate HTTP status codes.

## Non-guarantees

- Exact rate-limit thresholds and headers
- Internal metrics label set (may grow)
- Experimental CLI (`seed`, `import`) JSON schema may extend

## Compatibility checklist for clients

- Ignore unknown JSON fields.
- Treat `limit`/`offset` defaults as soft (server may clamp).
- Use `Accept: application/json` when calling BFF live proxies.

See also [openapi.yaml](./openapi.yaml).
