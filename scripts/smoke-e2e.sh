#!/usr/bin/env bash
# Smoke e2e for forum v0.3 — API + frontend BFF form flows.
# Requires: curl, jq, python3; running backend (:3000) + frontend (:4321).
set -euo pipefail

API="${API_BASE:-http://127.0.0.1:3000}"
WEB="${WEB_BASE:-http://localhost:4321}"
API_V1="${API}/api/v1"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DB="${DATABASE_FILE:-$ROOT/backend/forum.db}"

PASS=0
FAIL=0
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

COOKIE_A="$TMP/a.jar"
COOKIE_B="$TMP/b.jar"
COOKIE_WEB="$TMP/web.jar"
COOKIE_ADMIN="$TMP/admin.jar"

API_BODY=""
API_CODE=""
WEB_BODY=""
WEB_CODE=""
WEB_REDIRECT=""

SUF="$(date +%s)"
# usernames must match [A-Za-z_][A-Za-z0-9_]{2,31}
USER_A="a${SUF}"
USER_B="b${SUF}"
USER_C="c${SUF}"
USER_A="${USER_A:0:28}"
USER_B="${USER_B:0:28}"
USER_C="${USER_C:0:28}"
PASSWD="password123"
CAT_NAME="Smoke ${SUF}"
CAT_SLUG=""
THREAD_SLUG=""
POST_OP_ID=""
POST_REPLY_ID=""

green() { printf '\033[32m%s\033[0m\n' "$*"; }
red() { printf '\033[31m%s\033[0m\n' "$*"; }

ok() {
  PASS=$((PASS + 1))
  green "  ✓ $*"
}

bad() {
  FAIL=$((FAIL + 1))
  red "  ✗ $*"
}

assert_eq() {
  local label="$1" got="$2" want="$3"
  if [[ "$got" == "$want" ]]; then
    ok "$label ($got)"
  else
    bad "$label: got='$got' want='$want'"
  fi
}

assert_http() {
  local label="$1" code="$2" want="$3"
  if [[ "$code" == "$want" ]]; then
    ok "$label HTTP $code"
  else
    bad "$label HTTP $code (want $want)"
  fi
}

assert_contains() {
  local label="$1" hay="$2" needle="$3"
  if [[ "$hay" == *"$needle"* ]]; then
    ok "$label contains '$needle'"
  else
    bad "$label missing '$needle'"
  fi
}

assert_not_contains() {
  local label="$1" hay="$2" needle="$3"
  if [[ "$hay" != *"$needle"* ]]; then
    ok "$label lacks '$needle'"
  else
    bad "$label unexpectedly contains '$needle'"
  fi
}

section() {
  echo
  echo "==> $*"
}

# Read csrf cookie value from a Netscape jar (no extra HTTP call).
csrf_from_jar() {
  local jar="$1"
  if [[ ! -f "$jar" ]]; then
    return 1
  fi
  # jar columns: domain flag path secure expiry name value
  awk '$6 == "csrf" { print $7; exit }' "$jar"
}

# Ensure jar has a csrf cookie (issues one only if missing).
ensure_api_csrf() {
  local jar="$1"
  local token
  token="$(csrf_from_jar "$jar" || true)"
  if [[ -n "${token:-}" ]]; then
    printf '%s\n' "$token"
    return 0
  fi
  local out code body
  out="$(curl -sS -c "$jar" -b "$jar" -w '\n%{http_code}' "${API_V1}/auth/csrf")"
  code="$(printf '%s\n' "$out" | tail -n1)"
  body="$(printf '%s\n' "$out" | sed '$d')"
  if [[ "$code" != "200" ]]; then
    echo "csrf failed HTTP $code" >&2
    return 1
  fi
  # Prefer cookie jar value (source of truth after Set-Cookie).
  token="$(csrf_from_jar "$jar" || true)"
  if [[ -z "${token:-}" ]]; then
    token="$(printf '%s\n' "$body" | jq -r '.csrf_token')"
  fi
  printf '%s\n' "$token"
}

# Sets API_BODY / API_CODE
api_json() {
  local method="$1" path="$2" jar="$3"
  local payload="${4-}"
  local csrf out
  csrf="$(ensure_api_csrf "$jar")"
  if [[ -n "$payload" ]]; then
    out="$(curl -sS -c "$jar" -b "$jar" \
      -H "X-CSRF-Token: ${csrf}" \
      -H "Content-Type: application/json" \
      -X "$method" \
      -d "$payload" \
      -w '\n%{http_code}' \
      "${API_V1}${path}")"
  else
    out="$(curl -sS -c "$jar" -b "$jar" \
      -H "X-CSRF-Token: ${csrf}" \
      -H "Content-Type: application/json" \
      -X "$method" \
      -w '\n%{http_code}' \
      "${API_V1}${path}")"
  fi
  API_CODE="$(printf '%s\n' "$out" | tail -n1)"
  API_BODY="$(printf '%s\n' "$out" | sed '$d')"
}

api_get() {
  local path="$1"
  local jar="${2-}"
  local out
  if [[ -n "$jar" ]]; then
    out="$(curl -sS -c "$jar" -b "$jar" -w '\n%{http_code}' "${API_V1}${path}")"
  else
    out="$(curl -sS -w '\n%{http_code}' "${API_V1}${path}")"
  fi
  API_CODE="$(printf '%s\n' "$out" | tail -n1)"
  API_BODY="$(printf '%s\n' "$out" | sed '$d')"
}

extract_csrf_html() {
  python3 -c '
import re, sys
html = open(sys.argv[1], encoding="utf-8", errors="replace").read()
m = re.search(r"name=[\"'\'']csrf_token[\"'\'']\s+value=[\"'\'']([^\"'\'']+)[\"'\'']", html)
if not m:
    m = re.search(r"value=[\"'\'']([^\"'\'']+)[\"'\'']\s+name=[\"'\'']csrf_token[\"'\'']", html)
print(m.group(1) if m else "")
' "$1"
}

web_get() {
  local path="$1"
  local jar="${2-$COOKIE_WEB}"
  local attempt=1
  while true; do
    WEB_CODE="$(curl -sS -c "$jar" -b "$jar" -o "$TMP/web.html" -w '%{http_code}' "${WEB}${path}")"
    WEB_BODY="$(cat "$TMP/web.html")"
    if [[ "$WEB_CODE" == "429" || "$WEB_BODY" == *"rate limit exceeded"* ]]; then
      if [[ "$attempt" -ge 4 ]]; then
        break
      fi
      echo "    (rate limited on GET ${path}, sleeping 35s…)"
      sleep 35
      attempt=$((attempt + 1))
      continue
    fi
    break
  done
}

web_form() {
  local method="$1" path="$2" jar="$3"
  shift 3
  local args=()
  local pair
  for pair in "$@"; do
    args+=(--data-urlencode "$pair")
  done
  local meta
  # Astro security.checkOrigin requires a matching Origin on mutating requests.
  meta="$(curl -sS -c "$jar" -b "$jar" -X "$method" "${args[@]}" \
    -H "Origin: ${WEB}" \
    -H "Referer: ${WEB}/" \
    -o "$TMP/web_post_body" \
    -w '%{http_code}|%{redirect_url}' \
    "${WEB}${path}")"
  WEB_CODE="${meta%%|*}"
  WEB_REDIRECT="${meta#*|}"
  WEB_BODY="$(cat "$TMP/web_post_body" 2>/dev/null || true)"
}

j() {
  printf '%s\n' "$API_BODY" | jq -r "$1"
}

# ── readiness ──────────────────────────────────────────────────────────────
section "Readiness"
h="$(curl -sS -o /dev/null -w '%{http_code}' "${API}/health" || true)"
assert_http "backend /health" "$h" "200"
h="$(curl -sS -o /dev/null -w '%{http_code}' "${WEB}/" || true)"
assert_http "frontend /" "$h" "200"

# ── API: auth ──────────────────────────────────────────────────────────────
section "API auth"
api_json POST /auth/register "$COOKIE_A" "$(jq -n \
  --arg u "$USER_A" --arg e "${USER_A}@example.com" --arg p "$PASSWD" --arg d "Alice Smoke" \
  '{username:$u,email:$e,password:$p,display_name:$d}')"
assert_http "register A" "$API_CODE" "201"
assert_eq "register A username" "$(j '.user.username')" "$USER_A"

api_json POST /auth/register "$COOKIE_B" "$(jq -n \
  --arg u "$USER_B" --arg e "${USER_B}@example.com" --arg p "$PASSWD" --arg d "Bob Smoke" \
  '{username:$u,email:$e,password:$p,display_name:$d}')"
assert_http "register B" "$API_CODE" "201"

api_get /auth/me "$COOKIE_A"
assert_http "A /me" "$API_CODE" "200"
assert_eq "A me username" "$(j '.user.username')" "$USER_A"

# ── API: category + thread + reply ──────────────────────────────────────────
section "API forum CRUD + reply-to"
api_json POST /categories "$COOKIE_A" "$(jq -n --arg n "$CAT_NAME" '{name:$n,description:"smoke cat"}')"
assert_http "create category" "$API_CODE" "201"
CAT_SLUG="$(j '.category.slug')"
assert_contains "category slug" "$CAT_SLUG" "smoke"

api_json POST "/categories/${CAT_SLUG}/threads" "$COOKIE_A" \
  "$(jq -n '{title:"How do I smoke test?",body:"Opening **question** for e2e."}')"
assert_http "create thread" "$API_CODE" "201"
THREAD_SLUG="$(j '.thread.slug')"
POST_OP_ID="$(j '.first_post.id')"
assert_contains "thread has markdown html" "$(j '.first_post.body_html')" "<strong>question</strong>"

api_json POST "/categories/${CAT_SLUG}/threads/${THREAD_SLUG}/posts" "$COOKIE_B" \
  "$(jq -n --argjson rt "$POST_OP_ID" '{body:"Here is a **helpful** reply.",reply_to_post_id:$rt}')"
assert_http "create reply with reply_to" "$API_CODE" "201"
POST_REPLY_ID="$(j '.post.id')"
assert_eq "reply_to set" "$(j '.post.reply_to_post_id')" "$POST_OP_ID"

api_get "/categories/${CAT_SLUG}/threads/${THREAD_SLUG}/posts?limit=50"
assert_http "list posts" "$API_CODE" "200"
assert_eq "post count" "$(j '.posts | length')" "2"

# ── API: votes ──────────────────────────────────────────────────────────────
section "API Me too + Helpful"
api_json POST "/categories/${CAT_SLUG}/threads/${THREAD_SLUG}/me-too" "$COOKIE_B"
assert_http "B me-too" "$API_CODE" "200"
assert_eq "me-too count" "$(j '.count')" "1"
assert_eq "viewer_voted" "$(j '.viewer_voted')" "true"

set +e
api_json POST "/categories/${CAT_SLUG}/threads/${THREAD_SLUG}/me-too" "$COOKIE_A"
set -e
assert_http "A me-too own rejected" "$API_CODE" "400"

api_json POST "/categories/${CAT_SLUG}/threads/${THREAD_SLUG}/posts/${POST_REPLY_ID}/helpful" "$COOKIE_A"
assert_http "A marks B helpful" "$API_CODE" "200"
assert_eq "helpful count" "$(j '.count')" "1"

set +e
api_json POST "/categories/${CAT_SLUG}/threads/${THREAD_SLUG}/posts/${POST_REPLY_ID}/helpful" "$COOKIE_B"
set -e
assert_http "self-helpful rejected" "$API_CODE" "400"

api_get "/categories/${CAT_SLUG}/threads/${THREAD_SLUG}" "$COOKIE_B"
assert_http "get thread viewer_me_too" "$API_CODE" "200"
assert_eq "viewer_me_too" "$(j '.viewer_me_too')" "true"
assert_eq "me_too_count" "$(j '.thread.me_too_count')" "1"

api_get "/categories/${CAT_SLUG}/threads/${THREAD_SLUG}/posts?limit=50" "$COOKIE_A"
if [[ "$API_CODE" == "200" ]]; then
  hcount="$(printf '%s\n' "$API_BODY" | jq -r --argjson id "$POST_REPLY_ID" '.posts[]? | select(.id==$id) | .helpful_count // empty')"
  vmark="$(printf '%s\n' "$API_BODY" | jq -r --argjson id "$POST_REPLY_ID" '.posts[]? | select(.id==$id) | .viewer_marked_helpful // empty')"
  assert_eq "list helpful_count" "$hcount" "1"
  assert_eq "list viewer_marked_helpful" "$vmark" "true"
else
  bad "list posts for helpful flags HTTP $API_CODE"
fi

# ── API: edit + soft-delete ─────────────────────────────────────────────────
section "API edit + soft-delete"
api_json PATCH "/categories/${CAT_SLUG}/threads/${THREAD_SLUG}/posts/${POST_REPLY_ID}" "$COOKIE_B" \
  "$(jq -n '{body:"Edited **helpful** reply."}')"
assert_http "edit own reply" "$API_CODE" "200"
assert_contains "edited html" "$(j '.post.body_html')" "Edited"
edited_at="$(j '.post.edited_at // empty')"
if [[ -n "$edited_at" && "$edited_at" != "null" ]]; then
  ok "edited_at set ($edited_at)"
else
  bad "edited_at missing"
fi

api_json POST "/categories/${CAT_SLUG}/threads/${THREAD_SLUG}/posts" "$COOKIE_B" \
  "$(jq -n '{body:"ephemeral reply"}')"
assert_http "second reply" "$API_CODE" "201"
DEL_ID="$(j '.post.id')"
api_json DELETE "/categories/${CAT_SLUG}/threads/${THREAD_SLUG}/posts/${DEL_ID}" "$COOKIE_B"
assert_http "soft-delete reply" "$API_CODE" "204"

api_get "/categories/${CAT_SLUG}/threads/${THREAD_SLUG}/posts?limit=50"
deleted_flag="$(printf '%s\n' "$API_BODY" | jq -r --argjson id "$DEL_ID" '.posts[] | select(.id==$id) | .is_deleted')"
assert_eq "deleted post flag" "$deleted_flag" "true"

# ── API: profile + drafts ───────────────────────────────────────────────────
section "API profile + drafts"
api_json PATCH /users/me "$COOKIE_A" "$(jq -n '{display_name:"Alice Updated",bio:"Smoke bio A"}')"
assert_http "update profile" "$API_CODE" "200"
assert_eq "display_name" "$(j '.display_name')" "Alice Updated"
assert_eq "bio" "$(j '.bio')" "Smoke bio A"

api_get "/users/${USER_A}"
assert_http "public profile" "$API_CODE" "200"
assert_eq "profile bio" "$(j '.profile.user.bio')" "Smoke bio A"
rep="$(j '.profile.reputation')"
if [[ "$rep" =~ ^[0-9]+$ ]]; then
  ok "reputation is number ($rep)"
else
  bad "reputation invalid ($rep)"
fi

api_json PUT /drafts "$COOKIE_A" "$(jq -n \
  --arg c "$CAT_SLUG" \
  '{kind:"thread",category_slug:$c,title:"Draft title",body:"draft body"}')"
assert_http "upsert draft" "$API_CODE" "200"
DRAFT_ID="$(j '.draft.id')"

api_get /drafts "$COOKIE_A"
assert_http "list drafts" "$API_CODE" "200"
dc="$(printf '%s\n' "$API_BODY" | jq --argjson id "$DRAFT_ID" '[.drafts[] | select(.id==$id)] | length')"
assert_eq "draft present" "$dc" "1"

api_json DELETE "/drafts/${DRAFT_ID}" "$COOKIE_A"
assert_http "delete draft" "$API_CODE" "204"

# ── API: admin ──────────────────────────────────────────────────────────────
section "API admin"
api_get /admin/users "$COOKIE_A"
assert_http "non-admin forbidden" "$API_CODE" "403"

if command -v sqlite3 >/dev/null && [[ -f "$DB" ]]; then
  sqlite3 "$DB" "UPDATE users SET role='admin' WHERE username='${USER_A}';"
  ok "promoted $USER_A to admin via sqlite"
  api_get /admin/users "$COOKIE_A"
  assert_http "admin list users" "$API_CODE" "200"
  total="$(j '.total')"
  if [[ "$total" =~ ^[0-9]+$ ]] && [[ "$total" -ge 2 ]]; then
    ok "admin total users ($total)"
  else
    bad "admin total unexpected ($total)"
  fi
  B_ID="$(printf '%s\n' "$API_BODY" | jq -r --arg u "$USER_B" '.users[] | select(.username==$u) | .id')"
  api_json PATCH "/admin/users/${B_ID}" "$COOKIE_A" '{"role":"moderator"}'
  assert_http "promote B to moderator" "$API_CODE" "200"
  assert_eq "B role" "$(j '.role')" "moderator"
else
  bad "sqlite3 or db missing — skipped admin promote path ($DB)"
fi

# ── Frontend BFF smoke ──────────────────────────────────────────────────────
# Frontend SSR fans out to the API (me + category + thread + posts + csrf).
# Give the shared IP rate-limit window room after the API phase.
if [[ "${SMOKE_NO_PAUSE:-}" != "1" ]]; then
  section "Cooldowning for rate-limit window before UI phase"
  echo "  sleeping ${SMOKE_UI_PAUSE:-45}s…"
  sleep "${SMOKE_UI_PAUSE:-45}"
fi

section "Frontend pages (public)"
web_get "/"
assert_http "GET /" "$WEB_CODE" "200"
assert_contains "home has Browse" "$WEB_BODY" "Browse"

web_get "/search?q=smoke"
assert_http "GET /search" "$WEB_CODE" "200"

web_get "/login"
assert_http "GET /login" "$WEB_CODE" "200"
assert_contains "login form" "$WEB_BODY" 'action="/api/login"'

web_get "/register"
assert_http "GET /register" "$WEB_CODE" "200"

section "Frontend register + session"
rm -f "$COOKIE_WEB"
web_get "/register" "$COOKIE_WEB"
assert_http "register page" "$WEB_CODE" "200"
CSRF="$(extract_csrf_html "$TMP/web.html")"
if [[ -n "$CSRF" ]]; then
  ok "csrf_token from /register"
else
  bad "csrf_token not found on /register"
  CSRF="missing"
fi

web_form POST /api/register "$COOKIE_WEB" \
  "csrf_token=${CSRF}" \
  "username=${USER_C}" \
  "email=${USER_C}@example.com" \
  "password=${PASSWD}" \
  "display_name=Carol Smoke"
if [[ "$WEB_CODE" == "303" || "$WEB_CODE" == "302" || "$WEB_CODE" == "200" ]]; then
  ok "register BFF status $WEB_CODE"
else
  bad "register BFF status $WEB_CODE body=$(printf '%s' "$WEB_BODY" | head -c 200)"
fi

web_get "/" "$COOKIE_WEB"
assert_http "home after register" "$WEB_CODE" "200"
if [[ "$WEB_BODY" == *"$USER_C"* || "$WEB_BODY" == *"Carol"* ]]; then
  ok "session user visible on home"
else
  bad "session user not visible on home"
fi

section "Frontend ask + draft BFF"
sleep 1
web_get "/ask" "$COOKIE_WEB"
assert_http "GET /ask" "$WEB_CODE" "200"
CSRF="$(extract_csrf_html "$TMP/web.html")"
assert_contains "ask form" "$WEB_BODY" 'action="/api/ask"'

web_form POST /api/drafts "$COOKIE_WEB" \
  "csrf_token=${CSRF}" \
  "kind=thread" \
  "category_slug=${CAT_SLUG}" \
  "category=${CAT_SLUG}" \
  "title=UI draft title" \
  "body=UI draft body content" \
  "back=/ask"
if [[ "$WEB_CODE" == "303" || "$WEB_CODE" == "302" ]]; then
  ok "save draft redirect $WEB_CODE"
else
  bad "save draft status $WEB_CODE"
fi

web_get "/ask?draft=saved" "$COOKIE_WEB"
if [[ "$WEB_BODY" == *"Draft saved"* || "$WEB_BODY" == *"UI draft body"* ]]; then
  ok "draft restored / saved UI"
else
  bad "draft not reflected on /ask"
fi

web_get "/ask" "$COOKIE_WEB"
CSRF="$(extract_csrf_html "$TMP/web.html")"
web_form POST /api/ask "$COOKIE_WEB" \
  "csrf_token=${CSRF}" \
  "category=${CAT_SLUG}" \
  "title=UI published question ${SUF}" \
  "body=Published via **frontend** BFF."
if [[ "$WEB_CODE" == "303" || "$WEB_CODE" == "302" ]]; then
  ok "ask publish redirect $WEB_CODE"
else
  bad "ask publish status $WEB_CODE"
fi

THREAD_PATH=""
if [[ -n "${WEB_REDIRECT:-}" && "$WEB_REDIRECT" == *"/threads/"* ]]; then
  THREAD_PATH="$(python3 -c "from urllib.parse import urlparse; print(urlparse('''$WEB_REDIRECT''').path)")"
fi
if [[ -z "$THREAD_PATH" ]]; then
  api_get "/categories/${CAT_SLUG}/threads?limit=5"
  tslug="$(j '.threads[0].slug // empty')"
  if [[ -n "$tslug" ]]; then
    THREAD_PATH="/categories/${CAT_SLUG}/threads/${tslug}"
    ok "resolved thread path via API $THREAD_PATH"
  else
    bad "could not resolve published thread path"
  fi
else
  ok "thread path $THREAD_PATH"
fi

section "Frontend thread UI + votes BFF"
sleep 2
if [[ -n "$THREAD_PATH" ]]; then
  web_get "$THREAD_PATH" "$COOKIE_WEB"
  assert_http "GET published thread" "$WEB_CODE" "200"
  assert_contains "Me too control" "$WEB_BODY" "Me too"
  if [[ "$WEB_BODY" == *"Add a reply"* || "$WEB_BODY" == *"Post reply"* ]]; then
    ok "reply composer present"
  else
    bad "reply composer missing"
  fi
  if [[ "$WEB_BODY" == *"<strong>frontend</strong>"* || "$WEB_BODY" == *"frontend"* ]]; then
    ok "published body visible"
  else
    bad "published body missing"
  fi
fi

API_THREAD="/categories/${CAT_SLUG}/threads/${THREAD_SLUG}"
web_get "$API_THREAD" "$COOKIE_WEB"
assert_http "GET API-created thread via UI" "$WEB_CODE" "200"
assert_contains "helpful control" "$WEB_BODY" "Helpful"
if [[ "$WEB_BODY" == *"Top-ranking reply"* || "$WEB_BODY" == *"helpful"* || "$WEB_BODY" == *"Edited"* ]]; then
  ok "reply content present"
else
  bad "reply content missing on thread page"
fi

CSRF="$(extract_csrf_html "$TMP/web.html")"
web_form POST "/api/categories/${CAT_SLUG}/threads/${THREAD_SLUG}/me-too" "$COOKIE_WEB" \
  "csrf_token=${CSRF}" \
  "action=add"
if [[ "$WEB_CODE" == "303" || "$WEB_CODE" == "302" ]]; then
  ok "me-too BFF redirect $WEB_CODE"
else
  bad "me-too BFF status $WEB_CODE"
fi

web_get "$API_THREAD" "$COOKIE_WEB"
assert_contains "me too still shown" "$WEB_BODY" "Me too"

CSRF="$(extract_csrf_html "$TMP/web.html")"
web_form POST "/api/categories/${CAT_SLUG}/threads/${THREAD_SLUG}/posts/${POST_REPLY_ID}/helpful" "$COOKIE_WEB" \
  "csrf_token=${CSRF}" \
  "action=add"
if [[ "$WEB_CODE" == "303" || "$WEB_CODE" == "302" ]]; then
  ok "helpful BFF redirect $WEB_CODE"
else
  bad "helpful BFF status $WEB_CODE body=$(printf '%s' "$WEB_BODY" | head -c 120)"
fi

CSRF="$(extract_csrf_html "$TMP/web.html")"
web_form POST "/api/categories/${CAT_SLUG}/threads/${THREAD_SLUG}/posts" "$COOKIE_WEB" \
  "csrf_token=${CSRF}" \
  "body=UI reply quoting OP" \
  "reply_to_post_id=${POST_OP_ID}"
if [[ "$WEB_CODE" == "303" || "$WEB_CODE" == "302" ]]; then
  ok "reply BFF redirect $WEB_CODE"
else
  bad "reply BFF status $WEB_CODE"
fi
web_get "$API_THREAD" "$COOKIE_WEB"
assert_contains "UI reply visible" "$WEB_BODY" "UI reply quoting OP"
assert_contains "in response to" "$WEB_BODY" "in response to"

section "Frontend profile + settings + admin"
sleep 2
web_get "/u/${USER_A}" "$COOKIE_WEB"
assert_http "profile page" "$WEB_CODE" "200"
if [[ "$WEB_BODY" == *"Alice"* || "$WEB_BODY" == *"$USER_A"* ]]; then
  ok "profile shows user"
else
  bad "profile missing user name"
fi
assert_contains "reputation" "$WEB_BODY" "Reputation"
assert_contains "bio" "$WEB_BODY" "Smoke bio A"

web_get "/settings/profile" "$COOKIE_WEB"
assert_http "settings (logged in)" "$WEB_CODE" "200"
assert_contains "edit form" "$WEB_BODY" 'action="/api/profile"'
CSRF="$(extract_csrf_html "$TMP/web.html")"
web_form POST /api/profile "$COOKIE_WEB" \
  "csrf_token=${CSRF}" \
  "display_name=Carol Updated" \
  "bio=Carol bio from UI"
if [[ "$WEB_CODE" == "303" || "$WEB_CODE" == "302" ]]; then
  ok "profile update redirect $WEB_CODE"
else
  bad "profile update status $WEB_CODE"
fi
web_get "/u/${USER_C}" "$COOKIE_WEB"
assert_contains "updated display" "$WEB_BODY" "Carol Updated"
assert_contains "updated bio" "$WEB_BODY" "Carol bio from UI"

web_get "/admin" "$COOKIE_WEB"
if [[ "$WEB_CODE" == "403" ]]; then
  ok "non-admin /admin → 403"
else
  bad "expected 403 for /admin as non-admin, got $WEB_CODE"
fi

rm -f "$COOKIE_ADMIN"
web_get "/login" "$COOKIE_ADMIN"
CSRF="$(extract_csrf_html "$TMP/web.html")"
web_form POST /api/login "$COOKIE_ADMIN" \
  "csrf_token=${CSRF}" \
  "login=${USER_A}" \
  "password=${PASSWD}"
if [[ "$WEB_CODE" == "303" || "$WEB_CODE" == "302" || "$WEB_CODE" == "200" ]]; then
  ok "admin login BFF $WEB_CODE"
else
  bad "admin login status $WEB_CODE"
fi
web_get "/admin" "$COOKIE_ADMIN"
assert_http "admin page" "$WEB_CODE" "200"
assert_contains "admin users heading" "$WEB_BODY" "Users"
if [[ "$WEB_BODY" == *"$USER_B"* || "$WEB_BODY" == *"$USER_A"* ]]; then
  ok "admin lists smoke users"
else
  bad "admin page missing smoke users"
fi

# ── summary ─────────────────────────────────────────────────────────────────
echo
echo "────────────────────────────────────────"
echo "Smoke e2e finished: ${PASS} passed, ${FAIL} failed"
echo "Users: $USER_A (admin), $USER_B, $USER_C"
echo "Category: $CAT_SLUG  Thread: $THREAD_SLUG"
echo "────────────────────────────────────────"
if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
exit 0
