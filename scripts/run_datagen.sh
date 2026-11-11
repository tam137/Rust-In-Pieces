#!/bin/bash
#
# Runs several `suprah datagen` processes side by side (Matt-Magie step 102, stage A).
#
# Each process plays its own games with its own seed (base seed + process index) and appends to
# its own file, so processes never share a file and a run with the same arguments is reproducible.
# Interrupting the script (Ctrl+C or a signal) stops all processes; every file then ends with a
# whole game, because a process writes a game only when it is finished.
#
# Usage:
#   scripts/run_datagen.sh <datagen binary> <output dir> <processes> <games per process> <base seed> [datagen options...]
#
#   <datagen binary>     a build with the feature, e.g. target/datagen/release/suprah
#                        (cargo build --release --features datagen --target-dir target/datagen)
#   <output dir>         created if missing; gets part_<i>.txt and part_<i>.log per process
#   <processes>          at most 12 (compute rule of the match manager's host)
#   <games per process>  recorded games each process plays
#   <base seed>          process i uses seed base + i; use a new base for every new batch
#   [datagen options]    passed through, e.g. --nodes 5000 (see `<binary> datagen --help`)
#
# Example:
#   scripts/run_datagen.sh target/datagen/release/suprah ../data/n1 12 100000 1000

set -euo pipefail

MAX_PROCESSES=12

if [ "$#" -lt 5 ]; then
    sed -n '3,22p' "$0" | sed 's/^# \{0,1\}//'
    exit 2
fi

BINARY="$1"
OUT_DIR="$2"
PROCESSES="$3"
GAMES="$4"
BASE_SEED="$5"
shift 5

if [ ! -x "$BINARY" ]; then
    echo "error: $BINARY is not an executable" >&2
    exit 2
fi
# A build without the feature ignores the arguments and speaks UCI; stdin from /dev/null ends it.
if ! "$BINARY" datagen --help </dev/null 2>/dev/null | grep -q -- "--nodes"; then
    echo "error: $BINARY has no datagen mode (build it with --features datagen)" >&2
    exit 2
fi
if ! [[ "$PROCESSES" =~ ^[0-9]+$ ]] || [ "$PROCESSES" -lt 1 ] || [ "$PROCESSES" -gt "$MAX_PROCESSES" ]; then
    echo "error: processes must lie between 1 and $MAX_PROCESSES" >&2
    exit 2
fi
for value in "$GAMES" "$BASE_SEED"; do
    if ! [[ "$value" =~ ^[0-9]+$ ]]; then
        echo "error: games and base seed must be non-negative integers" >&2
        exit 2
    fi
done

mkdir -p "$OUT_DIR"

PIDS=()
stop_all() {
    echo "stopping ${#PIDS[@]} processes" >&2
    for pid in "${PIDS[@]}"; do
        kill -TERM "$pid" 2>/dev/null || true
    done
    wait 2>/dev/null || true
    exit 130
}
trap stop_all INT TERM

for i in $(seq 1 "$PROCESSES"); do
    "$BINARY" datagen --out "$OUT_DIR/part_$i.txt" --games "$GAMES" --seed $((BASE_SEED + i)) "$@" \
        > "$OUT_DIR/part_$i.log" 2>&1 &
    PIDS+=("$!")
done
echo "started $PROCESSES processes, seeds $((BASE_SEED + 1))..$((BASE_SEED + PROCESSES)), output in $OUT_DIR"

FAILED=0
for pid in "${PIDS[@]}"; do
    wait "$pid" || FAILED=$((FAILED + 1))
done

for i in $(seq 1 "$PROCESSES"); do
    tail -n 1 "$OUT_DIR/part_$i.log"
done
echo "positions in total: $(cat "$OUT_DIR"/part_*.txt | wc -l)"
if [ "$FAILED" -gt 0 ]; then
    echo "error: $FAILED processes failed, see the part_<i>.log files" >&2
    exit 1
fi
