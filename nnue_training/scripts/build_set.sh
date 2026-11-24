#!/bin/bash
#
# Builds a shuffled bullet training set from datagen batches and writes its manifest (see README,
# "Helper scripts").
#
# Each source is a batch directory under <data dir> with part_<i>.txt files and a MANIFEST.json
# (a rip-datagen batch, sealed by scripts/nnue_manifest.py data). Every text file is converted to bullet's
# binary format, all files are interleaved and the result is shuffled into <data dir>/<set>.data;
# scripts/nnue_manifest.py set then records sources, record count and sha256 in <set>.data.json.
# An existing <set>.data.json means the set is complete and the script does nothing.
#
# Usage:
#   nnue_training/scripts/build_set.sh <set> <data dir> <source>... [--note TEXT]
#
# Environment:
#   BULLET_UTILS   bullet-utils binary (required)
#   SHUFFLE_MB     memory for bullet-utils shuffle in MB (default: 4096)
#   TOOL_NOTE      text for the manifest's tool field (default: names the bullet-utils steps)
#
# Exit status: 0 on success or when the set exists, 1 when a step failed, 2 on bad arguments.

set -uo pipefail

usage() { sed -n '3,20p' "$0" | sed 's/^# \{0,1\}//'; exit 2; }

[ "$#" -ge 3 ] || usage
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
MANIFEST="$SCRIPT_DIR/../../scripts/nnue_manifest.py"
SET="$1"; DATA_DIR="$2"; shift 2
SOURCES=(); NOTE=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        --note) NOTE="${2:-}"; shift 2 ;;
        *) SOURCES+=("$1"); shift ;;
    esac
done
[ "${#SOURCES[@]}" -gt 0 ] || usage
BU="${BULLET_UTILS:-}"
[ -x "$BU" ] || { echo "error: set BULLET_UTILS to the bullet-utils binary" >&2; exit 2; }
SHUFFLE_MB="${SHUFFLE_MB:-4096}"
TOOL_NOTE="${TOOL_NOTE:-bullet-utils: convert per file, interleave, shuffle --mem-used-mb $SHUFFLE_MB}"

cd "$DATA_DIR" || exit 2
[ -f "$SET.data.json" ] && { echo "$SET.data.json exists, nothing to do"; exit 0; }
for src in "${SOURCES[@]}"; do
    [ -f "$src/MANIFEST.json" ] || { echo "error: $src has no MANIFEST.json" >&2; exit 2; }
done

TMP="bin_$SET"
rm -rf "$TMP" && mkdir "$TMP" || exit 1
fail() { echo "error: $1" >&2; rm -rf "$TMP"; exit 1; }
for src in "${SOURCES[@]}"; do
    for f in "$src"/part_*.txt; do
        nice "$BU" convert --from text --input "$f" --output "$TMP/${src}_$(basename "$f" .txt).data" > /dev/null 2>&1 \
            || fail "convert $f"
    done
done
nice "$BU" interleave "$TMP"/*.data --output "$TMP/all.data" > /dev/null 2>&1 || fail "interleave"
nice "$BU" shuffle --input "$TMP/all.data" --output "$SET.data" --mem-used-mb "$SHUFFLE_MB" > /dev/null 2>&1 || fail "shuffle"
rm -rf "$TMP"

args=(); for src in "${SOURCES[@]}"; do args+=(--source "$src"); done
python3 "$MANIFEST" set "$SET.data" "${args[@]}" --tool "$TOOL_NOTE" ${NOTE:+--note "$NOTE"} || exit 1
