#!/usr/bin/env bash
# Release naming shared by build_and_release.sh and its test (sourced, not executed).
#
# Cargo.toml carries the plain version on both branches (e.g. 0.54.0). The NNUE release line
# names its tag, its CHANGELOG.md header and its GitHub release with the suffix -NNUE
# (v0.54.0-NNUE, `## [V0.54.0-NNUE]`); the release-binaries workflow and its CHANGELOG guard
# rely on that, so the suffix is derived here instead of being added by hand.

# The branch whose releases carry the -NNUE suffix.
NNUE_RELEASE_BRANCH="feature/nnue-evaluation"

# Prints the release name for a Cargo version on a branch: the version itself, with -NNUE
# appended on the NNUE branch (never twice).
release_name() {
    local version="$1" branch="$2"
    if [ "$branch" = "$NNUE_RELEASE_BRANCH" ]; then
        echo "${version%-NNUE}-NNUE"
    else
        echo "$version"
    fi
}
