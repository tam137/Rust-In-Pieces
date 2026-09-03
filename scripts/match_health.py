#!/usr/bin/env python3
"""Trustworthiness checks on a Matt-Magie PGN, before its Elo is believed.

`task.md` prices every search change by matchplay, which makes the harness itself part of the
measurement chain. This reports the things that quietly invalidate a run:

* **Losses on time.** Matt-Magie writes `WhiteWinByTime` / `BlackWinByTime` into `Termination`.
  At a 1s + 0.1s time control a machine running too many concurrent games starts forfeiting,
  and the forfeits do not fall evenly on the two engines. Any at all is a reason to lower
  `concurrency` before trusting the result.
* **Duplicate games.** The opening pool is finite - 598 lines - and a run longer than the pool
  replays lines. Two games from the same line, the same colours and the same two engines carry
  one game's worth of information between them but are counted as two.
* **Colour bias.** A pool of even-ply lines hands over with White to move, and every pairing is
  played twice with the colours swapped, so a large residual White score is a property of the
  time control rather than of either engine. It is reported because it sets the floor on how
  much variance the pairing has to overcome.
* **The pentanomial shape**, which is what actually determines how many games a comparison
  needs. See `scripts/sprt.py --plan`.

Usage:
    scripts/match_health.py <mm>/gauntlet_lmp.pgn
"""

import argparse
import hashlib
import math
import os
import re
import sys
from collections import Counter, defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from pairing_elo import score_for  # noqa: E402


def parse_full_games(path):
    """Yields the tag dictionary and the move text of every game in a PGN."""
    with open(path, "r", encoding="utf-8", errors="ignore") as handle:
        content = handle.read()

    for block in re.split(r"(?=\[Event\s+)", content):
        if not block.strip():
            continue
        tags = dict(re.findall(r'\[(\w+)\s+"([^"]*)"\]', block))
        if not {"White", "Black", "Result"} <= tags.keys():
            continue
        moves = re.sub(r'\[[^\]]*\]', "", block).strip()
        yield tags, moves


def move_list(moves):
    """The moves of a game, with move numbers and the result token removed."""
    cleaned = re.sub(r"\d+\.", " ", moves)
    cleaned = re.sub(r"(1-0|0-1|1/2-1/2|\*)", " ", cleaned)
    return cleaned.split()


def opening_key(moves, plies):
    return " ".join(move_list(moves)[:plies])


def intraclass_correlation(groups):
    """One-way ANOVA estimate of the intraclass correlation, and the design effect it implies.

    `groups` maps a cluster label onto the observations in it. Returns (icc, design_effect,
    clusters, mean_size) or None when there is not enough structure to estimate it.

    A pool of opening lines is not a flat list: `openings/book_mixed.txt` has 598 lines but only
    17 distinct four-ply starts, so the lines are leaves of a narrow tree. Observations from the
    same branch resemble each other, and a confidence interval computed as if they were
    independent is too narrow by a factor of the design effect.
    """
    sizes = [len(values) for values in groups.values() if values]
    total = sum(sizes)
    clusters = len(sizes)
    if clusters < 2 or total <= clusters:
        return None

    flat = [value for values in groups.values() for value in values]
    grand_mean = sum(flat) / total

    between = sum(len(values) * (sum(values) / len(values) - grand_mean) ** 2
                  for values in groups.values() if values) / (clusters - 1)
    within = sum((value - sum(values) / len(values)) ** 2
                 for values in groups.values() if values
                 for value in values) / (total - clusters)

    # Effective cluster size, which is the plain mean only when the clusters are equal.
    effective_size = (total - sum(size ** 2 for size in sizes) / total) / (clusters - 1)
    denominator = between + (effective_size - 1) * within
    if denominator <= 0.0:
        return None
    icc = (between - within) / denominator
    icc = max(0.0, min(1.0, icc))
    mean_size = total / clusters
    return icc, 1.0 + (mean_size - 1) * icc, clusters, mean_size


def report(path, opening_plies, prefix_plies, cluster_plies):
    games = list(parse_full_games(path))
    if not games:
        sys.exit("no games found in %s" % path)

    print("%s: %d games\n" % (path, len(games)))

    # --- terminations ------------------------------------------------------------------
    terminations = Counter(tags.get("Termination", "?") for tags, _ in games)
    print("terminations")
    for name, count in terminations.most_common():
        print("  %-20s %6d  %5.1f%%" % (name, count, 100.0 * count / len(games)))

    forfeits = sum(count for name, count in terminations.items() if "ByTime" in name)
    if forfeits:
        by_engine = Counter()
        for tags, _ in games:
            name = tags.get("Termination", "")
            if "WhiteWinByTime" in name:
                by_engine[tags["Black"]] += 1
            elif "BlackWinByTime" in name:
                by_engine[tags["White"]] += 1
        print("\n  WARNING: %d games (%.1f%%) ended on the clock." % (
            forfeits, 100.0 * forfeits / len(games)))
        for engine, count in by_engine.most_common():
            print("    %-40s forfeited %d" % (engine, count))
        print("    Lower `concurrency` until this is zero, then re-run.")
    else:
        print("\n  no losses on time - the harness kept up with the time control")
    print()

    # --- colour bias -------------------------------------------------------------------
    white_score = 0.0
    counted = 0
    for tags, _ in games:
        value = score_for(tags["White"], tags["Black"], tags["Result"], tags["White"])
        if value is not None:
            white_score += value
            counted += 1
    if counted:
        print("colour")
        print("  White scores %.2f%% over %d games" % (100.0 * white_score / counted, counted))
        print()

    # --- duplicate games ---------------------------------------------------------------
    exact = defaultdict(list)
    for index, (tags, moves) in enumerate(games):
        key = (tags["White"], tags["Black"],
               hashlib.sha1(" ".join(move_list(moves)).encode()).hexdigest())
        exact[key].append(index)
    repeated = {key: rows for key, rows in exact.items() if len(rows) > 1}

    prefix = defaultdict(list)
    for index, (tags, moves) in enumerate(games):
        played = move_list(moves)
        if len(played) >= prefix_plies:
            key = (tags["White"], tags["Black"], " ".join(played[:prefix_plies]))
            prefix[key].append(index)
    near = {key: rows for key, rows in prefix.items() if len(rows) > 1}

    print("duplication")
    print("  identical games (same colours, same moves)      %6d in %d groups" % (
        sum(len(rows) - 1 for rows in repeated.values()), len(repeated)))
    print("  same colours and first %d plies                  %6d in %d groups" % (
        prefix_plies, sum(len(rows) - 1 for rows in near.values()), len(near)))
    # A handful of repeats over thousands of games is noise rather than a pool that is too
    # small: the 4800-game gauntlet of 2026-08-27 produced exactly one, and warning on that
    # teaches the reader to skip the warning. Half a percent is where it starts to matter.
    duplicate_share = sum(len(rows) - 1 for rows in repeated.values()) / len(games)
    if duplicate_share > 0.005:
        print("\n  WARNING: %.1f%% of games are exact repeats. They carry no new information"
              % (100.0 * duplicate_share))
        print("           but are counted as observations. The opening pool is too small for")
        print("           this run length - scripts/book_lines.py builds a wider one.")
    print()

    # --- opening reuse -----------------------------------------------------------------
    reuse = Counter(opening_key(moves, opening_plies) for _, moves in games)
    distribution = Counter(reuse.values())
    print("opening reuse (first %d plies, over all pairings)" % opening_plies)
    print("  %d distinct openings" % len(reuse))
    for times in sorted(distribution):
        print("    played %2dx   %5d openings" % (times, distribution[times]))
    print()

    # --- pentanomial shape per pairing --------------------------------------------------
    print("pair outcomes per pairing (feeds scripts/sprt.py --plan)")
    pairings = defaultdict(dict)
    roots = defaultdict(dict)
    for tags, moves in games:
        number = tags.get("Round", "")
        match = re.match(r"(\d+)(?:/(\S+))?", number)
        if not match:
            continue
        batch, index = match.group(2) or "", int(match.group(1))
        key = tuple(sorted((tags["White"], tags["Black"])))
        slot = (batch, (index - 1) // 2)
        value = score_for(tags["White"], tags["Black"], tags["Result"], key[0])
        if value is None:
            continue
        pairings[key].setdefault(slot, []).append(value)
        roots[key][slot] = opening_key(moves, cluster_plies)

    for key, slots in sorted(pairings.items()):
        complete = [sum(values) / 2.0 for values in slots.values() if len(values) == 2]
        if not complete:
            continue
        counts = Counter(complete)
        mean = sum(complete) / len(complete)
        variance = sum((v - mean) ** 2 for v in complete) / max(1, len(complete) - 1)
        print("  %s  vs  %s" % key)
        print("    %d pairs   %s   variance %.4f" % (
            len(complete),
            "  ".join("%s:%d" % (label, counts.get(score, 0))
                      for label, score in zip(("0-2", "0.5-1.5", "1-1", "1.5-0.5", "2-0"),
                                              (0.0, 0.25, 0.5, 0.75, 1.0))),
            variance))

        clustered = defaultdict(list)
        for slot, values in slots.items():
            if len(values) == 2:
                clustered[roots[key].get(slot, "?")].append(sum(values) / 2.0)
        estimate = intraclass_correlation(clustered)
        if estimate is not None:
            icc, design_effect, clusters, mean_size = estimate
            print("    %d opening families at %d plies, %.1f pairs each   "
                  "ICC %.3f   design effect %.2f" % (
                      clusters, cluster_plies, mean_size, icc, design_effect))
            print("    effective sample %d of %d pairs; intervals widen by %.0f%%" % (
                round(len(complete) / design_effect), len(complete),
                100.0 * (math.sqrt(design_effect) - 1.0)))
    print()


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("pgn", nargs="+")
    parser.add_argument("--opening-plies", type=int, default=8,
                        help="plies that identify an opening for the reuse count (default 8)")
    parser.add_argument("--prefix-plies", type=int, default=30,
                        help="plies that count as a near-duplicate game (default 30)")
    parser.add_argument("--cluster-plies", type=int, default=4,
                        help="plies that define an opening family for the design effect "
                             "(default 4)")
    args = parser.parse_args()
    for path in args.pgn:
        report(path, args.opening_plies, args.prefix_plies, args.cluster_plies)


if __name__ == "__main__":
    main()
