#!/usr/bin/env python3
"""Tests for `scripts/sprt.py`.

Run with `python3 scripts/test_sprt.py`. Pure standard library, so it costs nothing to run
while a match occupies the machine.
"""

import math
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import sprt  # noqa: E402


class ExpectedScore(unittest.TestCase):
    def test_zero_elo_is_an_even_score(self):
        self.assertAlmostEqual(sprt.expected_score(0.0), 0.5)

    def test_the_scale_is_the_one_pairing_elo_inverts(self):
        # `sprt.expected_score` and `pairing_elo.elo` must be inverses, or the hypotheses
        # would be stated on a different scale from the numbers `task.md` records.
        for value in (-200.0, -13.2, 0.0, 7.5, 34.2, 400.0):
            self.assertAlmostEqual(sprt.elo(sprt.expected_score(value)), value, places=6)


class ConstrainedMle(unittest.TestCase):
    def test_the_fitted_distribution_has_the_requested_mean(self):
        pdf = [(0.0, 0.10), (0.25, 0.20), (0.5, 0.40), (0.75, 0.20), (1.0, 0.10)]
        for target in (0.45, 0.5, 0.55, 0.62):
            fitted = sprt._constrained_mle(pdf, target)
            self.assertIsNotNone(fitted)
            self.assertAlmostEqual(sum(weight for _, weight in fitted), 1.0, places=9)
            mean = sum(value * weight for value, weight in fitted)
            self.assertAlmostEqual(mean, target, places=9)

    def test_a_target_outside_the_support_has_no_solution(self):
        only_draws = [(0.0, 0.0), (0.25, 0.0), (0.5, 1.0), (0.75, 0.0), (1.0, 0.0)]
        self.assertIsNone(sprt._constrained_mle(only_draws, 0.55))

    def test_the_fit_is_the_identity_when_the_mean_already_matches(self):
        pdf = [(0.0, 0.25), (0.25, 0.0), (0.5, 0.5), (0.75, 0.0), (1.0, 0.25)]
        fitted = sprt._constrained_mle(pdf, 0.5)
        for (_, before), (_, after) in zip(pdf, fitted):
            self.assertAlmostEqual(before, after, places=9)


class LogLikelihoodRatio(unittest.TestCase):
    def test_evidence_for_a_gain_drives_the_ratio_up(self):
        # 60% over 400 pairs is far beyond +10 Elo and must accept H1.
        counts = [20, 60, 160, 100, 60]
        llr = sprt.log_likelihood_ratio(counts, 0.0, 10.0)
        self.assertGreater(llr, math.log(0.95 / 0.05))

    def test_evidence_against_a_gain_drives_the_ratio_down(self):
        # A dead level result is evidence for H0 over H1 and must eventually accept it. How
        # long that takes is governed by the variance of the pair distribution, not by the
        # score: this one is 800 pairs at a realistic spread and only just clears the bound.
        counts = [20, 80, 600, 80, 20]
        llr = sprt.log_likelihood_ratio(counts, 0.0, 10.0)
        self.assertLess(llr, math.log(0.05 / 0.95))

    def test_rejecting_a_gain_takes_longer_when_the_pairs_are_spread_out(self):
        # Same 500 pairs, same level score, wider outcome distribution: not yet decided. The
        # pentanomial test reads the shape of the results, not only their mean, and a draw-heavy
        # engine therefore settles a null result far sooner than a sharp one.
        llr = sprt.log_likelihood_ratio([50, 100, 200, 100, 50], 0.0, 10.0)
        self.assertLess(llr, 0.0)
        self.assertGreater(llr, math.log(0.05 / 0.95))

    def test_a_level_result_is_ambiguous_while_the_sample_is_small(self):
        counts = [1, 2, 4, 2, 1]
        llr = sprt.log_likelihood_ratio(counts, 0.0, 10.0)
        self.assertGreater(llr, math.log(0.05 / 0.95))
        self.assertLess(llr, math.log(0.95 / 0.05))

    def test_the_ratio_scales_with_the_number_of_observations(self):
        # The LLR of the same empirical distribution is linear in the sample size, which is
        # what makes the boundaries a stopping rule rather than a threshold on a score.
        small = sprt.log_likelihood_ratio([10, 20, 40, 25, 15], 0.0, 10.0)
        large = sprt.log_likelihood_ratio([100, 200, 400, 250, 150], 0.0, 10.0)
        self.assertAlmostEqual(large, 10.0 * small, places=6)

    def test_degenerate_observations_yield_no_ratio(self):
        self.assertIsNone(sprt.log_likelihood_ratio([0, 0, 12, 0, 0], 0.0, 10.0))
        self.assertIsNone(sprt.log_likelihood_ratio([0, 0, 0, 0, 0], 0.0, 10.0))

    def test_a_tighter_pair_distribution_decides_faster(self):
        # Two samples of the same size with the same level score. The one whose pairs cluster
        # on 1-1 carries more evidence per observation, because the hypotheses differ in the
        # mean and the mean of a low-variance sample is the better resolved. This is the whole
        # reason the test is run on pairs rather than on games: the colour swap removes the
        # variance that the opening and the first move contribute.
        tight = sprt.log_likelihood_ratio([5, 20, 150, 20, 5], 0.0, 10.0)
        loose = sprt.log_likelihood_ratio([40, 30, 60, 30, 40], 0.0, 10.0)
        self.assertLess(tight, loose)
        self.assertLess(tight, 0.0)


PGN_TEMPLATE = """[Event "Suprah-Tournament"]
[Site "local"]
[Round "{number}/{total}"]
[White "{white}"]
[Black "{black}"]
[Result "{result}"]

1. e2e4 {result}

"""


def write_pgn(games, total):
    handle = tempfile.NamedTemporaryFile("w", suffix=".pgn", delete=False)
    for number, (white, black, result) in enumerate(games, start=1):
        handle.write(PGN_TEMPLATE.format(number=number, total=total, white=white,
                                         black=black, result=result))
    handle.close()
    return handle.name


class CollectPairs(unittest.TestCase):
    def setUp(self):
        self.paths = []

    def tearDown(self):
        for path in self.paths:
            os.unlink(path)

    def _pgn(self, games, total):
        path = write_pgn(games, total)
        self.paths.append(path)
        return path

    def test_a_colour_swapped_pair_becomes_one_observation(self):
        # A won as White and drew as Black: 1.5 out of 2, i.e. a normalised 0.75.
        path = self._pgn([("A", "B", "1-0"), ("B", "A", "1/2-1/2")], 2)
        self.assertEqual(sprt.collect_pairs([path], "A", "B"), [0.75])

    def test_an_incomplete_pair_is_dropped(self):
        path = self._pgn([("A", "B", "1-0")], 2)
        self.assertEqual(sprt.collect_pairs([path], "A", "B"), [])

    def test_other_pairings_in_a_gauntlet_are_ignored(self):
        path = self._pgn([
            ("A", "B", "1-0"), ("B", "A", "0-1"),
            ("A", "C", "0-1"), ("C", "A", "1-0"),
        ], 4)
        self.assertEqual(sprt.collect_pairs([path], "A", "B"), [1.0])
        self.assertEqual(sprt.collect_pairs([path], "A", "C"), [0.0])

    def test_the_perspective_follows_the_first_engine(self):
        path = self._pgn([("A", "B", "1-0"), ("B", "A", "1-0")], 2)
        self.assertEqual(sprt.collect_pairs([path], "A", "B"), [0.5])
        self.assertEqual(sprt.collect_pairs([path], "B", "A"), [0.5])

    def test_runs_of_different_lengths_are_not_paired_with_each_other(self):
        # `Round` carries "<game>/<total>". Two appended runs reuse game numbers, and pairing
        # across them would invent observations that were never played.
        first = self._pgn([("A", "B", "1-0")], 2)
        second = self._pgn([("B", "A", "1-0")], 400)
        self.assertEqual(sprt.collect_pairs([first, second], "A", "B"), [])


class Bucketing(unittest.TestCase):
    def test_every_pair_outcome_lands_in_its_own_category(self):
        self.assertEqual(sprt.bucket([0.0, 0.25, 0.5, 0.75, 1.0]), [1, 1, 1, 1, 1])

    def test_counts_accumulate(self):
        self.assertEqual(sprt.bucket([0.5] * 7 + [1.0] * 3), [0, 0, 7, 0, 3])


class Verdict(unittest.TestCase):
    def setUp(self):
        self.lower = math.log(0.05 / 0.95)
        self.upper = math.log(0.95 / 0.05)

    def test_the_bounds_are_inclusive_and_map_to_exit_codes(self):
        self.assertEqual(sprt.verdict(self.upper, self.lower, self.upper), ("H1 accepted", 0))
        self.assertEqual(sprt.verdict(self.lower, self.lower, self.upper), ("H0 accepted", 1))
        self.assertEqual(sprt.verdict(0.0, self.lower, self.upper), ("undecided", 2))
        self.assertEqual(sprt.verdict(None, self.lower, self.upper), ("undecided", 2))


if __name__ == "__main__":
    unittest.main(verbosity=2)
