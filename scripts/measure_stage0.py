#!/usr/bin/env python3
"""Stage-0 opportunity measurement for `task.md` specification 1.2.2.

A staged `MovePicker` is only worth building stage by stage if cutoffs are actually waiting
behind each stage. This script builds the current working tree twice -- once with the
`search-diag` Cargo feature and once without -- runs a fixed-depth search over a position corpus
on both, and reports:

  * how often the first searched move produced a beta cutoff, broken down by the picker stage
    that would have had to generate that move, and
  * whether the instrumented build searched a node-identical tree, which is what makes the
    measurement trustworthy: the counters must not perturb what they count.

Usage:
    scripts/measure_stage0.py [--depth 10] [--skip-build]
"""
import argparse
import os
import re
import shutil
import subprocess
import sys

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.dirname(SCRIPT_DIR)
BUILD_DIR = os.path.join(PROJECT_ROOT, "target", "stage0-measurement")

POSITIONS = [
    ("Startpos",        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"),
    ("Kiwipete",        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1"),
    ("Middlegame",      "r1bqkb1r/pp3ppp/2n1pn2/2pp4/2PP4/2N1PN2/PP3PPP/R1BQKB1R w KQkq - 0 6"),
    ("Sharp Tactical",  "r1b1kb1r/pppp1ppp/8/4q3/3n4/8/PPPPBPPP/RNBQK2R w KQkq - 0 1"),
    ("Rook Endgame",    "8/5pk1/7p/8/8/3R2P1/r4PKP/8 w - - 0 1"),
    ("Pawn Endgame",    "8/8/5k2/p4p2/P4P2/5K2/8/8 w - - 0 1"),
    ("Closed Centre",   "r1bq1rk1/pp2ppbp/2np1np1/8/2PNP3/2N1B3/PP2BPPP/R2Q1RK1 w - - 0 1"),
    ("Open Sicilian",   "r1bqkb1r/pp2pppp/2np1n2/8/3NP3/2N5/PPP2PPP/R1BQKB1R w KQkq - 0 6"),
    ("Queenless MG",    "r3k2r/ppp2ppp/2n1bn2/3p4/3P4/2N1BN2/PPP2PPP/R3K2R w KQkq - 0 1"),
    ("Bishop Pair EG",  "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1"),
    ("Sharp French",    "r2qkb1r/1b1n1ppp/p2ppn2/1p6/3NPP2/2N1B3/PPPQ2PP/R3KB1R w KQkq - 0 9"),
    ("Promotion Race",  "8/P1p5/1p1p4/3P4/1P1p4/8/2P4k/K7 w - - 0 1"),
    ("Tactical Pin",    "rnbq1rk1/pp2bppp/4pn2/2pp2B1/2PP4/2N1PN2/PP3PPP/R2QKB1R w KQ - 0 8"),
    ("King Attack",     "r1bq1rk1/pp1nbppp/2p1pn2/3p4/2PP4/2NBPN2/PP3PPP/R1BQ1RK1 w - - 0 8"),
]

# Depth, score, node count and principal variation of every completed iteration. Two builds
# agreeing on all four across the corpus is the node-identity criterion used since v0.31.0.
INFO_KEYS = ("depth", "score", "nodes", "pv")

DIAG = re.compile(
    r"SEARCHDIAG interior=(\d+) available=(\d+) \S+ first_cut=(\d+) \S+ "
    r"stage0_cut=(\d+) \S+ wasted_validation=(\d+) \S+ tt_present=(\d+) \S+ tt_unranked=(\d+)"
)
CLASS = re.compile(
    r"SEARCHDIAGCLASS pv_tt=(\d+) capture=(\d+) quiet_check=(\d+) killer_counter=(\d+) quiet=(\d+)"
)

STAGE_LABELS = [
    "Stage 0  PV/TT move",
    "Stage 1  capture",
    "Stage 1b quiet giving check",
    "Stage 2  killer / counter",
    "Stage 3  ordinary quiet",
]


def build(feature=None):
    """Builds the working tree and returns the path of a private copy of the binary."""
    cmd = ["cargo", "build", "--release"]
    name = "suprah-base"
    if feature:
        cmd += ["--features", feature]
        name = "suprah-diag"
    subprocess.run(cmd, cwd=PROJECT_ROOT, check=True,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    os.makedirs(BUILD_DIR, exist_ok=True)
    target = os.path.join(BUILD_DIR, name)
    shutil.copy2(os.path.join(PROJECT_ROOT, "target", "release", "suprah"), target)
    return target


def run(binary, fen, depth, settle_sec, timeout=300):
    cmd = (
        'echo "uci"; echo "setoption name OwnBook value false"; echo "isready"; '
        f'echo "position fen {fen}"; echo "go depth {depth}"; '
        f'sleep {settle_sec}; echo "quit"'
    )
    res = subprocess.run(f"({cmd}) | {binary}", shell=True, capture_output=True,
                         text=True, timeout=timeout, cwd=PROJECT_ROOT)
    return res.stdout, res.stderr


def info_signature(stdout):
    signature = []
    for line in stdout.splitlines():
        if not line.startswith("info depth"):
            continue
        tokens = line.split()
        entry = {}
        for key in INFO_KEYS:
            if key in tokens:
                idx = tokens.index(key)
                entry[key] = " ".join(tokens[idx + 1:]) if key == "pv" else tokens[idx + 1]
        signature.append(tuple(sorted(entry.items())))
    return signature


def last_match(pattern, stderr, width):
    hits = pattern.findall(stderr)
    return tuple(int(x) for x in hits[-1]) if hits else (0,) * width


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("-d", "--depth", type=int, default=10, help="fixed search depth")
    parser.add_argument("-s", "--settle", type=float, default=3.0,
                        help="seconds to let a search finish before sending quit")
    parser.add_argument("--skip-build", action="store_true",
                        help="reuse the binaries from a previous run")
    args = parser.parse_args()

    if args.skip_build:
        base = os.path.join(BUILD_DIR, "suprah-base")
        diag = os.path.join(BUILD_DIR, "suprah-diag")
        if not (os.path.exists(base) and os.path.exists(diag)):
            sys.exit("no previously built binaries in target/stage0-measurement")
    else:
        print("building default and search-diag binaries ...", file=sys.stderr)
        base, diag = build(), build("search-diag")

    totals = [0] * 7
    classes = [0] * 5
    identical = True

    print(f"\nFixed depth {args.depth}, {len(POSITIONS)} positions\n")
    header = (f"{'Position':<18}{'interior':>12}{'available':>12}{'first_cut':>12}"
              f"{'stage0_cut':>12}{'share':>9}  tree")
    print(header)
    print("-" * len(header))

    for name, fen in POSITIONS:
        out_base, _ = run(base, fen, args.depth, args.settle)
        out_diag, err_diag = run(diag, fen, args.depth, args.settle)
        same = info_signature(out_base) == info_signature(out_diag)
        identical &= same

        counts = last_match(DIAG, err_diag, 7)
        totals = [t + v for t, v in zip(totals, counts)]
        classes = [t + v for t, v in zip(classes, last_match(CLASS, err_diag, 5))]

        share = counts[3] * 100.0 / counts[0] if counts[0] else 0.0
        print(f"{name:<18}{counts[0]:>12}{counts[1]:>12}{counts[2]:>12}{counts[3]:>12}"
              f"{share:>8.1f}%  {'identical' if same else 'DIFFERS'}")

    interior, available, first_cut, stage0_cut, wasted, tt_present, tt_unranked = totals
    pct = lambda value: value * 100.0 / interior if interior else 0.0

    print("-" * len(header))
    print(f"{'TOTAL':<18}{interior:>12}{available:>12}{first_cut:>12}{stage0_cut:>12}"
          f"{pct(stage0_cut):>8.1f}%  {'identical' if identical else 'DIFFERS'}\n")
    print(f"  PV/TT move available at        {pct(available):5.1f}% of interior nodes")
    print(f"  cutoff on the first move at    {pct(first_cut):5.1f}% of interior nodes")
    print(f"  cutoff on the PV/TT move at    {pct(stage0_cut):5.1f}% of interior nodes  <-- saving")
    print(f"  validation without saving at   {pct(wasted):5.1f}% of interior nodes  <-- cost")
    print(f"  TT probe returned a move at    {pct(tt_present):5.1f}% of interior nodes  <-- ceiling")
    print(f"  ... of which never ranked      {pct(tt_unranked):5.1f}% of interior nodes")

    total_cuts = sum(classes) or 1
    cumulative = 0.0
    print("\n  First-move cutoffs by the stage that would have to produce the move:")
    for label, count in zip(STAGE_LABELS, classes):
        cumulative += pct(count)
        print(f"    {label:<30}{count:>10}{count * 100.0 / total_cuts:>7.1f}% of cutoffs"
              f"{pct(count):>7.1f}% of nodes   cumulative {cumulative:5.1f}%")

    print(f"\n  Node identity of the instrumented build: "
          f"{'confirmed' if identical else 'VIOLATED -- the measurement is invalid'}")
    return 0 if identical else 1


if __name__ == "__main__":
    sys.exit(main())
