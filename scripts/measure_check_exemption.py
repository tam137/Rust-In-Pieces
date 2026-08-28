#!/usr/bin/env python3
"""Sizes the check exemption of `task.md` section 11.

A move that gives check is currently exempt from all four of Late Move Reductions, Late Move
Pruning, Futility Pruning and the SEE pruning of bad captures. Section 11 proposes damping that
exemption, and makes the whole item conditional on this measurement: if the class is small, or if
the rules would not have fired on it anyway, the item ends here.

The script builds the working tree twice -- with and without the `search-diag` Cargo feature --
searches a position corpus at fixed depth on both, and reports:

  * how much of the searched tree is inside the exempt class, and
  * how much of it each of the four rules would actually have removed if its `gives_check` guard
    were dropped. That second number, not the first, is the size of the prize.

Node identity between the two builds is asserted, the same gate the Stage-0 measurement uses:
counters that perturb the tree they count would invalidate the result.

Usage:
    scripts/measure_check_exemption.py [--depth 10] [--skip-build]
"""
import argparse
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from measure_stage0 import BUILD_DIR, POSITIONS, PROJECT_ROOT, build  # noqa: E402
from uci_driver import search  # noqa: E402

CHECK = re.compile(
    r"SEARCHDIAGCHECK searched=(\d+) checks=(\d+) quiet_checks=(\d+) in_check=(\d+) "
    r"subtree_total=(\d+) subtree_check=(\d+) lmr_blocked=(\d+) subtree_lmr=(\d+) "
    r"lmp_blocked=(\d+) subtree_lmp=(\d+) fp_blocked=(\d+) subtree_fp=(\d+) "
    r"see_blocked=(\d+) subtree_see=(\d+)"
)
FIELDS = ("searched", "checks", "quiet_checks", "in_check", "subtree_total", "subtree_check",
          "lmr_blocked", "subtree_lmr", "lmp_blocked", "subtree_lmp", "fp_blocked",
          "subtree_fp", "see_blocked", "subtree_see")

RULES = [
    ("Late Move Reductions", "lmr_blocked", "subtree_lmr", "reduced, not deleted"),
    ("Late Move Pruning", "lmp_blocked", "subtree_lmp", "deleted outright"),
    ("Futility Pruning", "fp_blocked", "subtree_fp", "deleted outright"),
    ("SEE pruning of bad captures", "see_blocked", "subtree_see", "deleted outright"),
]


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("-d", "--depth", type=int, default=10, help="fixed search depth")
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

    totals = dict.fromkeys(FIELDS, 0)
    identical = True

    print(f"\nFixed depth {args.depth}, {len(POSITIONS)} positions\n")
    header = (f"{'Position':<18}{'searched':>11}{'checks':>9}{'%':>7}{'in_check':>10}{'%':>7}"
              f"{'blocked':>9}{'sub_blk':>9}{'%tree':>8}  tree")
    print(header)
    print("-" * len(header))

    for name, fen in POSITIONS:
        res_base = search(base, fen, args.depth, cwd=PROJECT_ROOT)
        res_diag = search(diag, fen, args.depth, cwd=PROJECT_ROOT)
        same = res_base.info_signature == res_diag.info_signature
        identical &= same

        # The counters are cumulative over every iteration of the iterative deepening, because
        # `dump` runs at the end of each `get_moves` call and the statics are never reset. The
        # last line is therefore the total for the whole `go depth N`, not for its last ply.
        hits = CHECK.findall(res_diag.stderr)
        row = dict(zip(FIELDS, (int(x) for x in hits[-1]))) if hits else dict.fromkeys(FIELDS, 0)
        for key in FIELDS:
            totals[key] += row[key]

        searched = row["searched"] or 1
        blocked = sum(row[c] for _, c, _, _ in RULES)
        sub_blocked = sum(row[s] for _, _, s, _ in RULES)
        print(f"{name:<18}{row['searched']:>11}{row['checks']:>9}"
              f"{row['checks'] * 100.0 / searched:>6.1f}%{row['in_check']:>10}"
              f"{row['in_check'] * 100.0 / searched:>6.1f}%{blocked:>9}{sub_blocked:>9}"
              f"{sub_blocked * 100.0 / searched:>7.2f}%  {'identical' if same else 'DIFFERS'}")

    # `searched` is incremented on exactly the same moves as `Stats::calculated_nodes`, so it is
    # the size of the searched tree and the correct denominator for every share below.
    searched = totals["searched"] or 1
    pct = lambda value: value * 100.0 / searched
    blocked_all = sum(totals[c] for _, c, _, _ in RULES)
    sub_blocked_all = sum(totals[s] for _, _, s, _ in RULES)

    print("-" * len(header))
    print(f"{'TOTAL':<18}{totals['searched']:>11}{totals['checks']:>9}"
          f"{pct(totals['checks']):>6.1f}%{totals['in_check']:>10}"
          f"{pct(totals['in_check']):>6.1f}%{blocked_all:>9}{sub_blocked_all:>9}"
          f"{pct(sub_blocked_all):>7.2f}%  {'identical' if identical else 'DIFFERS'}\n")

    print("  How big is the exempt class?")
    print(f"    searched moves that give check      {totals['checks']:>10}  {pct(totals['checks']):5.1f}%")
    print(f"    ... of which quiet                  {totals['quiet_checks']:>10}  {pct(totals['quiet_checks']):5.1f}%")
    print(f"    moves searched while in check       {totals['in_check']:>10}  {pct(totals['in_check']):5.1f}%"
          "   <- clean partition")
    print(f"    tree below a checking move          {totals['subtree_check']:>10}  "
          f"{pct(totals['subtree_check']):5.1f}%   <- nests, so an upper bound")

    print("\n  What would each rule have removed if its gives_check guard were dropped?")
    print(f"    {'rule':<30}{'moves':>10}{'% moves':>10}{'subtree':>10}{'% of tree':>11}")
    for label, count_key, sub_key, _ in RULES:
        print(f"    {label:<30}{totals[count_key]:>10}{pct(totals[count_key]):>9.2f}%"
              f"{totals[sub_key]:>10}{pct(totals[sub_key]):>10.2f}%")
    print(f"    {'ALL FOUR':<30}{blocked_all:>10}{pct(blocked_all):>9.2f}%"
          f"{sub_blocked_all:>10}{pct(sub_blocked_all):>10.2f}%   <- the whole prize")

    print(f"\n  Node identity of the instrumented build: "
          f"{'confirmed' if identical else 'VIOLATED -- the measurement is invalid'}")
    return 0 if identical else 1


if __name__ == "__main__":
    sys.exit(main())
