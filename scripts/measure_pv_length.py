#!/usr/bin/env python3
"""Length of the reported principal variation per depth, for two option sets of one binary.

A Transposition Table cutoff on the principal variation ends the reported line after one move,
because an entry carries a move and no line (`task.md` 8.1). This script shows how long the line
of every completed iteration is, with and without a set of options - by default
`TtCutPvGuard=true` against the shipped configuration - and the wall time each side took.

Lengths are a property of the tree and hold under load; the wall times do not, and are only
indicative when other processes share the cores.

Usage:
    scripts/measure_pv_length.py <binary> [--depth 16] [--a-options ...] [--b-options TtCutPvGuard=true]
"""
import argparse
import os
import statistics
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from measure_stage0 import POSITIONS  # noqa: E402
from measure_throughput import OPTIONS, parse_options  # noqa: E402
import uci_driver  # noqa: E402


def line_lengths(result):
    """Maps every completed depth of a search to the number of moves its line reports."""
    lengths = {}
    for entry in result.info_signature:
        fields = dict(entry)
        if "depth" in fields:
            lengths[int(fields["depth"])] = len(fields.get("pv", "").split())
    return lengths


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("binary")
    parser.add_argument("--depth", type=int, default=16)
    parser.add_argument("--a-options", nargs="*", default=(),
                        help="setoption pairs of side A, as Name=value (default: none)")
    parser.add_argument("--b-options", nargs="*", default=("TtCutPvGuard=true",),
                        help="setoption pairs of side B, as Name=value")
    args = parser.parse_args()

    if not os.path.isfile(args.binary):
        print(f"not a file: {args.binary}")
        return 2

    sides = (("A", parse_options(args.a_options)), ("B", parse_options(args.b_options)))
    per_depth = {"A": {}, "B": {}}
    time_ms = {"A": 0, "B": 0}
    final = {"A": [], "B": []}

    for name, fen in POSITIONS:
        row = []
        for side, extra in sides:
            result = uci_driver.search(args.binary, fen, args.depth, options=OPTIONS + extra)
            lengths = line_lengths(result)
            for depth, length in lengths.items():
                per_depth[side].setdefault(depth, []).append(length)
            time_ms[side] += result.time_ms
            last = lengths.get(args.depth, 0)
            final[side].append(last)
            row.append(f"{side} {last:2d} moves {result.time_ms:6d} ms")
        print(f"{name:<14} " + " | ".join(row), flush=True)

    print()
    print("depth  mean A  mean B")
    for depth in sorted(set(per_depth["A"]) | set(per_depth["B"])):
        a = per_depth["A"].get(depth, [])
        b = per_depth["B"].get(depth, [])
        mean_a = f"{statistics.mean(a):6.1f}" if a else "     -"
        mean_b = f"{statistics.mean(b):6.1f}" if b else "     -"
        print(f"{depth:5d}  {mean_a}  {mean_b}")
    print()
    for side, extra in sides:
        options = " ".join(f"{n}={v}" for n, v in extra) or "(defaults)"
        print(f"{side}: {options}: line at depth {args.depth} mean {statistics.mean(final[side]):.1f}, "
              f"min {min(final[side])}, total {time_ms[side] / 1000:.1f} s")
    return 0


if __name__ == "__main__":
    sys.exit(main())
