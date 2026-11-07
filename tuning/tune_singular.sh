#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

MM_DIR="${MM_DIR:-/home/tam137/chess/mattmagie}"
ENGINE_BIN="${ENGINE_BIN:-$MM_DIR/engines/suprah-0.51.0}"
MM_BIN="${MM_BIN:-$MM_DIR/target/release/Matt-Magie}"
OPENINGS="${OPENINGS:-$MM_DIR/openings_wide.txt}"
WORKERS="${WORKERS:-4}"
ITERS="${ITERS:-20000}"

echo "Starting SPSA tuning for singular group:"
echo "  Engine:   $ENGINE_BIN"
echo "  MM:       $MM_BIN"
echo "  Openings: $OPENINGS"
echo "  Workers:  $WORKERS"
echo "  Iters:    $ITERS"

python3 spsa_tuner.py \
    --group singular \
    --engine "$ENGINE_BIN" \
    --mm "$MM_BIN" \
    --openings "$OPENINGS" \
    --iters "$ITERS" \
    --workers "$WORKERS" \
    "$@"
