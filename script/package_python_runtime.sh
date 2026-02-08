#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
SRC_VENV="${1:-$ROOT_DIR/.venv}"
DEST_RT="$ROOT_DIR/src-tauri/python-runtime"
SRC_COSYVOICE="$ROOT_DIR/third_party/CosyVoice"
DEST_COSYVOICE="$ROOT_DIR/src-tauri/third_party/CosyVoice"

if [[ ! -x "$SRC_VENV/bin/python3" ]]; then
  echo "error: python runtime not found at $SRC_VENV/bin/python3"
  echo "hint: create venv first, e.g. python3.10 -m venv .venv"
  exit 1
fi

echo "packing python runtime..."
rm -rf "$DEST_RT"
mkdir -p "$DEST_RT"
cp -a "$SRC_VENV"/. "$DEST_RT"/

# Strip caches to reduce bundle size
find "$DEST_RT" -type d -name "__pycache__" -prune -exec rm -rf {} +
find "$DEST_RT" -type f -name "*.pyc" -delete

if [[ -d "$SRC_COSYVOICE/cosyvoice" ]]; then
  echo "syncing CosyVoice source..."
  mkdir -p "$(dirname "$DEST_COSYVOICE")"
  rm -rf "$DEST_COSYVOICE"
  mkdir -p "$DEST_COSYVOICE"
  cp -a "$SRC_COSYVOICE/cosyvoice" "$DEST_COSYVOICE/"
  if [[ -d "$SRC_COSYVOICE/asset" ]]; then
    cp -a "$SRC_COSYVOICE/asset" "$DEST_COSYVOICE/"
  fi
  if [[ -d "$SRC_COSYVOICE/third_party/Matcha-TTS/matcha" ]]; then
    mkdir -p "$DEST_COSYVOICE/third_party/Matcha-TTS"
    cp -a "$SRC_COSYVOICE/third_party/Matcha-TTS/matcha" "$DEST_COSYVOICE/third_party/Matcha-TTS/"
  fi
fi

echo "done: $DEST_RT"
echo "python: $DEST_RT/bin/python3"
