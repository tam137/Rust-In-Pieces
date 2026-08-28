#!/bin/bash
#
# Runs a Matt-Magie tournament and stops it as soon as an SPRT over one pairing is decided.
#
# Every measurement in `task.md` so far has played a game count fixed in advance, which means a
# run that is already conclusive keeps playing and a run that never will keeps playing too. This
# wrapper leaves the tournament configuration alone - set `rounds` generously - and ends it at
# the first decision.
#
# Usage:
#   scripts/run_sprt_match.sh <tournament.trn> <engineA> <engineB> [sprt.py options...]
#
#   <tournament.trn>   name of the file inside the Matt-Magie directory
#   <engineA>          substring of the challenger's UCI id name, e.g. BOTH
#   <engineB>          substring of the opponent's id name, e.g. LMP
#
# Anything after the two engines is handed to `scripts/sprt.py`, so the hypotheses are set there:
#   scripts/run_sprt_match.sh gauntlet_lmp.trn BOTH LMP --elo0 0 --elo1 10
#
# The pairing named is the only one that governs the stop. Other pairings in a gauntlet keep
# playing until that one decides, which is what makes a gauntlet's cross-version check free.

set -u

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# Sibling of the repository by default; override with MM_DIR on a host with another layout.
# See the path policy in AGENTS.md.
MM_DIR="${MM_DIR:-$REPO_ROOT/../matt-magie}"
POLL_SECONDS="${SPRT_POLL_SECONDS:-60}"

if [[ $# -lt 3 ]]; then
    sed -n '2,25p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
    exit 3
fi

TRN="$1"; ENGINE_A="$2"; ENGINE_B="$3"; shift 3

if [[ ! -d "$MM_DIR" ]]; then
    echo "Matt-Magie directory not found next to the repository: $MM_DIR" >&2
    exit 3
fi
if [[ ! -f "$MM_DIR/$TRN" ]]; then
    echo "tournament file not found: $MM_DIR/$TRN" >&2
    exit 3
fi

PGN_NAME="$(sed -n 's/^[[:space:]]*pgn[[:space:]]*=[[:space:]]*\([^[:space:]#]*\).*/\1/p' "$MM_DIR/$TRN" | tail -1)"
if [[ -z "$PGN_NAME" ]]; then
    echo "no 'pgn = ...' line in $TRN; the watchdog cannot find the games" >&2
    exit 3
fi
PGN="$MM_DIR/$PGN_NAME"

if [[ -f "$PGN" ]]; then
    echo "warning: $PGN_NAME already exists and Matt-Magie appends to it."
    echo "         Games from an earlier run with the same round total would be mixed into"
    echo "         this test. Move it aside first if that is not what you want."
    echo
fi

LOG="$MM_DIR/${PGN_NAME%.pgn}.sprt.log"
: > "$LOG"

echo "tournament : $TRN"
echo "pairing    : $ENGINE_A vs $ENGINE_B"
echo "pgn        : $PGN_NAME"
echo "trace      : ${PGN_NAME%.pgn}.sprt.log"
echo "poll       : every ${POLL_SECONDS}s"
echo

cd "$MM_DIR" || exit 3
setsid ./mm.sh -t "$TRN" > "${PGN_NAME%.pgn}.out" 2>&1 &
MM_PID=$!
# `setsid` makes the tournament its own process group, so one signal reaches every game rather
# than only the shell that scheduled them.
MM_PGID="$(ps -o pgid= -p "$MM_PID" 2>/dev/null | tr -d ' ')"
echo "tournament running as pid $MM_PID (process group $MM_PGID)"

stop_tournament() {
    if [[ -n "$MM_PGID" ]]; then
        kill -TERM "-$MM_PGID" 2>/dev/null
        for _ in $(seq 1 20); do
            kill -0 "-$MM_PGID" 2>/dev/null || return 0
            sleep 0.5
        done
        kill -KILL "-$MM_PGID" 2>/dev/null
    fi
}
trap 'echo; echo "interrupted, stopping the tournament"; stop_tournament; exit 130' INT TERM

VERDICT=2
while kill -0 "$MM_PID" 2>/dev/null; do
    sleep "$POLL_SECONDS"
    [[ -f "$PGN" ]] || continue

    OUTPUT="$(python3 "$REPO_ROOT/scripts/sprt.py" "$PGN" --engines "$ENGINE_A" "$ENGINE_B" "$@" 2>&1)"
    VERDICT=$?
    {
        date +"--- %H:%M:%S"
        echo "$OUTPUT"
    } >> "$LOG"
    echo "$OUTPUT" | sed -n '2p;5p'

    if [[ "$VERDICT" -eq 0 || "$VERDICT" -eq 1 ]]; then
        echo
        echo "SPRT decided; stopping the tournament."
        stop_tournament
        break
    fi
done

wait "$MM_PID" 2>/dev/null
trap - INT TERM

echo
echo "=== final ==="
python3 "$REPO_ROOT/scripts/sprt.py" "$PGN" --engines "$ENGINE_A" "$ENGINE_B" "$@"
VERDICT=$?
echo
python3 "$REPO_ROOT/scripts/pairing_elo.py" "$PGN"
exit "$VERDICT"
