#!/usr/bin/env python3
"""Node-identity check for the negamax refactor (`task.md` section 7).

The refactor converts `SearchService::minimax` from an asymmetric absolute-score minimax --
parallel `if white { ... } else { ... }` blocks across every pruning rule, the PVS null window
and the Transposition Table -- to a canonical negamax with side-to-move-relative scores. It is a
pure refactor: the search tree it walks must be bit-identical to the build it replaces.

This script proves that by running two binaries over the same corpus and comparing the tree they
actually walked, not the `nodes` field. Per `task.md` rule 4, UCI `nodes` reports *generated*
moves, so it is recorded but never used as the criterion. The criterion is
`SEARCHTREE calculated=/eval=` -- interior moves played and Quiescence entries -- plus the depth,
score and principal variation of every completed iteration.

Both binaries must be built with `--features search-diag`, otherwise no `SEARCHTREE` line is
emitted and the comparison silently degrades to the info lines alone; the run is refused in that
case rather than reported as identical.

Searches are driven through `uci_driver`, which reads stdout until `bestmove` instead of
sleeping for a fixed interval, so a position is always compared against the same completed
fixed-depth search.

The corpus is `measure_stage0.POSITIONS` plus a **colour-swapped copy of every position**. The
shared corpus is white-to-move throughout, which is precisely the blind spot a colour-symmetry
refactor has: a sign error on the Black branch would leave every shared position identical.

Usage:
    scripts/verify_negamax_identity.py --baseline <path> --candidate <path> [--depth 9]
"""
import argparse
import os
import re
import sys

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.dirname(SCRIPT_DIR)

sys.path.insert(0, SCRIPT_DIR)
import uci_driver  # noqa: E402
from measure_stage0 import POSITIONS  # noqa: E402

TREE = re.compile(r"SEARCHTREE calculated=(\d+) eval=(\d+)")


def swap_colours(fen):
    """The same position with the colours exchanged and the board flipped top to bottom.

    Rank order is reversed and every piece letter changes case, so the resulting position is the
    mirror image with the other side to move. Castling rights swap case as well; the en passant
    square's rank is mirrored. The result is a legal FEN describing a strategically identical
    position for the other colour, which is what makes a sign error on the Black branch visible.
    """
    board, side, castling, ep, halfmove, fullmove = fen.split()

    ranks = board.split("/")
    flipped = "/".join(r.swapcase() for r in reversed(ranks))

    side = "b" if side == "w" else "w"
    castling = "-" if castling == "-" else "".join(sorted(castling.swapcase()))
    if ep != "-":
        ep = ep[0] + str(9 - int(ep[1]))

    return " ".join([flipped, side, castling, ep, halfmove, fullmove])


def corpus():
    entries = [(f"{name} (w)", fen) for name, fen in POSITIONS]
    entries += [(f"{name} (b, mirrored)", swap_colours(fen)) for name, fen in POSITIONS]
    return entries


def run(binary, fen, depth):
    return uci_driver.search(binary, fen, depth, timeout=600, cwd=PROJECT_ROOT)


def tree_signature(stderr):
    return [(int(c), int(e)) for c, e in TREE.findall(stderr)]


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--baseline", required=True,
                        help="a search-diag build of the pre-refactor engine")
    parser.add_argument("--candidate", required=True,
                        help="a search-diag build of the refactored engine")
    parser.add_argument("--depth", type=int, default=9)
    args = parser.parse_args()

    positions = corpus()
    failures = []

    for name, fen in positions:
        base = run(args.baseline, fen, args.depth)
        cand = run(args.candidate, fen, args.depth)

        base_tree, cand_tree = tree_signature(base.stderr), tree_signature(cand.stderr)

        problems = []
        if not base_tree:
            problems.append("baseline emitted no SEARCHTREE line -- is it a search-diag build?")
        if not cand_tree:
            problems.append("candidate emitted no SEARCHTREE line -- is it a search-diag build?")
        if base_tree != cand_tree:
            problems.append(f"tree {base_tree} != {cand_tree}")
        if base.best_move != cand.best_move:
            problems.append(f"bestmove {base.best_move} != {cand.best_move}")
        if base.info_signature != cand.info_signature:
            for depth_idx, (a, b) in enumerate(zip(base.info_signature, cand.info_signature)):
                if a != b:
                    problems.append(f"iteration {depth_idx + 1}: {a} != {b}")
                    break
            else:
                problems.append(f"iteration count {len(base.info_signature)} "
                                f"!= {len(cand.info_signature)}")

        status = "OK  " if not problems else "FAIL"
        tree = base_tree[-1] if base_tree else ("?", "?")
        print(f"{status} {name:<28} d{base.depth:<3} {base.best_move:<6} "
              f"calculated={tree[0]} eval={tree[1]}", flush=True)
        for problem in problems:
            print(f"       {problem}", flush=True)
        if problems:
            failures.append(name)

    print()
    print(f"{len(positions) - len(failures)}/{len(positions)} positions identical "
          f"at depth {args.depth}")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
