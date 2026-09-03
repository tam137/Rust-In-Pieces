#!/usr/bin/env python3
"""The 8.1 cold-versus-warm gate, run against `task.md` 7.1 and 10.8 together.

Every benchmark in this repository sends `ucinewgame` before each position and therefore searches
with empty hash tables. A played game does not: the Transposition Table and the pawn hash table
accumulate across eighty moves. A defect that only shows once those tables are full is invisible
to every one of those benchmarks -- and to self-play matches, which share it on both sides. That
is exactly how fail-soft (8.1) survived four 1000-game runs while costing roughly two hundred Elo,
and how the cold-versus-warm measurement found it in minutes.

This script searches one fixed 60-position game twice with the same build:

  cold  `ucinewgame` before every position, so both tables start empty every time
  warm  no `ucinewgame` after the first, so the tables fill the way they do in a game

and reports how far the score for the same position moves between the two passes. The metrics are
8.1's, so the numbers land on the scale of the table already in that section: positions drifting
more than 50cp, mean drift, max drift. Its fail-hard reference is 0 of 60 above 50cp, mean 5.5,
max 31.

Two open items produce drift of this shape and the gate cannot tell them apart on its own, so it
runs a 2x2 rather than a single pass:

  A  as shipped                                   the total
  B  `EnableLazyEval false`                       10.8's channel closed, by option, no rebuild
  C  colour-blind `bound_for`                     7.1's channel closed, needs a second binary
  D  both                                         the floor that belongs to neither

A minus B is 10.8's contribution, A minus C is 7.1's, and anything left in D is a third source
that neither item names. B and D search different trees from A and C, so the four are compared on
their own cold-versus-warm self-consistency only, never against each other's scores.

Usage:

    python3 scripts/measure_cold_warm_drift.py --binary <path to the shipped build> \
        --blind-binary <path to the colour-blind build> --depth 9

    python3 scripts/measure_cold_warm_drift.py --binary <path> --self-check

`--self-check` runs two cold passes and requires them to agree position for position. Until that
passes, no warm number from this harness means anything.
"""

import argparse
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import uci_driver  # noqa: E402

PROJECT_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DEFAULT_BINARY = os.path.join(PROJECT_ROOT, "target", "release", "suprah")

# 8.1 counts a position as drifting when the two passes disagree by more than this. It is not a
# tunable: the reference table in that section is stated on it.
DRIFT_THRESHOLD_CP = 50

# What 8.1 measured for the build with no such defect, quoted here so the report can be read
# without opening `task.md`.
FAIL_HARD_REFERENCE = "0 / 60 above 50cp, mean 5.5, max 31"

# The first 60 plies of a v0.37.2-rc against v0.37.1 gauntlet game, in UCI notation. It is written
# out rather than read from a PGN because `<mm>` and its PGNs do not travel with the repository,
# and a gate whose corpus changes between hosts measures nothing. The arc matters: it starts from
# the initial position with 32 pieces and ends with 13, so the opening, the middlegame and the
# transition into an endgame are all searched. A sequence that never leaves the opening fills
# neither hash table, and 10.8 needs repeated pawn structures to show at all.
SEQUENCE = (
    "d2d4 g8f6 g1f3 e7e6 c2c4 d7d5 b1c3 f8e7 c1f4 e8g8 e2e3 b8d7 f1e2 d5c4 e2c4 a7a6 c4b3 b7b5 "
    "e1g1 c7c5 d4d5 e6d5 c3d5 f6d5 b3d5 a8a7 d1e2 d7f6 d5b3 c5c4 b3c2 f6d5 f4b8 a7b7 b8e5 e7d6 "
    "f1d1 d6e5 f3e5 f8e8 e2h5 g7g6 e5g6 d8f6 g6h4 c4c3 b2c3 d5c3 h5h7 g8f8 h4g6 f7g6 h7g6 f6g6 "
    "c2g6 c3d1 g6e8 d1e3 e8c6 b7c7"
).split()


def positions_of(sequence):
    """Every position the sequence passes through, as the move prefix that reaches it.

    The last move of the sequence is never played: it is the move the engine is asked to find at
    the final position.
    """
    return [tuple(sequence[:i]) for i in range(len(sequence))]


def parse_score(text):
    """Splits a UCI `score` field into its kind and value, or `None` if there is not one.

    A mate score keeps its kind. It is a distance in moves and averaging one into a centipawn
    drift would produce a number in no unit at all.
    """
    if not text:
        return None
    tokens = text.split()
    if len(tokens) != 2 or tokens[0].lower() not in ("cp", "mate"):
        return None
    try:
        return tokens[0].lower(), int(tokens[1])
    except ValueError:
        return None


def score_drift(cold, warm):
    """`(delta, mate_mismatch)` for one position's two passes.

    `delta` is the unsigned centipawn distance when both passes returned a centipawn score, and
    `None` otherwise. `mate_mismatch` marks the case the delta cannot express: the two passes
    disagree about whether the position is mating at all, or by how far. That is a worse
    disagreement than any centipawn figure, not a missing one.
    """
    left, right = parse_score(cold), parse_score(warm)
    if left is None or right is None:
        return None, False
    if left[0] == "cp" and right[0] == "cp":
        return abs(left[1] - right[1]), False
    if left == right:
        return 0, False
    return None, True


class Entry:
    """One position searched in both passes."""

    __slots__ = ("ply", "cold", "warm")

    def __init__(self, ply, cold, warm):
        self.ply = ply
        self.cold = cold
        self.warm = warm


class Summary:
    __slots__ = ("label", "positions", "comparable", "drifting", "mean", "maximum",
                 "worst_ply", "mate_mismatches")

    def __init__(self, label, positions, comparable, drifting, mean, maximum, worst_ply,
                 mate_mismatches):
        self.label = label
        self.positions = positions
        self.comparable = comparable
        self.drifting = drifting
        self.mean = mean
        self.maximum = maximum
        self.worst_ply = worst_ply
        self.mate_mismatches = mate_mismatches


def summarise(label, entries, threshold=DRIFT_THRESHOLD_CP):
    """8.1's three metrics over one build's two passes, plus the mate disagreements.

    A mate mismatch counts as a drifting position -- the two passes disagree about the same
    position -- but stays out of the mean and the maximum, which are centipawn figures.
    """
    deltas = []
    drifting = 0
    mismatches = 0
    maximum = 0
    worst_ply = None
    for entry in entries:
        delta, mismatch = score_drift(entry.cold, entry.warm)
        if mismatch:
            mismatches += 1
            drifting += 1
            continue
        if delta is None:
            continue
        deltas.append(delta)
        if delta > threshold:
            drifting += 1
        if delta > maximum:
            maximum = delta
            worst_ply = entry.ply
    mean = sum(deltas) / len(deltas) if deltas else 0.0
    return Summary(label, len(entries), len(deltas), drifting, mean, maximum, worst_ply,
                   mismatches)


def is_at_fail_hard_levels(summary):
    """Whether this build's tables are as inert as 8.1's fail-hard reference.

    The reference is 0 of 60 above 50cp. A mate mismatch disqualifies on its own: it is a larger
    disagreement than the threshold can express.
    """
    return summary.drifting == 0 and summary.mate_mismatches == 0


class Attribution:
    __slots__ = ("total", "lazy_share", "bound_share", "floor", "third_source")

    def __init__(self, total, lazy_share, bound_share, floor, third_source):
        self.total = total
        self.lazy_share = lazy_share
        self.bound_share = bound_share
        self.floor = floor
        self.third_source = third_source


def attribute(total, without_lazy, without_bound, floor):
    """Splits the drifting positions between 10.8, 7.1 and whatever is left.

    Each share is what closing that one channel removes. The two shares do not have to add up to
    the total and are not expected to: the defects can move the same position, and the floor is
    what survives both levers. Anything in the floor belongs to neither open item and is recorded
    as a new question rather than credited to one of them.
    """
    return Attribution(
        total=total.drifting,
        lazy_share=total.drifting - without_lazy.drifting,
        bound_share=total.drifting - without_bound.drifting,
        floor=floor.drifting,
        third_source=floor.drifting > 0,
    )


def run_pass(binary, options, depth, cold, progress=None):
    """Searches the whole sequence once and returns the score of every position.

    `cold` clears both hash tables before each position; the warm pass clears them once, at the
    start, and then lets them fill. Nothing else differs between the two.
    """
    scores = []
    with uci_driver.Session(binary, options=options) as session:
        session.new_game()
        for index, moves in enumerate(positions_of(SEQUENCE)):
            if cold and index > 0:
                session.new_game()
            result = session.search(moves, depth)
            scores.append(result.score)
            if progress:
                progress(index, result)
    return scores


def run_build(label, binary, options, depth, verbose):
    def progress(index, result):
        if verbose:
            sys.stderr.write(f"\r  {label}: position {index + 1}/{len(SEQUENCE)}   ")
            sys.stderr.flush()

    cold = run_pass(binary, options, depth, cold=True, progress=progress)
    warm = run_pass(binary, options, depth, cold=False, progress=progress)
    if verbose:
        sys.stderr.write("\r" + " " * 40 + "\r")
    return [Entry(ply, c, w) for ply, (c, w) in enumerate(zip(cold, warm))]


def format_summaries(summaries):
    lines = [
        f"{'Run':<26} {'positions':>9} {'>50cp':>7} {'mean':>7} {'max':>6} {'worst ply':>10} {'mate':>5}",
        "-" * 76,
    ]
    for summary in summaries:
        worst = "-" if summary.worst_ply is None else str(summary.worst_ply)
        lines.append(
            f"{summary.label:<26} {summary.comparable:>9} {summary.drifting:>7} "
            f"{summary.mean:>7.1f} {summary.maximum:>6} {worst:>10} {summary.mate_mismatches:>5}"
        )
    lines.append("")
    lines.append(f"8.1 fail-hard reference:   {FAIL_HARD_REFERENCE}")
    return "\n".join(lines)


def format_worst_positions(label, entries, limit=10):
    ranked = []
    for entry in entries:
        delta, mismatch = score_drift(entry.cold, entry.warm)
        if mismatch:
            ranked.append((10 ** 6, entry, "mate disagreement"))
        elif delta is not None and delta > 0:
            ranked.append((delta, entry, f"{delta}"))
    ranked.sort(key=lambda item: -item[0])
    if not ranked:
        return f"{label}: no position moved at all."
    lines = [f"{label}: the positions that moved most", "", f"{'ply':>4} {'cold':>12} {'warm':>12} {'drift':>18}"]
    for _, entry, shown in ranked[:limit]:
        lines.append(f"{entry.ply:>4} {entry.cold:>12} {entry.warm:>12} {shown:>18}")
    return "\n".join(lines)


def self_check(binary, depth, verbose):
    """Two cold passes must agree position for position, or nothing else here is measurable."""
    first = run_pass(binary, (), depth, cold=True)
    second = run_pass(binary, (), depth, cold=True)
    disagreements = [(ply, a, b) for ply, (a, b) in enumerate(zip(first, second)) if a != b]
    if disagreements:
        print(f"FAILED: {len(disagreements)} of {len(first)} positions disagree between two "
              f"identical cold passes. The harness is not deterministic and no warm number from "
              f"it can be believed.")
        for ply, a, b in disagreements[:10]:
            print(f"  ply {ply:>3}: {a!r} then {b!r}")
        return 1
    print(f"OK: two cold passes agree on all {len(first)} positions at depth {depth}.")
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--binary", default=DEFAULT_BINARY,
                        help="the shipped build, as released (default: the release build in this tree)")
    parser.add_argument("--blind-binary", default=None,
                        help="a build with `SearchService::bound_for` made colour-blind. Without "
                             "it only runs A and B are possible and 7.1 cannot be attributed.")
    parser.add_argument("--depth", type=int, default=9,
                        help="fixed search depth, so the two passes differ in table state and in "
                             "nothing else (default: 9, the depth of the 7.1 store census)")
    parser.add_argument("--self-check", action="store_true",
                        help="run two cold passes and require them to agree, then stop")
    parser.add_argument("--quiet", action="store_true", help="no per-position progress")
    args = parser.parse_args()

    verbose = not args.quiet
    if args.self_check:
        return self_check(args.binary, args.depth, verbose)

    runs = [
        ("A  as shipped", args.binary, ()),
        ("B  EnableLazyEval false", args.binary, (("EnableLazyEval", "false"),)),
    ]
    if args.blind_binary:
        runs += [
            ("C  colour-blind bound_for", args.blind_binary, ()),
            ("D  both", args.blind_binary, (("EnableLazyEval", "false"),)),
        ]

    summaries = []
    collected = {}
    for label, binary, options in runs:
        entries = run_build(label, binary, options, args.depth, verbose)
        collected[label[0]] = entries
        summaries.append(summarise(label, entries))

    print(f"Cold versus warm over {len(SEQUENCE)} positions at fixed depth {args.depth}")
    print()
    print(format_summaries(summaries))
    print()
    print(format_worst_positions(summaries[0].label, collected["A"]))
    print()

    total = summaries[0]
    if is_at_fail_hard_levels(total):
        print("Verdict: run A is at 8.1's fail-hard levels. Neither 7.1 nor 10.8 moves a played")
        print("search measurably, so neither is priced and neither gates the next release.")
        return 0

    print("Verdict: run A drifts. The attribution below says which channel carries it.")
    if not args.blind_binary:
        print("Only runs A and B were possible: pass --blind-binary to attribute 7.1 as well.")
        return 0

    found = attribute(total, summaries[1], summaries[2], summaries[3])
    print()
    print(f"  drifting positions, as shipped          {found.total}")
    print(f"  removed by closing 10.8's channel       {found.lazy_share}")
    print(f"  removed by closing 7.1's channel        {found.bound_share}")
    print(f"  surviving both levers                   {found.floor}")
    if found.third_source:
        print()
        print("  A third source of drift survives both levers. It belongs to neither open item")
        print("  and is to be recorded as a new question, not credited to 7.1 or 10.8.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
