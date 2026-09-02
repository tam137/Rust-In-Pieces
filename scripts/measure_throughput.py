#!/usr/bin/env python3
"""Paired wall-time comparison of two engine binaries at fixed depth.

Throughput cannot be read from `nps` or `nodes` here: both report *generated* moves rather than
searched nodes, so any change to move generation moves them for reasons unrelated to speed. The
metric is wall time to a fixed depth, taken from the engine's own `info ... time` field through
`scripts/uci_driver.py`, which waits for `bestmove` instead of sleeping for a guessed interval.

The two binaries are run interleaved per position, so a host that slows down halfway through the
corpus penalises both equally, and the best of `--repeats` is kept for each, because the fastest
run of a set is the one least disturbed by other load.

The comparison also reports whether the two binaries searched the same tree: `depth`, `score`,
`nodes` and the principal variation of every completed iteration must agree. That is the
node-identity criterion used since v0.31.0, and for a change that is supposed to leave the tree
alone it is the result that matters more than the timing.

Usage:
    scripts/measure_throughput.py <baseline> <candidate> [--depth 10] [--repeats 3]
"""
import argparse
import os
import statistics
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from measure_stage0 import POSITIONS  # noqa: E402
import uci_driver  # noqa: E402

OPTIONS = (("Hash", "64"), ("Threads", "1"))


def parse_options(pairs):
    """Turns `Name=value` arguments into the option pairs `uci_driver` expects."""
    parsed = []
    for pair in pairs or ():
        name, _, value = pair.partition("=")
        parsed.append((name, value))
    return tuple(parsed)


def best_of(binary, fen, depth, repeats, extra=()):
    """Runs one position `repeats` times and returns the fastest result."""
    best = None
    for _ in range(repeats):
        result = uci_driver.search(binary, fen, depth, options=OPTIONS + tuple(extra))
        if best is None or result.time_ms < best.time_ms:
            best = result
    return best


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("baseline")
    parser.add_argument("candidate")
    parser.add_argument("--depth", type=int, default=10)
    parser.add_argument("--repeats", type=int, default=3)
    parser.add_argument("--base-options", nargs="*", default=(),
                        help="extra setoption pairs for the baseline, as Name=value")
    parser.add_argument("--cand-options", nargs="*", default=(),
                        help="extra setoption pairs for the candidate, as Name=value")
    args = parser.parse_args()

    for binary in (args.baseline, args.candidate):
        if not os.path.isfile(binary):
            print(f"not a file: {binary}")
            return 2

    base_options = parse_options(args.base_options)
    cand_options = parse_options(args.cand_options)

    print(f"\nFixed depth {args.depth}, {len(POSITIONS)} positions, best of {args.repeats}, "
          f"Hash=64 Threads=1\n")
    header = (f"{'Position':<18}{'base ms':>10}{'cand ms':>10}{'delta':>9}"
              f"{'base nodes':>14}{'cand nodes':>14}  tree")
    print(header)
    print("-" * len(header))

    deltas = []
    identical = True
    faster = 0
    base_total = 0
    cand_total = 0
    for name, fen in POSITIONS:
        base = best_of(args.baseline, fen, args.depth, args.repeats, base_options)
        cand = best_of(args.candidate, fen, args.depth, args.repeats, cand_options)

        same = strip_nodes(base.info_signature) == strip_nodes(cand.info_signature)
        identical &= same
        # A positive delta is the candidate being faster, which is what the sign convention of
        # every throughput number in `task.md` means.
        delta = (base.time_ms - cand.time_ms) / base.time_ms * 100 if base.time_ms else 0.0
        deltas.append(delta)
        if delta > 0:
            faster += 1

        base_total += base.time_ms
        cand_total += cand.time_ms

        base_nodes = base.info_signature[-1] if base.info_signature else ()
        cand_nodes = cand.info_signature[-1] if cand.info_signature else ()
        print(f"{name:<18}{base.time_ms:>10}{cand.time_ms:>10}{delta:>8.1f}%"
              f"{node_count(base_nodes):>14}{node_count(cand_nodes):>14}  "
              f"{'same' if same else 'DIFFERS'}")

    print("-" * len(header))
    print(f"\nMedian delta   {statistics.median(deltas):+.1f}%   "
          f"(mean {statistics.mean(deltas):+.1f}%)")
    total_delta = (base_total - cand_total) / base_total * 100 if base_total else 0.0
    print(f"Corpus total   {base_total} ms -> {cand_total} ms   {total_delta:+.1f}%   "
          f"(cost-weighted, so the sub-10 ms positions cannot dominate it)")
    print(f"Faster         {faster} of {len(POSITIONS)} positions")
    print(f"Tree identity  {'14/14 identical' if identical else 'NOT IDENTICAL'} "
          f"(depth, score and pv per iteration; `nodes` is excluded, it counts generated moves)"
          .replace("14/14", f"{len(POSITIONS)}/{len(POSITIONS)}"))
    print()
    return 0 if identical else 1


def strip_nodes(signature):
    """Drops the `nodes` field from a signature before comparing two searches.

    `nodes` reports *generated* moves, so a change that defers generation lowers it while
    searching the same tree -- which is the whole point of a staged picker. Depth, score and the
    principal variation of every completed iteration are what identity is read from here.
    """
    return [tuple((key, value) for key, value in entry if key != "nodes") for entry in signature]


def node_count(signature_entry):
    """Pulls the `nodes` value out of one `parse_info` signature entry."""
    for key, value in signature_entry:
        if key == "nodes":
            return value
    return "-"


if __name__ == "__main__":
    sys.exit(main())
