#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-v1.0.0}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "=== Membangun Paket Rilis Linux ($VERSION) ==="
cargo build --release

PKG_NAME="karir-ai-${VERSION}-linux-x86_64"
DIST_DIR="$ROOT/dist"
STAGE_DIR="$DIST_DIR/$PKG_NAME"

rm -rf "$STAGE_DIR"
mkdir -p "$STAGE_DIR/data/processed"

cp "$ROOT/target/release/karir-ai" "$STAGE_DIR/"
cp -r "$ROOT/static" "$STAGE_DIR/static"
cp "$ROOT/data/processed/"* "$STAGE_DIR/data/processed/"
cp "$ROOT/firebase.json" "$ROOT/.firebaserc" "$ROOT/firestore.rules" "$ROOT/.env.example" "$ROOT/README.md" "$STAGE_DIR/"
cp "$ROOT/run.sh" "$STAGE_DIR/start.sh"
chmod +x "$STAGE_DIR/karir-ai" "$STAGE_DIR/start.sh"

mkdir -p "$DIST_DIR"
TAR_PATH="$DIST_DIR/${PKG_NAME}.tar.gz"
tar -C "$DIST_DIR" -czf "$TAR_PATH" "$PKG_NAME"
(cd "$DIST_DIR" && sha256sum "${PKG_NAME}.tar.gz" > "${PKG_NAME}.tar.gz.sha256")

echo "[OK] Arsip rilis Linux siap: $TAR_PATH"
