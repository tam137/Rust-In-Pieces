#!/usr/bin/env python3
"""Per-pairing Elo from a Matt-Magie PGN, with paired-opening confidence intervals.

The Matt-Magie scoreboard is a Bradley-Terry rating normalised to a pool average, so two ratings
from different PGNs are not comparable and `skills/engine_release_procedure.md` requires every
result to be read per pairing instead. This script does that, and adds the interval the
scoreboard does not report.

Two intervals are printed:

* **unpaired** treats every game as independent. This is what the earlier measurements in
  `task.md` assumed, and it is too narrow whenever the games of a match share openings.
* **paired** treats one opening, played once with each colour assignment, as a single
  observation. It is the honest interval for a run driven by an openings file, because the two
  games of a pair are strongly correlated by construction.

Usage:
    scripts/pairing_elo.py ../matt-magie/null_test.pgn
"""

import argparse
import math
import re
import sys
from collections import defaultdict


def parse_games(path):
    """Yields (white, black, result, game_number) for every game in a PGN."""
    with open(path, "r", encoding="utf-8", errors="ignore") as handle:
        content = handle.read()

    for block in re.split(r"(?=\[Event\s+)", content):
        if not block.strip():
            continue
        tags = dict(re.findall(r'\[(\w+)\s+"([^"]*)"\]', block))
        if not {"White", "Black", "Result"} <= tags.keys():
            continue

        # Matt-Magie writes the round as "<game number>/<total games>". The total is carried
        # along because a PGN is appended to across runs, and two runs of different lengths
        # otherwise reuse the same game numbers and would be paired with each other.
        round_tag = tags.get("Round", "")
        number = None
        match = re.match(r"(\d+)(?:/(\S+))?", round_tag)
        if match:
            number = (match.group(2) or "", int(match.group(1)))

        yield tags["White"], tags["Black"], tags["Result"], number


def score_for(white, black, result, engine):
    """Score of `engine` in one game, or None if it did not play or the game was unfinished."""
    if result == "1-0":
        winner, loser = white, black
    elif result == "0-1":
        winner, loser = black, white
    elif result in ("1/2-1/2", "1/2"):
        if engine in (white, black):
            return 0.5
        return None
    else:
        return None

    if engine == winner:
        return 1.0
    if engine == loser:
        return 0.0
    return None


def elo(score):
    if score <= 0.0:
        return float("-inf")
    if score >= 1.0:
        return float("inf")
    return -400.0 * math.log10(1.0 / score - 1.0)


def elo_interval(score, standard_error):
    """95% interval on the Elo scale, carried over from the score by the delta method."""
    if standard_error == 0 or score <= 0.0 or score >= 1.0:
        return (float("nan"), float("nan"))
    low = max(1e-9, score - 1.96 * standard_error)
    high = min(1.0 - 1e-9, score + 1.96 * standard_error)
    return (elo(low), elo(high))


def analyse(path, minimum_games):
    games = list(parse_games(path))
    if not games:
        sys.exit("no games found in %s" % path)

    # Group by unordered pairing.
    pairings = defaultdict(list)
    for white, black, result, number in games:
        pairings[tuple(sorted((white, black)))].append((white, black, result, number))

    print("%s: %d games, %d pairings\n" % (path, len(games), len(pairings)))

    for (first, second), played in sorted(pairings.items()):
        if len(played) < minimum_games:
            continue

        scores = []
        wins = draws = losses = 0
        by_pair = defaultdict(list)

        for white, black, result, number in played:
            value = score_for(white, black, result, first)
            if value is None:
                continue
            scores.append(value)
            if value == 1.0:
                wins += 1
            elif value == 0.5:
                draws += 1
            else:
                losses += 1
            if number is not None:
                batch, index = number
                by_pair[(batch, (index - 1) // 2)].append(value)

        total = len(scores)
        if total == 0:
            continue
        mean = sum(scores) / total

        # Unpaired: every game its own observation.
        variance = sum((value - mean) ** 2 for value in scores) / max(1, total - 1)
        unpaired_se = math.sqrt(variance / total)

        # Paired: one opening played with both colour assignments is one observation.
        complete_pairs = [values for values in by_pair.values() if len(values) == 2]
        paired_line = "paired    n/a (game numbering incomplete)"
        if len(complete_pairs) >= 2:
            pair_means = [sum(values) / 2.0 for values in complete_pairs]
            pair_mean = sum(pair_means) / len(pair_means)
            pair_variance = sum((value - pair_mean) ** 2 for value in pair_means) / (len(pair_means) - 1)
            paired_se = math.sqrt(pair_variance / len(pair_means))
            low, high = elo_interval(pair_mean, paired_se)
            paired_line = "paired    %+7.1f  95%% CI [%+.0f, %+.0f]   (%d pairs)" % (
                elo(pair_mean), low, high, len(pair_means))

        low, high = elo_interval(mean, unpaired_se)
        print("%s  vs  %s" % (first, second))
        print("  %d games   +%d =%d -%d   score %.1f%%" % (total, wins, draws, losses, mean * 100))
        print("  unpaired  %+7.1f  95%% CI [%+.0f, %+.0f]" % (elo(mean), low, high))
        print("  %s" % paired_line)
        print()


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("pgn", nargs="+", help="PGN files to analyse")
    parser.add_argument("--min-games", type=int, default=2,
                        help="skip pairings with fewer games than this")
    args = parser.parse_args()

    for path in args.pgn:
        analyse(path, args.min_games)


if __name__ == "__main__":
    main()
