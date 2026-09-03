#!/usr/bin/env python3
"""Sequential Probability Ratio Test over a Matt-Magie PGN, on paired openings.

Every measurement in `task.md` so far has run a fixed number of games chosen in advance from
the width of the confidence interval it needed. That is the expensive way to answer a yes/no
question: a run that is going to be decisive after 400 games still plays all 4800, and a run
whose true effect is zero plays all 4800 to say so. A sequential test stops as soon as the
evidence is conclusive in either direction, and for the +5 to +20 Elo features left in the
backlog that is usually a large fraction of the run.

The test is **pentanomial**, not per-game. Matt-Magie plays one opening line twice with the
colours swapped, so the two games of a pair are correlated by construction and a per-game test
understates its own error. The pair is the observation, and its normalised score falls into one
of five categories:

    0.00  lost both        0.25  half a point        0.50  level
    0.75  one and a half   1.00  won both

This is the same reason `scripts/pairing_elo.py` prints a paired interval, and the paired
interval is the one `task.md` calls honest.

The statistic is the generalised SPRT of Van den Bergh: for each hypothesis the empirical
pentanomial distribution is projected onto the set of distributions with the hypothesised mean
score, and the log-likelihood ratio is taken between the two projections. It needs no
assumption about the draw rate, which a normal approximation would.

Usage:
    # Verdict on the pairing between two engines, from whatever is in the PGN right now.
    scripts/sprt.py <mm>/gauntlet_lmp.pgn --engines BOTH LMP

    # How the test would have progressed, and where it would have stopped.
    scripts/sprt.py <mm>/gauntlet_lmp.pgn --engines BOTH LMP --trajectory

Exit codes are meant for a watchdog:
    0  H1 accepted - the first engine is stronger by at least `--elo1`
    1  H0 accepted - the gain is not there
    2  undecided, keep playing
    3  usage or parsing error
"""

import argparse
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from pairing_elo import parse_games, score_for, elo  # noqa: E402


# Normalised score of a pair, i.e. the mean of its two game scores.
PENTANOMIAL_SCORES = (0.0, 0.25, 0.5, 0.75, 1.0)
PENTANOMIAL_LABELS = ("0-2", "0.5-1.5", "1-1", "1.5-0.5", "2-0")


def expected_score(elo_value):
    """Expected score of a player rated `elo_value` above its opponent."""
    return 1.0 / (1.0 + 10.0 ** (-elo_value / 400.0))


def _constrained_mle(pdf, target):
    """Distribution closest to `pdf` in likelihood whose mean is exactly `target`.

    The solution has the form `q_j = p_j / (1 + x * (a_j - target))`, where `x` solves
    `sum_j p_j * (a_j - target) / (1 + x * (a_j - target)) = 0`. That constraint also makes the
    weights sum to one, so no separate normalisation is needed. Returns None when `target` lies
    outside the observed support, where no such distribution exists.
    """
    support = [value for value, weight in pdf if weight > 0.0]
    if not support:
        return None
    low, high = min(support), max(support)
    if not (low < target < high):
        return None

    def constraint(x):
        return sum(weight * (value - target) / (1.0 + x * (value - target))
                   for value, weight in pdf if weight > 0.0)

    # `1 + x * (a - target)` must stay positive for every observed `a`, which bounds `x` to
    # the open interval below. `constraint` is strictly decreasing on it and changes sign.
    epsilon = 1e-12
    lower = -1.0 / (high - target) + epsilon
    upper = 1.0 / (target - low) - epsilon

    left, right = lower, upper
    for _ in range(200):
        middle = 0.5 * (left + right)
        if constraint(middle) > 0.0:
            left = middle
        else:
            right = middle
    x = 0.5 * (left + right)

    return [(value, weight / (1.0 + x * (value - target))) for value, weight in pdf]


def log_likelihood_ratio(counts, elo0, elo1):
    """Cumulative LLR of the observed pair counts, for H0: `elo0` against H1: `elo1`.

    Returns None while the observations are still degenerate - all pairs level, say - because
    no distribution with the hypothesised mean can then be fitted to them.
    """
    total = sum(counts)
    if total == 0:
        return None

    empirical = [(score, count / total) for score, count in zip(PENTANOMIAL_SCORES, counts)]
    fitted0 = _constrained_mle(empirical, expected_score(elo0))
    fitted1 = _constrained_mle(empirical, expected_score(elo1))
    if fitted0 is None or fitted1 is None:
        return None

    per_observation = 0.0
    for (_, observed), (_, under0), (_, under1) in zip(empirical, fitted0, fitted1):
        if observed > 0.0:
            per_observation += observed * math.log(under1 / under0)
    return total * per_observation


def collect_pairs(paths, first, second):
    """Pair scores for `first` against `second`, in the order the games were played.

    A pair is two games carrying consecutive game numbers within the same run, which is how
    Matt-Magie schedules a colour swap over one opening line. Incomplete pairs - a run stopped
    mid-pair, or an unfinished game - are dropped rather than counted as halves.
    """
    partial = {}
    ordered = []
    for path in paths:
        for white, black, result, number in parse_games(path):
            if {white, black} != {first, second} or number is None:
                continue
            value = score_for(white, black, result, first)
            if value is None:
                continue
            batch, index = number
            key = (path, batch, (index - 1) // 2)
            if key in partial:
                ordered.append((partial.pop(key) + value) / 2.0)
            else:
                partial[key] = value
    return ordered


def bucket(pair_scores):
    counts = [0] * 5
    for value in pair_scores:
        counts[PENTANOMIAL_SCORES.index(min(PENTANOMIAL_SCORES,
                                            key=lambda s: abs(s - value)))] += 1
    return counts


def resolve_engine_names(paths, wanted):
    """Maps two user-supplied substrings onto the full engine names in the PGN."""
    names = set()
    for path in paths:
        for white, black, _, _ in parse_games(path):
            names.add(white)
            names.add(black)

    resolved = []
    for needle in wanted:
        # An exact match wins outright, or a version prefix like "V0.34.0" would be ambiguous
        # against its own measurement variants "V0.34.0-LMP" and "V0.34.0-BOTH".
        exact = [name for name in names if name == needle]
        matches = exact if len(exact) == 1 else [
            name for name in names if needle.lower() in name.lower()]
        if len(matches) != 1:
            sys.exit("'%s' matches %d engines in the PGN (%s); use a longer substring "
                     "or the full id name"
                     % (needle, len(matches), ", ".join(sorted(names)) or "none"))
        resolved.append(matches[0])
    if resolved[0] == resolved[1]:
        sys.exit("both patterns resolved to '%s'" % resolved[0])
    return resolved


def verdict(llr, lower_bound, upper_bound):
    if llr is None:
        return "undecided", 2
    if llr >= upper_bound:
        return "H1 accepted", 0
    if llr <= lower_bound:
        return "H0 accepted", 1
    return "undecided", 2


def report(first, second, pair_scores, elo0, elo1, lower_bound, upper_bound):
    counts = bucket(pair_scores)
    total = len(pair_scores)
    llr = log_likelihood_ratio(counts, elo0, elo1)
    label, code = verdict(llr, lower_bound, upper_bound)

    print("%s  vs  %s" % (first, second))
    print("  %d pairs (%d games)   %s" % (
        total, 2 * total,
        "  ".join("%s:%d" % (name, count)
                  for name, count in zip(PENTANOMIAL_LABELS, counts))))
    if total:
        mean = sum(pair_scores) / total
        print("  score %.2f%%   Elo %+.1f" % (100.0 * mean, elo(mean)))
    print("  H0 elo=%+g   H1 elo=%+g   bounds [%.3f, %.3f]" % (
        elo0, elo1, lower_bound, upper_bound))
    if llr is None:
        print("  LLR n/a (observations still degenerate)")
    else:
        print("  LLR %+.3f   %s" % (llr, label))
    return code


def trajectory(first, second, pair_scores, elo0, elo1, lower_bound, upper_bound, step):
    print("%s  vs  %s   - LLR trajectory, H0 %+g / H1 %+g, bounds [%.3f, %.3f]\n" % (
        first, second, elo0, elo1, lower_bound, upper_bound))
    print("   pairs    games      score      Elo        LLR   verdict")
    stopped_at = None
    for index in range(step, len(pair_scores) + 1, step):
        window = pair_scores[:index]
        llr = log_likelihood_ratio(bucket(window), elo0, elo1)
        label, _ = verdict(llr, lower_bound, upper_bound)
        mean = sum(window) / index
        print("  %6d   %6d   %7.2f%%   %+7.1f   %8s   %s" % (
            index, 2 * index, 100.0 * mean, elo(mean),
            "n/a" if llr is None else "%+.3f" % llr, label))
        if stopped_at is None and label != "undecided":
            stopped_at = (index, label)

    print()
    if stopped_at is None:
        print("  never decided over %d pairs (%d games)"
              % (len(pair_scores), 2 * len(pair_scores)))
        return 2
    index, label = stopped_at
    print("  first decided at %d pairs (%d games): %s" % (index, 2 * index, label))
    if len(pair_scores) > index:
        saved = 2 * (len(pair_scores) - index)
        print("  a sequential run would have stopped %d games (%.0f%%) earlier" % (
            saved, 100.0 * saved / (2 * len(pair_scores))))
    return 0 if label == "H1 accepted" else 1


def plan(pair_scores, elo0, elo1, lower_bound, upper_bound, grid):
    """Expected cost of the test, from the pair distribution the harness actually produces.

    A fixed-length run has to be sized before it starts, from a guess at the effect it is
    looking for. Getting that wrong is expensive in both directions, and `task.md` has an
    example of each: the LMP round robin resolved +/-23.5 Elo and was asked to settle +13.2,
    while the follow-up gauntlet is sized at 1600 games per pairing on the same guess.

    This estimates, for a range of true effects, how many games a sequential test would need.
    The distribution of pair outcomes is taken from real games rather than assumed, because it
    is the variance of that distribution - not the score - that sets the cost, and this engine's
    is high: barely a third of its pairs come back level.

    The estimate is Wald's: the boundary divided by the expected log-likelihood ratio per
    observation. It ignores overshoot past the boundary and is therefore mildly optimistic,
    by a few percent at these bounds.
    """
    total = len(pair_scores)
    counts = bucket(pair_scores)
    observed = [(score, count / total) for score, count in zip(PENTANOMIAL_SCORES, counts)]

    mean = sum(pair_scores) / total
    variance = sum((value - mean) ** 2 for value in pair_scores) / max(1, total - 1)

    print("pair distribution from %d pairs (%d games)" % (total, 2 * total))
    print("  %s" % "  ".join("%s:%.3f" % (name, weight)
                             for name, (_, weight) in zip(PENTANOMIAL_LABELS, observed)))
    print("  mean %.4f   variance %.4f   sd %.4f" % (mean, variance, math.sqrt(variance)))
    print()
    print("SPRT  H0 elo=%+g   H1 elo=%+g   bounds [%.3f, %.3f]" % (
        elo0, elo1, lower_bound, upper_bound))
    print()
    print("  true Elo   decides    exp. pairs   exp. games   fixed-N +/- at that many games")
    for elo_true in grid:
        tilted = _constrained_mle(observed, expected_score(elo_true))
        if tilted is None:
            print("  %+8.1f   %s" % (elo_true, "outside the observed support"))
            continue
        per_observation = log_likelihood_ratio(
            [weight for _, weight in tilted], elo0, elo1)
        if per_observation is None or abs(per_observation) < 1e-12:
            print("  %+8.1f   %s" % (elo_true, "never (the test cannot separate here)"))
            continue

        if per_observation > 0.0:
            pairs, decision = upper_bound / per_observation, "H1"
        else:
            pairs, decision = lower_bound / per_observation, "H0"
        pairs = int(math.ceil(pairs))

        # Between the two hypotheses the ratio has almost no drift and the test wanders. That
        # band is the price of stating a null and an alternative that are close together, and
        # printing the arithmetic there would suggest a precision the estimate does not have.
        if pairs > 100000:
            print("  %+8.1f   %s" % (
                elo_true, "indifference region - the test does not converge here"))
            continue

        # What a fixed-length run of the same size would have resolved, for comparison: the
        # half-width of its 95% interval on the paired observations.
        half_width = 1.96 * math.sqrt(variance / pairs)
        half_elo = 0.5 * (elo(min(0.999999, 0.5 + half_width)) -
                          elo(max(0.000001, 0.5 - half_width)))
        print("  %+8.1f   %s         %8d     %8d   %+.1f" % (
            elo_true, decision, pairs, 2 * pairs, half_elo))
    print()
    print("  Read the last column against the second: a fixed run of that length resolves an")
    print("  effect only when the effect is larger than the half-width shown.")
    return 0


def main():
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("pgn", nargs="+", help="PGN files to read")
    parser.add_argument("--engines", nargs=2, required=True, metavar=("A", "B"),
                        help="substrings identifying the two engines; A is the challenger")
    parser.add_argument("--elo0", type=float, default=0.0,
                        help="null hypothesis, in Elo (default 0)")
    parser.add_argument("--elo1", type=float, default=10.0,
                        help="alternative hypothesis, in Elo (default 10)")
    parser.add_argument("--alpha", type=float, default=0.05,
                        help="probability of accepting H1 when H0 holds (default 0.05)")
    parser.add_argument("--beta", type=float, default=0.05,
                        help="probability of accepting H0 when H1 holds (default 0.05)")
    parser.add_argument("--trajectory", action="store_true",
                        help="replay the PGN and report where the test would have stopped")
    parser.add_argument("--step", type=int, default=25,
                        help="pairs between trajectory rows (default 25)")
    parser.add_argument("--plan", action="store_true",
                        help="estimate how many games the test needs, from the observed "
                             "distribution of pair outcomes")
    parser.add_argument("--grid", type=float, nargs="+",
                        default=[-30.0, -20.0, -10.0, -5.0, 0.0, 5.0, 10.0, 13.0,
                                 20.0, 30.0, 50.0],
                        help="true Elo values to cost out under --plan")
    args = parser.parse_args()

    if not args.elo0 < args.elo1:
        sys.exit("--elo0 must be below --elo1")

    first, second = resolve_engine_names(args.pgn, args.engines)
    pair_scores = collect_pairs(args.pgn, first, second)
    if not pair_scores:
        sys.exit("no complete game pairs for %s vs %s" % (first, second))

    lower_bound = math.log(args.beta / (1.0 - args.alpha))
    upper_bound = math.log((1.0 - args.beta) / args.alpha)

    if args.plan:
        return plan(pair_scores, args.elo0, args.elo1, lower_bound, upper_bound, args.grid)
    if args.trajectory:
        return trajectory(first, second, pair_scores, args.elo0, args.elo1,
                          lower_bound, upper_bound, max(1, args.step))
    return report(first, second, pair_scores, args.elo0, args.elo1,
                  lower_bound, upper_bound)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except BrokenPipeError:
        sys.exit(3)
