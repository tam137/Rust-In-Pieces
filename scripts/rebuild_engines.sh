#!/usr/bin/env bash
# Rebuilds released engines from their git tags with the current build flags (scripts/build_flags.sh)
# into the Matt-Magie engines directory, under the names build_and_release.sh deploys them as.
#
# Every engine in a match must be built with the same flags. Releases up to v0.54.0 were built
# without -C target-cpu; on x86-64 the x86-64-v3 build of the same source is about 11% faster, so a
# new release against an old binary measures the flag. Rebuild the opponents a gauntlet uses once,
# on the host that plays it.
#
# Usage:
#   scripts/rebuild_engines.sh <tag> [<tag> ...]      e.g. v0.54.0-NNUE v0.53.0-NNUE v0.52.0
#
# Environment:
#   MM_DIR             Matt-Magie directory (default: ../matt-magie, relative to the repository)
#   SUPRAH_TARGET_CPU  target-cpu level, see scripts/build_flags.sh
#
# An existing binary of the same name is moved once to <mm>/engines/baseline-x86-64/ before it is
# replaced, so runs made with it stay reproducible; a later rebuild overwrites without moving again.
# Each tag is built in a temporary git worktree, so the working tree is never touched.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"
source scripts/build_flags.sh

if [ "$#" -eq 0 ]; then
    sed -n '2,20p' "$0"
    exit 2
fi

MM_ROOT="${MM_DIR:-../matt-magie}"
if [ ! -f "$MM_ROOT/mm.sh" ]; then
    echo "Error: '$MM_ROOT' is not a Matt-Magie directory (no mm.sh). Set MM_DIR." >&2
    exit 1
fi
ENGINES="$MM_ROOT/engines"
BACKUP="$ENGINES/baseline-x86-64"

if [ -z "$(suprah_target_cpu)" ]; then
    echo "No target-cpu level on this host: the rebuilds would equal the existing binaries. Nothing to do."
    exit 0
fi
suprah_apply_build_flags

for tag in "$@"; do
    git rev-parse -q --verify "refs/tags/$tag" >/dev/null || { echo "Error: no tag '$tag'." >&2; exit 1; }
done

WORK="$(mktemp -d)"
trap 'git worktree remove --force "$WORK/src" 2>/dev/null || true; rm -rf "$WORK"' EXIT
# One target directory for every tag: dependencies are compiled once.
TARGET="$REPO_ROOT/target/rebuild"

for tag in "$@"; do
    name="$(suprah_engine_name "$tag")"
    echo "== $tag -> $ENGINES/$name"
    git worktree add -q --detach "$WORK/src" "$tag"
    (cd "$WORK/src" && cargo build --release --target-dir "$TARGET")
    if [ -f "$ENGINES/$name" ] && [ ! -f "$BACKUP/$name" ]; then
        mkdir -p "$BACKUP"
        mv "$ENGINES/$name" "$BACKUP/$name"
        echo "   previous binary kept as $BACKUP/$name"
    fi
    cp "$TARGET/release/suprah" "$ENGINES/$name"
    chmod +x "$ENGINES/$name"
    git worktree remove --force "$WORK/src"
    id_name="$( (echo uci; sleep 1; echo quit) | "$ENGINES/$name" 2>/dev/null | grep -m1 '^id name' || true)"
    echo "   ${id_name:-no id name reported}"
done
