#!/usr/bin/env python3
"""Compares two binaries over a large position sample, for changes that move the search tree.

`scripts/measure_throughput.py` uses a 14-position corpus, which is the right size for a
node-identity check: identity is binary, and fourteen positions that all agree is a strong
statement. It is the wrong size for a *move-ordering* change. Re-permuting a tie class re-shapes
the tree chaotically -- individual positions move by factors in both directions -- so fourteen
positions cannot tell an ordering that is worse from one that merely reshuffled.

This script answers the aggregate question instead: over a few hundred positions drawn from the
same opening pool the games are played on, does the candidate need more or less work to reach the
same depth? Positions come from an openings file, played out as `position startpos moves ...`,
with the hash tables cleared between them so the positions stay independent. The two engines are
driven alternately, position by position, so a host that slows down affects both equally.

Usage:
    scripts/measure_tree_size.py <baseline> <candidate> [--positions 300] [--depth 10]
"""
import argparse
import os
import statistics
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import uci_driver  # noqa: E402

OPTIONS = (("Hash", "64"), ("Threads", "1"))
DEFAULT_OPENINGS = os.path.join(
    os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "openings", "book_width.txt")


def load_positions(path, count):
    positions = []
    with open(path) as handle:
        for line in handle:
            moves = line.split()
            if moves:
                positions.append(moves)
            if len(positions) >= count:
                break
    return positions


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("baseline")
    parser.add_argument("candidate")
    parser.add_argument("--openings", default=DEFAULT_OPENINGS)
    parser.add_argument("--positions", type=int, default=300)
    parser.add_argument("--depth", type=int, default=10)
    args = parser.parse_args()

    positions = load_positions(args.openings, args.positions)
    if not positions:
        print(f"no positions in {args.openings}")
        return 2

    print(f"\n{len(positions)} positions from {os.path.basename(args.openings)}, "
          f"fixed depth {args.depth}, Hash=64 Threads=1\n")

    base_times, cand_times, base_nodes, cand_nodes = [], [], [], []
    identical = 0
    with uci_driver.Session(args.baseline, options=OPTIONS) as base, \
            uci_driver.Session(args.candidate, options=OPTIONS) as cand:
        for index, moves in enumerate(positions, start=1):
            # Alternate which engine searches first. Running the baseline first every time would
            # hand the candidate a warmed cache and a settled host on every single position, and
            # a systematic few percent is exactly the size of effect this script is asked about.
            if index % 2:
                base.new_game()
                base_result = base.search(moves, args.depth)
                cand.new_game()
                cand_result = cand.search(moves, args.depth)
            else:
                cand.new_game()
                cand_result = cand.search(moves, args.depth)
                base.new_game()
                base_result = base.search(moves, args.depth)

            if base_result.info_signature == cand_result.info_signature:
                identical += 1
            base_times.append(base_result.time_ms)
            cand_times.append(cand_result.time_ms)
            base_nodes.append(node_count(base_result))
            cand_nodes.append(node_count(cand_result))

            if index % 25 == 0:
                print(f"  {index}/{len(positions)}   "
                      f"time {sum(base_times)} ms -> {sum(cand_times)} ms   "
                      f"nodes {sum(base_nodes)} -> {sum(cand_nodes)}")

    ratios = [b / c if c else 1.0 for b, c in zip(base_times, cand_times) if b and c]
    print(f"\nTime      {sum(base_times)} ms -> {sum(cand_times)} ms   "
          f"{pct(sum(base_times), sum(cand_times)):+.1f}%")
    print(f"Nodes     {sum(base_nodes)} -> {sum(cand_nodes)}   "
          f"{pct(sum(base_nodes), sum(cand_nodes)):+.1f}%   (generated moves, not searched nodes)")
    print(f"Median    per-position time ratio {statistics.median(ratios):.3f} "
          f"(above 1.000 means the candidate is faster)")
    faster = sum(1 for b, c in zip(base_times, cand_times) if c < b)
    print(f"Faster    {faster} of {len(positions)} positions")
    print(f"Identical {identical} of {len(positions)} trees\n")
    return 0


def pct(base, cand):
    return (base - cand) / base * 100 if base else 0.0


def node_count(result):
    for key, value in (result.info_signature[-1] if result.info_signature else ()):
        if key == "nodes":
            return int(value)
    return 0


if __name__ == "__main__":
    sys.exit(main())
