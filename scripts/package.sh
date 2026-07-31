#!/usr/bin/env bash
# Build a portable pre-release package (backend binary + frontend server + migrations).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="${VERSION:-0.1.0-alpha.1}"
TARGET="${TARGET:-$(rustc -vV | sed -n 's/^host: //p')}"
OUT_NAME="forum-${VERSION}-${TARGET}"
DIST_ROOT="${ROOT}/dist"
PKG_DIR="${DIST_ROOT}/${OUT_NAME}"
ARCHIVE="${DIST_ROOT}/${OUT_NAME}.tar.gz"

echo "==> Packaging ${OUT_NAME}"

rm -rf "${PKG_DIR}"
mkdir -p "${PKG_DIR}/backend" "${PKG_DIR}/frontend" "${PKG_DIR}/docs"

echo "==> Backend release build"
(
  cd "${ROOT}/backend"
  cargo build --release
)
cp "${ROOT}/backend/target/release/forum-backend" "${PKG_DIR}/backend/"
cp -R "${ROOT}/backend/migrations" "${PKG_DIR}/backend/migrations"
cp "${ROOT}/backend/.env.example" "${PKG_DIR}/backend/.env.example"

echo "==> Frontend production build"
(
  cd "${ROOT}/frontend"
  if [[ ! -d node_modules ]]; then
    npm ci
  fi
  npm run build
)
# Standalone server needs dist + production node_modules
cp -R "${ROOT}/frontend/dist" "${PKG_DIR}/frontend/dist"
cp "${ROOT}/frontend/package.json" "${PKG_DIR}/frontend/package.json"
cp "${ROOT}/frontend/package-lock.json" "${PKG_DIR}/frontend/package-lock.json"
cp "${ROOT}/frontend/.env.example" "${PKG_DIR}/frontend/.env.example"
(
  cd "${PKG_DIR}/frontend"
  npm ci --omit=dev
)

cp "${ROOT}/README.md" "${PKG_DIR}/"
cp "${ROOT}/docs/DEPLOY.md" "${PKG_DIR}/docs/" 2>/dev/null || true
cp "${ROOT}/LICENSE" "${PKG_DIR}/" 2>/dev/null || true

cat > "${PKG_DIR}/run-backend.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/backend"
if [[ ! -f .env ]]; then
  cp .env.example .env
  echo "Created backend/.env from example — edit JWT_SECRET before production use."
fi
set -a
# shellcheck disable=SC1091
source .env
set +a
exec ./forum-backend
EOF

cat > "${PKG_DIR}/run-frontend.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/frontend"
export HOST="${HOST:-0.0.0.0}"
export PORT="${PORT:-4321}"
export NODE_ENV=production
if [[ -f .env ]]; then
  set -a
  # shellcheck disable=SC1091
  source .env
  set +a
fi
exec node ./dist/server/entry.mjs
EOF

chmod +x "${PKG_DIR}/run-backend.sh" "${PKG_DIR}/run-frontend.sh" "${PKG_DIR}/backend/forum-backend"

cat > "${PKG_DIR}/VERSION" <<EOF
version=${VERSION}
target=${TARGET}
built_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)
EOF

echo "==> Creating archive ${ARCHIVE}"
mkdir -p "${DIST_ROOT}"
tar -C "${DIST_ROOT}" -czf "${ARCHIVE}" "${OUT_NAME}"

# Checksums
if command -v shasum >/dev/null 2>&1; then
  (cd "${DIST_ROOT}" && shasum -a 256 "$(basename "${ARCHIVE}")" > "$(basename "${ARCHIVE}").sha256")
elif command -v sha256sum >/dev/null 2>&1; then
  (cd "${DIST_ROOT}" && sha256sum "$(basename "${ARCHIVE}")" > "$(basename "${ARCHIVE}").sha256")
fi

echo "==> Done"
echo "    ${ARCHIVE}"
ls -lh "${ARCHIVE}" "${ARCHIVE}.sha256" 2>/dev/null || ls -lh "${ARCHIVE}"
