#!/usr/bin/env bash
# Build flags shared by build_and_release.sh, scripts/rebuild_engines.sh and the measurement
# procedure (sourced, not executed).
#
# Every engine in one match or throughput run must be built with the same flags: on x86-64 the
# x86-64-v3 build is about 11% faster than the default x86-64 one (task.md 6.2), so a binary built
# one way against one built the other measures the compiler flag, not the engine. The default is
# what the release-binaries workflow ships: x86-64-v3 on x86-64, and nothing on ARM, where NEON is
# part of the baseline target.
#
# SUPRAH_TARGET_CPU overrides the level on any architecture (`native`, `x86-64`, ...); set it to the
# empty string to build without -C target-cpu at all, as releases up to v0.54.0 were built.

# Prints the target-cpu level for an architecture (`uname -m`), honouring SUPRAH_TARGET_CPU.
suprah_target_cpu() {
    local arch="${1:-$(uname -m)}"
    if [ -n "${SUPRAH_TARGET_CPU+set}" ]; then
        echo "$SUPRAH_TARGET_CPU"
    elif [ "$arch" = "x86_64" ]; then
        echo "x86-64-v3"
    else
        echo ""
    fi
}

# Prints RUSTFLAGS with -C target-cpu=<level> appended; RUSTFLAGS unchanged for an empty level.
suprah_rustflags() {
    local level="$1"
    if [ -z "$level" ]; then
        echo "${RUSTFLAGS:-}"
    elif [ -z "${RUSTFLAGS:-}" ]; then
        echo "-C target-cpu=$level"
    else
        echo "$RUSTFLAGS -C target-cpu=$level"
    fi
}

# Succeeds when a CPU with the given flags (default: this host's /proc/cpuinfo) can run a build
# for `level`. Only x86-64-v3 is checked - a binary for a level the CPU lacks dies with SIGILL in
# the middle of a match instead of failing here.
suprah_cpu_supports() {
    local level="$1" cpu_flags="${2:-}"
    [ "$level" = "x86-64-v3" ] || return 0
    if [ -z "$cpu_flags" ]; then
        cpu_flags="$(grep -m1 '^flags' /proc/cpuinfo 2>/dev/null)"
    fi
    local needed
    for needed in avx2 bmi2 fma movbe; do
        case " $cpu_flags " in
            *" $needed "*) ;;
            *) return 1 ;;
        esac
    done
    return 0
}

# Prints the Matt-Magie engine file name of a release tag, as build_and_release.sh deploys it:
# v0.53.0-NNUE -> suprah-0.53.0-nnue, v0.52.0 -> suprah-0.52.0.
suprah_engine_name() {
    local tag="${1#v}"
    echo "suprah-${tag,,}"
}

# Exports RUSTFLAGS for this host and prints the level, or fails when the CPU cannot run it.
suprah_apply_build_flags() {
    local level
    level="$(suprah_target_cpu)"
    if ! suprah_cpu_supports "$level"; then
        echo "Error: this CPU cannot run a target-cpu=$level build. Set SUPRAH_TARGET_CPU (e.g. to x86-64 or empty)." >&2
        return 1
    fi
    RUSTFLAGS="$(suprah_rustflags "$level")"
    export RUSTFLAGS
    echo "Build flags: target-cpu=${level:-<default>} (RUSTFLAGS='${RUSTFLAGS}')"
}
