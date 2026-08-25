#!/usr/bin/env python3
"""Node-identity check for the Stage-0 short-circuit (`task.md` 1.2.2, released in v0.33.0).

Runs the same `--features search-diag` binary twice per position, once with `EnableTtMoveFirst`
on and once off, and compares the tree the search actually walked.

The UCI `nodes` field cannot be used for this: it reports `Stats::created_nodes`, the number of
*generated* moves, which Stage 0 legitimately reduces. The comparison is made on
`SEARCHTREE calculated=/eval=` -- interior moves played and Quiescence entries -- plus depth,
score and the principal variation of every completed iteration.

Usage:
    scripts/verify_stage0_identity.py [--depth 10] [--skip-build]
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

sys.path.insert(0, SCRIPT_DIR)
from measure_stage0 import POSITIONS, INFO_KEYS  # noqa: E402

TREE = re.compile(r"SEARCHTREE calculated=(\d+) eval=(\d+)")


def build():
    subprocess.run(["cargo", "build", "--release", "--features", "search-diag"],
                   cwd=PROJECT_ROOT, check=True,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    os.makedirs(BUILD_DIR, exist_ok=True)
    target = os.path.join(BUILD_DIR, "suprah-identity")
    shutil.copy2(os.path.join(PROJECT_ROOT, "target", "release", "suprah"), target)
    return target


def run(binary, fen, depth, enabled, settle):
    cmd = (
        'echo "uci"; echo "setoption name OwnBook value false"; '
        f'echo "setoption name EnableTtMoveFirst value {str(enabled).lower()}"; '
        f'echo "isready"; echo "position fen {fen}"; echo "go depth {depth}"; '
        f'sleep {settle}; echo "quit"'
    )
    res = subprocess.run(f"({cmd}) | {binary}", shell=True, capture_output=True,
                         text=True, timeout=300, cwd=PROJECT_ROOT)
    return res.stdout, res.stderr


def info_signature(stdout):
    signature = []
    for line in stdout.splitlines():
        if not line.startswith("info depth"):
            continue
        tokens = line.split()
        entry = {}
        for key in INFO_KEYS:
            if key == "nodes":  # generated moves -- expected to differ, see the module docstring
                continue
            if key in tokens:
                idx = tokens.index(key)
                entry[key] = " ".join(tokens[idx + 1:]) if key == "pv" else tokens[idx + 1]
        signature.append(tuple(sorted(entry.items())))
    return signature


def tree_size(stderr):
    hits = TREE.findall(stderr)
    return tuple(int(x) for x in hits[-1]) if hits else (0, 0)


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("-d", "--depth", type=int, default=10)
    parser.add_argument("-s", "--settle", type=float, default=3.0)
    parser.add_argument("--skip-build", action="store_true")
    args = parser.parse_args()

    binary = os.path.join(BUILD_DIR, "suprah-identity")
    if not args.skip_build:
        print("building search-diag binary ...", file=sys.stderr)
        binary = build()
    elif not os.path.exists(binary):
        sys.exit("no previously built binary in target/stage0-measurement")

    header = (f"{'Position':<18}{'searched off':>14}{'searched on':>14}"
              f"{'qsearch off':>13}{'qsearch on':>13}  verdict")
    print(f"\nFixed depth {args.depth}, {len(POSITIONS)} positions\n")
    print(header)
    print("-" * len(header))

    all_ok = True
    for name, fen in POSITIONS:
        out_off, err_off = run(binary, fen, args.depth, False, args.settle)
        out_on, err_on = run(binary, fen, args.depth, True, args.settle)

        calc_off, qs_off = tree_size(err_off)
        calc_on, qs_on = tree_size(err_on)
        same_tree = (calc_off, qs_off) == (calc_on, qs_on)
        same_info = info_signature(out_off) == info_signature(out_on)
        ok = same_tree and same_info
        all_ok &= ok

        verdict = "identical" if ok else (
            "TREE DIFFERS" if not same_tree else "PV/SCORE DIFFERS")
        print(f"{name:<18}{calc_off:>14}{calc_on:>14}{qs_off:>13}{qs_on:>13}  {verdict}")

    print("-" * len(header))
    print(f"\n  Stage-0 node identity: "
          f"{'confirmed' if all_ok else 'VIOLATED -- do not release'}")
    return 0 if all_ok else 1


if __name__ == "__main__":
    sys.exit(main())
