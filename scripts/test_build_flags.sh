#!/usr/bin/env bash
# Tests for scripts/build_flags.sh.
# Run with `bash scripts/test_build_flags.sh`; exit status 0 when every case passes.

source "$(dirname "$0")/build_flags.sh"

failures=0

expect() {
    local what="$1" expected="$2" actual="$3"
    if [ "$actual" = "$expected" ]; then
        echo "ok:   $what -> '$actual'"
    else
        echo "FAIL: $what -> '$actual', expected '$expected'"
        failures=$((failures + 1))
    fi
}

# The default level per architecture: what CI ships on x86, nothing on ARM (NEON is baseline).
unset SUPRAH_TARGET_CPU
expect "target cpu on x86_64" "x86-64-v3" "$(suprah_target_cpu x86_64)"
expect "target cpu on aarch64" "" "$(suprah_target_cpu aarch64)"
# An explicit value wins on every architecture; an empty one switches the flag off.
expect "override on x86_64" "native" "$(SUPRAH_TARGET_CPU=native suprah_target_cpu x86_64)"
expect "override on aarch64" "neoverse-n1" "$(SUPRAH_TARGET_CPU=neoverse-n1 suprah_target_cpu aarch64)"
expect "empty override on x86_64" "" "$(SUPRAH_TARGET_CPU= suprah_target_cpu x86_64)"

# The rustc flags: appended to whatever RUSTFLAGS already holds, nothing when no level is set.
expect "rustflags for v3" "-C target-cpu=x86-64-v3" "$(RUSTFLAGS= suprah_rustflags x86-64-v3)"
expect "rustflags appended" "-C debuginfo=1 -C target-cpu=x86-64-v3" "$(RUSTFLAGS='-C debuginfo=1' suprah_rustflags x86-64-v3)"
expect "rustflags without a level" "" "$(RUSTFLAGS= suprah_rustflags '')"

# The Matt-Magie engine name of a release tag, as build_and_release.sh deploys it.
expect "NNUE tag" "suprah-0.53.0-nnue" "$(suprah_engine_name v0.53.0-NNUE)"
expect "HCE tag" "suprah-0.52.0" "$(suprah_engine_name v0.52.0)"
expect "tag without v" "suprah-0.52.0" "$(suprah_engine_name 0.52.0)"

# The CPU check: v3 needs AVX2, BMI2, FMA and MOVBE; other levels are not checked.
expect "v3 on a v3 cpu" "0" "$(suprah_cpu_supports x86-64-v3 'fpu avx avx2 bmi1 bmi2 fma movbe sse4_2'; echo $?)"
expect "v3 without avx2" "1" "$(suprah_cpu_supports x86-64-v3 'fpu avx bmi1 bmi2 fma movbe sse4_2'; echo $?)"
expect "no level" "0" "$(suprah_cpu_supports '' 'fpu'; echo $?)"

if [ "$failures" -ne 0 ]; then
    echo "$failures failure(s)"
    exit 1
fi
echo "all passed"
