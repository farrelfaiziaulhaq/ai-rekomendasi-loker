#!/usr/bin/env bash
set -euo pipefail

echo "=== Setup Lingkungan KarirAI (Linux / macOS) ==="
if ! command -v cargo >/dev/null 2>&1; then
  echo "[1] Memasang Rust toolchain via rustup..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  # shellcheck disable=SC1090
  source "$HOME/.cargo/env"
else
  echo "[OK] Rust sudah terpasang: $(cargo --version)"
fi

if ! command -v firebase >/dev/null 2>&1 && command -v npm >/dev/null 2>&1; then
  echo "[Opsional] Memasang firebase-tools untuk mode Emulator..."
  npm install -g firebase-tools || true
fi

echo "[SELESAI] Jalankan './run.sh' (mode instan) atau './run.sh --emulator' (dengan Firebase Emulator :4000)."
