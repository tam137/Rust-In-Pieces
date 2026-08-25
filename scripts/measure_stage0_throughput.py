#!/usr/bin/env python3
"""Throughput of the Stage-0 short-circuit (`task.md` 1.2.2).

Measures wall time to complete a fixed depth over the position corpus, which is the only
metric-independent way to price this change: Stage 0 skips move generation, and the UCI `nodes`
field reports *generated* moves, so `nodes` and `nps` both fall while the engine gets faster.

Three configurations on one binary:
  off        EnableTtMoveFirst=false          -- the v0.32.0 search
  snapshot   +Stage0HistorySnapshot=true      -- short-circuit, tree bit-identical to `off`
  live       +Stage0HistorySnapshot=false     -- short-circuit, ranks against the live history

Usage:
    scripts/measure_stage0_throughput.py [--depth 10] [--repeats 3]
"""
import argparse
import os
import subprocess
import sys

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.dirname(SCRIPT_DIR)
BINARY = os.path.join(PROJECT_ROOT, "target", "release", "suprah")

sys.path.insert(0, SCRIPT_DIR)
from measure_stage0 import POSITIONS  # noqa: E402

CONFIGS = [
    ("off", "false", "true"),
    ("snapshot", "true", "true"),
    ("live", "true", "false"),
]


def run(fen, depth, first, snapshot, settle):
    cmd = (
        'echo "uci"; echo "setoption name OwnBook value false"; '
        f'echo "setoption name EnableTtMoveFirst value {first}"; '
        f'echo "setoption name Stage0HistorySnapshot value {snapshot}"; '
        f'echo "isready"; echo "position fen {fen}"; echo "go depth {depth}"; '
        f'sleep {settle}; echo "quit"'
    )
    res = subprocess.run(f"({cmd}) | {BINARY}", shell=True, capture_output=True,
                         text=True, timeout=300, cwd=PROJECT_ROOT)
    last = None
    for line in res.stdout.splitlines():
        if line.startswith("info depth"):
            tokens = line.split()
            if "time" in tokens:
                last = int(tokens[tokens.index("time") + 1])
    return last or 0


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("-d", "--depth", type=int, default=10)
    parser.add_argument("-r", "--repeats", type=int, default=3)
    parser.add_argument("-s", "--settle", type=float, default=3.0)
    args = parser.parse_args()

    if not os.path.exists(BINARY):
        sys.exit("build the release binary first: cargo build --release")

    # Best of N per position and configuration: the minimum is the least noisy estimator here,
    # because scheduling noise can only ever add time.
    totals = {}
    print(f"\nFixed depth {args.depth}, {len(POSITIONS)} positions, best of {args.repeats}\n")
    header = f"{'Position':<18}" + "".join(f"{name:>12}" for name, _, _ in CONFIGS)
    print(header)
    print("-" * len(header))

    for name, fen in POSITIONS:
        row = {}
        for cfg_name, first, snapshot in CONFIGS:
            best = min(run(fen, args.depth, first, snapshot, args.settle)
                       for _ in range(args.repeats))
            row[cfg_name] = best
            totals[cfg_name] = totals.get(cfg_name, 0) + best
        print(f"{name:<18}" + "".join(f"{row[c]:>12}" for c, _, _ in CONFIGS))

    print("-" * len(header))
    print(f"{'TOTAL ms':<18}" + "".join(f"{totals[c]:>12}" for c, _, _ in CONFIGS))

    baseline = totals["off"]
    print()
    for cfg_name, _, _ in CONFIGS:
        if cfg_name == "off":
            continue
        speedup = baseline / totals[cfg_name] if totals[cfg_name] else 0.0
        print(f"  {cfg_name:<10} {speedup:6.3f}x  ({(speedup - 1) * 100:+.1f}%)")


if __name__ == "__main__":
    sys.exit(main())
