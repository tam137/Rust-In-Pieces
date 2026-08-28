#!/usr/bin/env python3
"""Prices razoring at depth 1 (`task.md` section 3) on a fixed-depth corpus.

Razoring changes the search tree, so it cannot be verified by node identity the way the
throughput wins of Milestone 1 were. What can be measured deterministically, before spending a
single game on it, is what it costs and what it buys:

  * **tree size** -- interior nodes searched to a fixed depth. Fewer nodes at the same depth is
    what a pruning rule is for.
  * **wall time** to that same depth. `task.md` rule 4: the UCI `nodes` field reports *generated*
    moves and is useless here, so time is measured directly.
  * **agreement** -- whether the razored search still returns the same move and score. A rule that
    saves time by playing something else has not been priced by a node count.

The engine is driven over UCI with `EnableRazoring`, so both sides of the comparison are the same
binary and no version-suffixed build is needed.

Usage:
    scripts/measure_razoring.py [--depth 10] [--margin 300] [--binary PATH]
"""
import argparse
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from measure_stage0 import POSITIONS, PROJECT_ROOT  # noqa: E402
from uci_driver import search  # noqa: E402

TREE = re.compile(r"SEARCHTREE calculated=(\d+) eval=(\d+)")


def probe(binary, fen, depth, options):
    """One completed fixed-depth search: (searched nodes, engine-reported ms, depth, score, move).

    `dump_tree` runs once per iteration of the iterative deepening and `Stats` is fresh on each
    of those calls, so the cost of the whole `go depth N` is the sum of the per-iteration counts,
    not the last one. The time is the engine's own cumulative `info ... time`, which excludes
    process start-up.
    """
    result = search(binary, fen, depth, options, cwd=PROJECT_ROOT)
    nodes = sum(int(m[0]) for m in TREE.findall(result.stderr))
    return nodes, result.time_ms, result.depth, result.score, result.best_move


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("-d", "--depth", type=int, default=10)
    parser.add_argument("-m", "--margin", type=int, default=300)
    parser.add_argument("-r", "--repeats", type=int, default=3,
                        help="timing samples per position; the fastest is reported")
    parser.add_argument("--binary", default=os.path.join(PROJECT_ROOT, "target", "release", "suprah"),
                        help="a search-diag build, so SEARCHTREE carries the searched tree size")
    args = parser.parse_args()

    off = [("EnableRazoring", "false")]
    on = [("EnableRazoring", "true"), ("RazoringMargin", str(args.margin))]

    print(f"\nFixed depth {args.depth}, margin {args.margin}, {len(POSITIONS)} positions, "
          f"best of {args.repeats}\n")
    header = (f"{'Position':<18}{'nodes off':>11}{'nodes on':>11}{'d nodes':>9}"
              f"{'ms off':>9}{'ms on':>9}{'d time':>9}  agree")
    print(header)
    print("-" * len(header))

    sum_off = sum_on = 0
    time_off = time_on = 0.0
    disagreements = []

    for name, fen in POSITIONS:
        # The node count is deterministic, so only the timing needs repeating; the fastest sample
        # is the one least contaminated by whatever else the machine was doing.
        a = min((probe(args.binary, fen, args.depth, off) for _ in range(args.repeats)),
                key=lambda r: r[1])
        b = min((probe(args.binary, fen, args.depth, on) for _ in range(args.repeats)),
                key=lambda r: r[1])

        sum_off += a[0]
        sum_on += b[0]
        time_off += a[1]
        time_on += b[1]
        agree = (a[3], a[4]) == (b[3], b[4])
        if not agree:
            disagreements.append((name, a[3], a[4], b[3], b[4]))

        d_nodes = (b[0] - a[0]) * 100.0 / a[0] if a[0] else 0.0
        d_time = (b[1] - a[1]) * 100.0 / a[1] if a[1] else 0.0
        print(f"{name:<18}{a[0]:>11}{b[0]:>11}{d_nodes:>8.1f}%{a[1]:>9.0f}{b[1]:>9.0f}"
              f"{d_time:>8.1f}%  {'yes' if agree else 'NO'}")

    print("-" * len(header))
    d_nodes = (sum_on - sum_off) * 100.0 / sum_off if sum_off else 0.0
    d_time = (time_on - time_off) * 100.0 / time_off if time_off else 0.0
    print(f"{'TOTAL':<18}{sum_off:>11}{sum_on:>11}{d_nodes:>8.1f}%{time_off:>9.0f}"
          f"{time_on:>9.0f}{d_time:>8.1f}%\n")

    print(f"  tree size   {d_nodes:+.1f}%   (negative is the rule working)")
    print(f"  wall time   {d_time:+.1f}%   (this is the number that turns into depth in a game)")
    if disagreements:
        print(f"\n  {len(disagreements)} of {len(POSITIONS)} positions returned a different "
              f"move or score:")
        for name, sa, ma, sb, mb in disagreements:
            print(f"    {name:<18} off: {sa:>12} {ma:<6}  on: {sb:>12} {mb}")
        print("  A different move is not by itself a defect - it is the reason the rule needs a "
              "match.")
    else:
        print("\n  every position returned the same move and score")
    return 0


if __name__ == "__main__":
    sys.exit(main())
