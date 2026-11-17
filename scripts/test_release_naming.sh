#!/usr/bin/env bash
# Tests for scripts/release_naming.sh.
# Run with `bash scripts/test_release_naming.sh`; exit status 0 when every case passes.

source "$(dirname "$0")/release_naming.sh"

failures=0

check() {
    local expected="$1" version="$2" branch="$3" actual
    actual="$(release_name "$version" "$branch")"
    if [ "$actual" = "$expected" ]; then
        echo "ok:   release_name '$version' '$branch' -> '$actual'"
    else
        echo "FAIL: release_name '$version' '$branch' -> '$actual', expected '$expected'"
        failures=$((failures + 1))
    fi
}

# HCE releases on master keep the plain version.
check "0.54.0" "0.54.0" "master"
# NNUE releases carry the suffix the tag, the CHANGELOG header and the release title use.
check "0.54.0-NNUE" "0.54.0" "feature/nnue-evaluation"
# A version that already carries the suffix is not suffixed twice.
check "0.54.0-NNUE" "0.54.0-NNUE" "feature/nnue-evaluation"
# Any other branch (a work branch, a detached HEAD) is treated as HCE.
check "0.54.0" "0.54.0" "HEAD"
check "0.54.0" "0.54.0" "feature/some-experiment"

if [ "$failures" -ne 0 ]; then
    echo "$failures failure(s)"
    exit 1
fi
echo "all passed"
