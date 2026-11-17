#!/bin/bash
#
# Trains a network and survives GPU resets of the integrated GPU (see README, "Helper scripts").
#
# The trainer saves a checkpoint every <save> superbatches. When it dies (a GPU reset kills the
# process), this script waits briefly and resumes from the newest complete checkpoint; without one it
# starts over. After a successful run only the final checkpoint is kept, unless a checkpoint holds a
# MANIFEST.json. Next to the trainer it starts two helpers that end with this script:
#   gpu_watch.sh   logs queue evictions and memory counters to <out>/gpuwatch/
#   cache_trim.py  keeps the page cache of the training file small (the remedy for the resets)
#
# Usage:
#   nnue_training/scripts/train_resilient.sh <name> <wdl> <superbatches> <data file> [save rate]
#
# Environment:
#   TRAINER     trainer binary (default: ../target/release/suprah-nnue-trainer next to this script)
#   OUT_DIR     where <name>/<name>-<sb>/ checkpoints and logs go (default: current directory)
#   NET_BYTES   size of a complete quantised.bin, checked before cleaning up (default: 3940928)
#   MAX_TRIES   attempts before giving up (default: 10)
#   GPU_WATCH   0 disables gpu_watch.sh (default: 1)
#   CACHE_TRIM  0 disables cache_trim.py (default: 1)
#   HSA_OVERRIDE_GFX_VERSION is passed through untouched; set it yourself where ROCm needs it.
#
# Exit status: 0 when the final checkpoint exists, 1 when all attempts failed, 2 on bad arguments.

set -u

if [ "$#" -lt 4 ]; then
    sed -n '3,24p' "$0" | sed 's/^# \{0,1\}//'
    exit 2
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
NAME="$1"; WDL="$2"; SB="$3"; DATA="$(realpath "$4")"; SAVE="${5:-2}"
TRAINER="$(realpath "${TRAINER:-$SCRIPT_DIR/../target/release/suprah-nnue-trainer}")"
NET_BYTES="${NET_BYTES:-3940928}"
MAX_TRIES="${MAX_TRIES:-10}"

[ -x "$TRAINER" ] || { echo "error: trainer $TRAINER is not executable (set TRAINER)" >&2; exit 2; }
[ -f "$DATA" ] || { echo "error: data file $DATA not found" >&2; exit 2; }

cd "${OUT_DIR:-.}" || exit 2
LOG="${NAME}_run.log"; RESUMES="${NAME}_resumes.txt"

# Keep only the final checkpoint; intermediate ones exist to resume from.
cleanup() {
    local final="$NAME/$NAME-$SB"
    if [ "$(stat -c %s "$final/quantised.bin" 2>/dev/null)" != "$NET_BYTES" ]; then
        echo "$(date +%F_%T) cleanup skipped: $final/quantised.bin missing or not $NET_BYTES bytes" >> "$RESUMES"
        return
    fi
    for c in "$NAME/$NAME-"*/; do
        c="${c%/}"
        [ "$c" = "$final" ] || [ -f "$c/MANIFEST.json" ] || rm -rf -- "$c"
    done
    echo "$(date +%F_%T) cleanup: kept $final" >> "$RESUMES"
}

: > "$RESUMES"
if [ "${GPU_WATCH:-1}" != 0 ]; then
    "$SCRIPT_DIR/gpu_watch.sh" "$NAME" $$ 5 gpuwatch > /dev/null 2>&1 &
fi
if [ "${CACHE_TRIM:-1}" != 0 ]; then
    mkdir -p gpuwatch
    python3 "$SCRIPT_DIR/cache_trim.py" "$DATA" $$ "gpuwatch/${NAME}_trim.log" 20 > /dev/null 2>&1 &
fi

for try in $(seq 0 $((MAX_TRIES - 1))); do
    last=$(ls -d "$NAME/$NAME-"* 2>/dev/null | sed "s|.*-||" | sort -n | tail -1)
    if [ -n "$last" ] && [ -f "$NAME/$NAME-$last/quantised.bin" ] && [ -d "$NAME/$NAME-$last/optimiser_state" ]; then
        [ "$last" -ge "$SB" ] && { cleanup; exit 0; }
        echo "$(date +%F_%T) resume from $NAME-$last (try $try)" >> "$RESUMES"
        "$TRAINER" -d "$DATA" -s "$SB" --wdl "$WDL" -n "$NAME" -o "$NAME" --save-rate "$SAVE" \
            --start $((last + 1)) --resume "$NAME/$NAME-$last" >> "$LOG" 2>&1
    else
        [ "$try" -gt 0 ] && echo "$(date +%F_%T) restart from scratch (try $try)" >> "$RESUMES"
        "$TRAINER" -d "$DATA" -s "$SB" --wdl "$WDL" -n "$NAME" -o "$NAME" --save-rate "$SAVE" >> "$LOG" 2>&1
    fi
    [ $? -eq 0 ] && { cleanup; exit 0; }
    sleep 20   # let the GPU reset settle
done
exit 1
