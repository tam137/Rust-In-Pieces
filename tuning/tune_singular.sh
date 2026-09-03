#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

ENGINE_BIN="${ENGINE_BIN:-../target/release/suprah}"
MM_BIN="${MM_BIN:-../../matt-magie/target/release/Matt-Magie}"
BOOK="${BOOK:-../Performance.bin}"
WORKERS="${WORKERS:-4}"
GAMES="${GAMES:-2500}"
ITERS="${ITERS:-100}"

echo "Starting SPSA tuning for singular group:"
echo "  Engine:  $ENGINE_BIN"
echo "  MM:      $MM_BIN"
echo "  Book:    $BOOK"
echo "  Workers: $WORKERS"
echo "  Games:   $GAMES"
echo "  Iters:   $ITERS"

python3 spsa_tuner.py \
    --group singular \
    --engine "$ENGINE_BIN" \
    --mm "$MM_BIN" \
    --book "$BOOK" \
    --games "$GAMES" \
    --workers "$WORKERS" \
    --time 1 \
    --inc 10 \
    --mutate 10.0 \
    --lr 5.0 \
    --iters "$ITERS"
