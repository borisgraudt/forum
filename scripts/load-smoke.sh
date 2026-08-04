#!/usr/bin/env bash
# Tiny load baseline: sequential + parallel health/list hits.
# Usage: scripts/load-smoke.sh [base_url] [n]
set -euo pipefail

BASE="${1:-http://127.0.0.1:3000}"
N="${2:-50}"

echo "==> load-smoke $BASE  n=$N"

t0=$(date +%s%N)
for i in $(seq 1 "$N"); do
  code=$(curl -s -o /dev/null -w '%{http_code}' "$BASE/health")
  [[ "$code" == "200" ]] || { echo "health failed HTTP $code"; exit 1; }
done
t1=$(date +%s%N)
ms=$(( (t1 - t0) / 1000000 ))
echo "sequential health x$N: ${ms}ms  (~$(( ms > 0 ? N * 1000 / ms : 0 )) rps)"

t0=$(date +%s%N)
# shellcheck disable=SC2046
printf '%s\n' $(seq 1 "$N") | xargs -P 8 -I{} curl -s -o /dev/null "$BASE/api/v1/categories"
t1=$(date +%s%N)
ms=$(( (t1 - t0) / 1000000 ))
echo "parallel categories x$N (8 workers): ${ms}ms"

if curl -s -o /dev/null -w '%{http_code}' "$BASE/metrics" | grep -q 200; then
  echo "metrics ok"
  curl -s "$BASE/metrics" | head -20
fi

echo "done"
