#!/usr/bin/env python3
"""Ratings for every engine that has played, on one scale anchored to a fixed opponent.

Rule 3 in `task.md` says never to read Elo off the Matt-Magie scoreboard, because that rating is
normalised to the pool average and two runs therefore sit on two different scales. The rule is
right, and the consequence is that the project has per-pairing numbers and no curve: there is no
answer to "how much has the engine gained since v0.30.3" that does not come from adding up
differences measured under different conditions.

This builds the curve properly. Every pairing in every PGN given is one observation of a rating
*difference*, with its own paired standard error. Weighted least squares turns those differences
into one rating per engine, with a chosen anchor pinned to zero. Runs connect through the engines
they share, which is what makes a permanent anchor worth the games it costs: an engine that plays
the anchor in its own run lands on the same scale as every engine that ever did.

    engines = <challenger>, <the anchor>, ...

An engine with no path of pairings to the anchor cannot be placed and is reported separately
rather than given a number.

The scale still assumes the anchor plays the same in every run it appears in. That holds for a
frozen binary at a fixed time control on one machine, and breaks across machines - which is worth
saying out loud, because it is exactly what happened to this project when the work moved hosts.

Usage:
    scripts/version_curve.py ../matt-magie/*.pgn --anchor 0.34.0
"""

import argparse
import math
import os
import sys
from collections import defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from pairing_elo import parse_games, score_for, elo, elo_interval  # noqa: E402


def pairwise_differences(paths):
    """One rating difference per (pairing, run), with the standard error of the paired estimate.

    Pairings are kept separate per file and per round total, because appending two runs to one
    PGN is normal here and they are not the same experiment.
    """
    buckets = defaultdict(lambda: defaultdict(list))
    for path in paths:
        for white, black, result, number in parse_games(path):
            if number is None:
                continue
            batch, index = number
            first, second = sorted((white, black))
            value = score_for(white, black, result, first)
            if value is None:
                continue
            buckets[(path, batch, first, second)][(index - 1) // 2].append(value)

    observations = []
    for (path, batch, first, second), slots in buckets.items():
        pairs = [sum(values) / 2.0 for values in slots.values() if len(values) == 2]
        if len(pairs) < 8:
            continue
        mean = sum(pairs) / len(pairs)
        if mean <= 0.0 or mean >= 1.0:
            continue
        variance = sum((value - mean) ** 2 for value in pairs) / (len(pairs) - 1)
        standard_error = math.sqrt(variance / len(pairs))
        if standard_error <= 0.0:
            continue

        # Elo is a nonlinear function of the score, so the interval is carried over by the
        # delta method and its half-width used as the error on the Elo scale.
        low, high = elo_interval(mean, standard_error)
        if not all(map(math.isfinite, (low, high))):
            continue
        observations.append({
            "run": os.path.basename(path) + ("#" + batch if batch else ""),
            "first": first,
            "second": second,
            "elo": elo(mean),
            "sigma": (high - low) / (2.0 * 1.96),
            "pairs": len(pairs),
        })
    return observations


def connected_to(anchor, observations):
    """Engines reachable from the anchor through played pairings."""
    neighbours = defaultdict(set)
    for item in observations:
        neighbours[item["first"]].add(item["second"])
        neighbours[item["second"]].add(item["first"])

    seen = {anchor}
    frontier = [anchor]
    while frontier:
        current = frontier.pop()
        for other in neighbours[current]:
            if other not in seen:
                seen.add(other)
                frontier.append(other)
    return seen


def solve(anchor, engines, observations):
    """Weighted least squares for the ratings, with `anchor` pinned to zero.

    Minimises sum over pairings of w * (r_first - r_second - d)^2 with w = 1 / sigma^2. The
    normal equations are a weighted graph Laplacian; pinning the anchor removes the one degree
    of freedom that a set of differences cannot determine.
    """
    free = [name for name in engines if name != anchor]
    index = {name: position for position, name in enumerate(free)}
    size = len(free)
    if size == 0:
        return {anchor: 0.0}, {}

    matrix = [[0.0] * size for _ in range(size)]
    vector = [0.0] * size
    for item in observations:
        weight = 1.0 / (item["sigma"] ** 2)
        first, second, difference = item["first"], item["second"], item["elo"]
        i = index.get(first)
        j = index.get(second)
        if i is not None:
            matrix[i][i] += weight
            vector[i] += weight * difference
        if j is not None:
            matrix[j][j] += weight
            vector[j] -= weight * difference
        if i is not None and j is not None:
            matrix[i][j] -= weight
            matrix[j][i] -= weight

    # Gauss-Jordan with partial pivoting, and the inverse kept for the standard errors.
    augmented = [row[:] + [1.0 if k == r else 0.0 for k in range(size)] + [vector[r]]
                 for r, row in enumerate(matrix)]
    for column in range(size):
        pivot = max(range(column, size), key=lambda r: abs(augmented[r][column]))
        if abs(augmented[pivot][column]) < 1e-12:
            return None, None
        augmented[column], augmented[pivot] = augmented[pivot], augmented[column]
        scale = augmented[column][column]
        augmented[column] = [value / scale for value in augmented[column]]
        for row in range(size):
            if row == column:
                continue
            factor = augmented[row][column]
            if factor:
                augmented[row] = [a - factor * b
                                  for a, b in zip(augmented[row], augmented[column])]

    ratings = {anchor: 0.0}
    errors = {anchor: 0.0}
    for name, position in index.items():
        ratings[name] = augmented[position][-1]
        errors[name] = math.sqrt(max(0.0, augmented[position][size + position]))
    return ratings, errors


def resolve_one(needle, names, role):
    """Maps a user-supplied fragment onto one engine name.

    An exact match wins outright. Without that rule a version prefix like "V0.34.0" would be
    ambiguous against its own measurement variants "V0.34.0-LMP" and "V0.34.0-BOTH", which is
    precisely the naming scheme `task.md` prescribes for them.
    """
    exact = [name for name in names if name == needle]
    if len(exact) == 1:
        return exact[0]
    matches = [name for name in names if needle.lower() in name.lower()]
    if len(matches) == 1:
        return matches[0]
    if not matches:
        sys.exit("%s '%s' matches no engine in these PGNs (%s)"
                 % (role, needle, ", ".join(names)))
    sys.exit("%s '%s' is ambiguous between %s; give the full id name"
             % (role, needle, ", ".join(matches)))


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("pgn", nargs="+")
    parser.add_argument("--anchor", required=True,
                        help="substring of the anchor engine's UCI id name")
    parser.add_argument("--min-pairs", type=int, default=8,
                        help="ignore pairings with fewer complete pairs (default 8)")
    args = parser.parse_args()

    observations = [item for item in pairwise_differences(args.pgn)
                    if item["pairs"] >= args.min_pairs]
    if not observations:
        sys.exit("no pairing in these PGNs has enough complete pairs")

    names = sorted({item["first"] for item in observations} |
                   {item["second"] for item in observations})
    anchor = resolve_one(args.anchor, names, "anchor")

    reachable = connected_to(anchor, observations)
    placed = sorted(reachable)
    stranded = [name for name in names if name not in reachable]
    usable = [item for item in observations
              if item["first"] in reachable and item["second"] in reachable]

    ratings, errors = solve(anchor, placed, usable)
    if ratings is None:
        sys.exit("the pairing graph is degenerate; add a run that shares an engine")

    print("anchor: %s  (pinned to 0)" % anchor)
    print("%d pairings over %d runs, %d engines placed\n"
          % (len(usable), len({item["run"] for item in usable}), len(placed)))

    print("  %-42s %9s %9s" % ("engine", "Elo", "+/- 95%"))
    for name in sorted(placed, key=lambda n: -ratings[n]):
        print("  %-42s %+9.1f %9s" % (
            name, ratings[name],
            "-" if name == anchor else "%.1f" % (1.96 * errors[name])))

    print("\npairings used")
    for item in sorted(usable, key=lambda o: (o["run"], o["first"])):
        residual = ratings[item["first"]] - ratings[item["second"]] - item["elo"]
        print("  %-28s %-30s vs %-30s %+7.1f  (%d pairs, residual %+.1f)" % (
            item["run"], item["first"], item["second"], item["elo"],
            item["pairs"], residual))

    if stranded:
        print("\nnot connected to the anchor - no rating on this scale:")
        for name in stranded:
            print("  %s" % name)
        print("  Include the anchor in every run and this disappears.")


if __name__ == "__main__":
    main()
