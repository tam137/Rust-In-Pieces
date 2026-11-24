#!/bin/bash
# Tests train_resilient.sh with a fake trainer: the hidden size reaches every trainer call, the
# expected network size comes from the trainer (or NET_BYTES), and only the final checkpoint is
# kept. No GPU, no data beyond an empty file. Run: nnue_training/scripts/test_train_resilient.sh
set -u
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
FAILS=0
check() { if eval "$2"; then echo "ok   $1"; else echo "FAIL $1"; FAILS=$((FAILS + 1)); fi; }

# The fake trainer answers `expected-size N` as the real one (formula of expected_quantised_bytes),
# logs every training call and writes checkpoints of the size it was asked for.
cat > "$WORK/trainer" <<'EOF'
#!/bin/bash
size() { local v=$(( 10 * 768 * $1 + $1 + 8 * 2 * $1 + 8 )); echo $(( (v * 2 + 63) / 64 * 64 )); }
if [ "$1" = "expected-size" ]; then size "$2"; exit 0; fi
echo "$*" >> "$CALLS"
hidden=256; sb=0; save=1; name=; out=
while [ $# -gt 0 ]; do
    case "$1" in
        --hidden) hidden=$2; shift ;; -s) sb=$2; shift ;; --save-rate) save=$2; shift ;;
        -n) name=$2; shift ;; -o) out=$2; shift ;; *) ;;
    esac
    shift
done
for ((i = save; i <= sb; i += save)); do
    mkdir -p "$out/$name-$i/optimiser_state"
    head -c "$(size "$hidden")" /dev/zero > "$out/$name-$i/quantised.bin"
done
EOF
chmod +x "$WORK/trainer"
: > "$WORK/data.data"

# run <name> [VAR=value ...]: one training of 4 superbatches, a checkpoint every 2.
run() {
    local name="$1"; shift
    (cd "$WORK" && env TRAINER="$WORK/trainer" OUT_DIR="$WORK" GPU_WATCH=0 CACHE_TRIM=0 \
        CALLS="$WORK/calls_$name" "$@" "$SCRIPT_DIR/train_resilient.sh" "$name" 0.2 4 "$WORK/data.data" 2)
}

run n512
check "exit 0 with the default HIDDEN" "[ \$? -eq 0 ]"
check "default hidden is 512 and reaches the trainer" "grep -q -- '--hidden 512' '$WORK/calls_n512'"
check "only the final checkpoint is kept" "[ \"\$(ls '$WORK/n512')\" = 'n512-4' ]"
check "cleanup accepted the 512 size" "grep -q 'cleanup: kept' '$WORK/n512_resumes.txt'"

run n256 HIDDEN=256
check "--hidden 256 reaches the trainer" "grep -q -- '--hidden 256' '$WORK/calls_n256'"
check "256 cleanup kept the final checkpoint" "[ \"\$(ls '$WORK/n256')\" = 'n256-4' ]"

run nwrong HIDDEN=512 NET_BYTES=3940928
check "an explicit NET_BYTES overrides and blocks a wrong-size cleanup" \
    "grep -q 'cleanup skipped' '$WORK/nwrong_resumes.txt' && [ \$(ls '$WORK/nwrong' | wc -l) -eq 2 ]"

[ "$FAILS" -eq 0 ] && echo "all passed" || { echo "$FAILS failed"; exit 1; }
