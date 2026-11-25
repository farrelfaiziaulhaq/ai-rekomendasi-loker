#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

USE_EMULATOR=0
USE_RELEASE=0
APP_ARGS=()

for arg in "$@"; do
  case "$arg" in
    --emulator) USE_EMULATOR=1 ;;
    --release)  USE_RELEASE=1 ;;
    --reset|--eval|--case-study) APP_ARGS+=("$arg") ;;
    *) APP_ARGS+=("$arg") ;;
  esac
done

EMU_PID=""
cleanup() {
  if [[ -n "$EMU_PID" ]] && kill -0 "$EMU_PID" 2>/dev/null; then
    echo "Mematikan Firebase Emulator (PID $EMU_PID)..."
    kill "$EMU_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM

if [[ "$USE_EMULATOR" -eq 1 ]]; then
  export FIRESTORE_EMULATOR_HOST="127.0.0.1:8088"
  export FIREBASE_AUTH_EMULATOR_HOST="127.0.0.1:9098"
  export FIREBASE_PROJECT_ID="demo-karir-ai"
  echo "[1/2] Menyalakan Firebase Emulator Suite (Auth :9098, Firestore :8088, UI :4040)..."
  firebase emulators:start --project demo-karir-ai --only auth,firestore >/tmp/karir_ai_firebase.log 2>&1 &
  EMU_PID=$!
  for _ in $(seq 1 30); do
    if curl -sf http://127.0.0.1:8088/ >/dev/null 2>&1; then
      echo "[OK] Cloud Firestore Emulator siap! Dashboard UI: http://127.0.0.1:4040"
      break
    fi
    sleep 1
  done
fi

if [[ -x "./karir-ai" ]]; then
  ./karir-ai "${APP_ARGS[@]}"
elif [[ "$USE_RELEASE" -eq 1 ]]; then
  cargo run --release -- "${APP_ARGS[@]}"
else
  cargo run -- "${APP_ARGS[@]}"
fi
