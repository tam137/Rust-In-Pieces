#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

MM_DIR="${MM_DIR:-/home/tam137/chess/mattmagie}"
ENGINE_BIN="${ENGINE_BIN:-$MM_DIR/engines/suprah-0.51.0}"
MM_BIN="${MM_BIN:-$MM_DIR/target/release/Matt-Magie}"
OPENINGS="${OPENINGS:-$MM_DIR/openings_wide.txt}"
GROUP="${GROUP:-search_and_ordering}"
WORKERS="${WORKERS:-12}"
ITERS="${ITERS:-20000}"

python3 spsa_tuner.py \
    --group "$GROUP" \
    --engine "$ENGINE_BIN" \
    --mm "$MM_BIN" \
    --openings "$OPENINGS" \
    --iters "$ITERS" \
    --workers "$WORKERS" \
    "$@"
