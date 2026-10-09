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

if [[ -f "$PGN" && "${SPRT_APPEND:-0}" != "1" ]]; then
    # Matt-Magie appends, and a pair is two consecutive game numbers of the same round total, so
    # the games of an earlier run of the same tournament would be paired with this one's.
    echo "$PGN_NAME already exists and Matt-Magie appends to it: an earlier run's games would" >&2
    echo "be paired with this run's. Move it aside, or set SPRT_APPEND=1 to extend that run." >&2
    exit 3
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
# than only the shell that scheduled them. Without job control it calls setsid() and execs in
# place, so the group id becomes the PID - but only once the child has got that far. Reading the
# group straight after the launch can return this script's own group, and signalling that would
# stop the watchdog and leave the tournament running. Wait for the group to be the tournament's.
OWN_PGID="$(ps -o pgid= -p $$ 2>/dev/null | tr -d ' ')"
MM_PGID=""
for _ in $(seq 1 50); do
    if [[ "$(ps -o pgid= -p "$MM_PID" 2>/dev/null | tr -d ' ')" == "$MM_PID" ]]; then
        MM_PGID="$MM_PID"
        break
    fi
    sleep 0.1
done
if [[ -z "$MM_PGID" || "$MM_PGID" == "$OWN_PGID" ]]; then
    echo "the tournament did not get a process group of its own; stopping it" >&2
    kill -TERM "$MM_PID" 2>/dev/null
    exit 3
fi
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

# `sprt.py` exits 0 or 1 on a verdict, 2 while undecided and 3 on any error. Only a verdict
# stops the tournament as a result; an error stops it too, but is reported as one.
STOP_VERDICT=""
while kill -0 "$MM_PID" 2>/dev/null; do
    sleep "$POLL_SECONDS"
    # `mm.sh` creates the PGN empty at the start.
    [[ -s "$PGN" ]] || continue

    OUTPUT="$(python3 "$REPO_ROOT/scripts/sprt.py" "$PGN" --engines "$ENGINE_A" "$ENGINE_B" "$@" 2>&1)"
    VERDICT=$?
    {
        date +"--- %H:%M:%S"
        echo "$OUTPUT"
    } >> "$LOG"
    echo "$OUTPUT" | sed -n '1,2p;5p'

    if [[ "$VERDICT" -eq 0 || "$VERDICT" -eq 1 ]]; then
        echo
        echo "SPRT decided; stopping the tournament."
        STOP_VERDICT="$VERDICT"
        stop_tournament
        break
    fi
    if [[ "$VERDICT" -eq 3 ]]; then
        echo
        echo "sprt.py reported an error; stopping the tournament. See ${PGN_NAME%.pgn}.sprt.log." >&2
        STOP_VERDICT=3
        stop_tournament
        break
    fi
done

wait "$MM_PID" 2>/dev/null
trap - INT TERM

echo
echo "=== final ==="
python3 "$REPO_ROOT/scripts/sprt.py" "$PGN" --engines "$ENGINE_A" "$ENGINE_B" "$@"
FINAL_VERDICT=$?
echo
python3 "$REPO_ROOT/scripts/pairing_elo.py" "$PGN"
# The verdict that stopped the run is the run's verdict. Games that finished while the
# tournament was being stopped are in the report above, not in the exit code.
exit "${STOP_VERDICT:-$FINAL_VERDICT}"
