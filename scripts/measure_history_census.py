#!/usr/bin/env python3
"""Censuses what the Late Move Reduction reads out of the history table, `task.md` 23.3.

The two thresholds `lmr_history_good_threshold` and `lmr_history_bad_threshold` were calibrated
against an unsigned table that saturated at zero and was rescaled whenever an entry passed 9000.
23.3 replaces that with a signed table bounded by `model::MAX_HISTORY` and updated by gravity, so
both thresholds now sit on a scale that no longer exists. This script is what replaces guessing at
the new ones: it reports how often each branch of the rule fires and where the population that
reaches the decision actually sits.

The engine must be built with the `search-diag` feature, which is what emits the `SEARCHDIAGHIST`
line. The counters are process-global and cumulative, so one `Session` over the whole pool and the
last line it printed carries the totals.

The positions are the ones `scripts/measure_tree_size.py` uses -- `task.md` says the 14-position
corpus cannot rank an ordering change, and a distribution is the same kind of question.

Usage:
    scripts/measure_history_census.py <search-diag binary> [--positions 300] [--depth 10]
"""
import argparse
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import uci_driver  # noqa: E402
from measure_tree_size import DEFAULT_OPENINGS, OPTIONS, load_positions  # noqa: E402

HIST = re.compile(
    r"SEARCHDIAGHIST decisions=(\d+) good=(\d+) bad=(\d+) zero=(\d+) negative=(\d+) buckets=(\S*)")


def census(binary, positions, depth):
    """Runs the pool through one process and returns the cumulative counters of the last dump."""
    with uci_driver.Session(binary, options=OPTIONS) as session:
        for index, moves in enumerate(positions, start=1):
            session.new_game()
            session.search(moves, depth)
            if index % 25 == 0:
                print(f"  {index}/{len(positions)}", flush=True)
    matches = HIST.findall(session.stderr)
    if not matches:
        raise SystemExit(f"no SEARCHDIAGHIST line from {binary} -- is it a search-diag build?")
    decisions, good, bad, zero, negative, buckets = matches[-1]
    parsed = {}
    for item in buckets.split(",") if buckets else ():
        label, _, count = item.partition(":")
        parsed[label] = int(count)
    return dict(decisions=int(decisions), good=int(good), bad=int(bad),
                zero=int(zero), negative=int(negative), buckets=parsed)


def bucket_order(label):
    """Sorts the histogram from the most negative slot to the most positive one."""
    if label == "0":
        return 0.0
    sign, _, power = label.partition("2^")
    return (1 if sign == "+" else -1) * (int(power) + 1)


def report(name, data):
    total = data["decisions"] or 1
    print(f"\n{name}")
    print(f"  decisions {data['decisions']:>12,}")
    for key in ("good", "bad", "zero", "negative"):
        print(f"  {key:<9} {data[key]:>12,}   {100.0 * data[key] / total:6.2f}%")

    print("\n  distribution of the entry the rule read")
    running = 0
    for label in sorted(data["buckets"], key=bucket_order):
        count = data["buckets"][label]
        running += count
        edge = "" if label == "0" else f"  [{2 ** int(label.split('2^')[1]):>6}..)"
        print(f"    {label:>7}{edge:<12} {count:>12,}   {100.0 * count / total:6.2f}%   "
              f"cumulative {100.0 * running / total:6.2f}%")


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("binary")
    parser.add_argument("--openings", default=DEFAULT_OPENINGS)
    parser.add_argument("--positions", type=int, default=300)
    parser.add_argument("--depth", type=int, default=10)
    parser.add_argument("--label", default=None)
    args = parser.parse_args()

    positions = load_positions(args.openings, args.positions)
    if not positions:
        raise SystemExit(f"no positions in {args.openings}")

    print(f"\n{len(positions)} positions from {os.path.basename(args.openings)}, "
          f"fixed depth {args.depth}, Hash=64 Threads=1")
    data = census(args.binary, positions, args.depth)
    report(args.label or os.path.basename(args.binary), data)
    return 0


if __name__ == "__main__":
    sys.exit(main())
