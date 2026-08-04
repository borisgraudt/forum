# Load baseline (document for v1.0)

Run on the same host class you intend to deploy (e.g. 1 vCPU / 1 GB VPS).

```bash
# API up on :3000
./scripts/load-smoke.sh http://127.0.0.1:3000 200
```

## Expected ballpark (local / small VPS)

| Scenario | Target |
|----------|--------|
| Sequential `GET /health` ×200 | &lt; 500 ms total on laptop SSD |
| Parallel `GET /api/v1/categories` ×200 (8 workers) | Completes without 5xx |
| Single-thread page SSR (Astro) | TTFB typically tens of ms when API is local |

Record your numbers in the release notes:

```
date:
host:
health_rps:
categories_parallel_ms:
notes:
```

Lighthouse **production** mobile Browse: aim Performance / A11y / BP / SEO **≥ 95** (project goal 100 on clean prod).

See also [ops.md](./ops.md).
