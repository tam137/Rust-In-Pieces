#!/usr/bin/env python3
"""Tests for `scripts/measure_cold_warm_drift.py`.

Run with `python3 scripts/test_cold_warm_drift.py`. Pure standard library apart from the
sequence-legality test, which is skipped when `python-chess` is absent, so it costs nothing to run
while a match occupies the machine.

Only the arithmetic is covered here. Driving the engine is not unit-testable and is verified
instead by the script's `--self-check` mode, which requires two cold passes to agree position for
position before any warm number is believed.
"""

import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import measure_cold_warm_drift as gate  # noqa: E402


class ScoreParsing(unittest.TestCase):
    def test_a_centipawn_score(self):
        self.assertEqual(gate.parse_score("cp 54"), ("cp", 54))

    def test_a_negative_centipawn_score(self):
        self.assertEqual(gate.parse_score("cp -128"), ("cp", -128))

    def test_a_mate_score_keeps_its_kind(self):
        # A mate score is a distance in moves, not a centipawn value. Averaging one into a drift
        # figure would produce a number in no unit at all.
        self.assertEqual(gate.parse_score("mate 3"), ("mate", 3))
        self.assertEqual(gate.parse_score("mate -2"), ("mate", -2))

    def test_whitespace_and_case_do_not_matter(self):
        self.assertEqual(gate.parse_score("  CP  54 "), ("cp", 54))

    def test_an_absent_or_malformed_score_is_none(self):
        for text in ("", "   ", "cp", "cp x", "lowerbound 12", None):
            self.assertIsNone(gate.parse_score(text), text)


class Drift(unittest.TestCase):
    def test_two_centipawn_scores_drift_by_their_difference(self):
        self.assertEqual(gate.score_drift("cp 44", "cp 103"), (59, False))

    def test_drift_is_unsigned(self):
        self.assertEqual(gate.score_drift("cp 103", "cp 44"), (59, False))

    def test_identical_mate_scores_do_not_drift(self):
        self.assertEqual(gate.score_drift("mate 4", "mate 4"), (0, False))

    def test_a_different_mate_distance_is_a_mismatch_not_a_distance(self):
        # `mate 4` against `mate 9` is five moves of disagreement, not five centipawns.
        self.assertEqual(gate.score_drift("mate 4", "mate 9"), (None, True))

    def test_a_mate_against_a_centipawn_score_is_the_worst_case_and_carries_no_delta(self):
        self.assertEqual(gate.score_drift("cp 300", "mate 5"), (None, True))
        self.assertEqual(gate.score_drift("mate 5", "cp 300"), (None, True))

    def test_a_missing_score_is_neither_a_drift_nor_a_mismatch(self):
        self.assertEqual(gate.score_drift("", "cp 12"), (None, False))


def entries(*pairs):
    return [gate.Entry(ply, cold, warm) for ply, (cold, warm) in enumerate(pairs)]


class Summarise(unittest.TestCase):
    def test_the_threshold_is_the_one_8_1_recorded(self):
        # 8.1's table counts positions drifting more than 50cp. Changing this constant would put
        # the result on a different scale from the reference it is read against.
        self.assertEqual(gate.DRIFT_THRESHOLD_CP, 50)

    def test_a_build_that_does_not_drift(self):
        s = gate.summarise("A", entries(("cp 20", "cp 20"), ("cp -8", "cp -3")))
        self.assertEqual((s.positions, s.comparable, s.drifting), (2, 2, 0))
        self.assertEqual(s.maximum, 5)
        self.assertAlmostEqual(s.mean, 2.5)

    def test_the_threshold_is_exclusive(self):
        exactly = gate.summarise("A", entries(("cp 0", "cp 50")))
        beyond = gate.summarise("A", entries(("cp 0", "cp 51")))
        self.assertEqual(exactly.drifting, 0)
        self.assertEqual(beyond.drifting, 1)

    def test_the_worst_position_is_named(self):
        s = gate.summarise("A", entries(("cp 0", "cp 5"), ("cp 0", "cp 90"), ("cp 0", "cp 7")))
        self.assertEqual(s.worst_ply, 1)
        self.assertEqual(s.maximum, 90)

    def test_a_mate_mismatch_is_counted_and_kept_out_of_the_mean(self):
        s = gate.summarise("A", entries(("cp 10", "cp 20"), ("cp 300", "mate 5")))
        self.assertEqual(s.mate_mismatches, 1)
        self.assertEqual(s.comparable, 1)
        self.assertAlmostEqual(s.mean, 10.0)
        # It is still a disagreement about the same position, so it counts as drifting.
        self.assertEqual(s.drifting, 1)

    def test_no_comparable_position_does_not_divide_by_zero(self):
        s = gate.summarise("A", entries(("", ""), ("", "")))
        self.assertEqual((s.comparable, s.drifting, s.maximum), (0, 0, 0))
        self.assertAlmostEqual(s.mean, 0.0)
        self.assertIsNone(s.worst_ply)


class Verdict(unittest.TestCase):
    def test_a_run_at_fail_hard_levels_is_clean(self):
        # 8.1's fail-hard reference: 0 of 60 above 50cp, mean 5.5, max 31.
        s = gate.summarise("A", entries(("cp 0", "cp 31"), ("cp 0", "cp 4")))
        self.assertTrue(gate.is_at_fail_hard_levels(s))

    def test_one_position_beyond_the_threshold_is_not_clean(self):
        s = gate.summarise("A", entries(("cp 0", "cp 51")))
        self.assertFalse(gate.is_at_fail_hard_levels(s))

    def test_a_mate_mismatch_alone_is_not_clean(self):
        s = gate.summarise("A", entries(("cp 300", "mate 5")))
        self.assertFalse(gate.is_at_fail_hard_levels(s))


class Attribution(unittest.TestCase):
    def summaries(self, total, no_lazy, no_bound, floor):
        def drifters(count):
            return entries(*([("cp 0", "cp 200")] * count + [("cp 0", "cp 1")] * (10 - count)))
        return gate.attribute(
            gate.summarise("A", drifters(total)),
            gate.summarise("B", drifters(no_lazy)),
            gate.summarise("C", drifters(no_bound)),
            gate.summarise("D", drifters(floor)),
        )

    def test_drift_that_disappears_with_lazy_evaluation_off_belongs_to_10_8(self):
        a = self.summaries(total=8, no_lazy=1, no_bound=7, floor=0)
        self.assertEqual(a.lazy_share, 7)
        self.assertEqual(a.bound_share, 1)
        self.assertFalse(a.third_source)

    def test_drift_that_disappears_with_a_colour_blind_bound_belongs_to_7_1(self):
        a = self.summaries(total=8, no_lazy=7, no_bound=2, floor=0)
        self.assertEqual(a.bound_share, 6)
        self.assertEqual(a.lazy_share, 1)

    def test_drift_surviving_both_levers_is_a_third_source(self):
        # Neither open item names it, so it must not be attributed to either.
        a = self.summaries(total=9, no_lazy=6, no_bound=6, floor=5)
        self.assertTrue(a.third_source)
        self.assertEqual(a.floor, 5)

    def test_a_clean_baseline_attributes_nothing(self):
        a = self.summaries(total=0, no_lazy=0, no_bound=0, floor=0)
        self.assertEqual((a.lazy_share, a.bound_share, a.floor), (0, 0, 0))
        self.assertFalse(a.third_source)


class Sequence(unittest.TestCase):
    def test_it_is_sixty_positions(self):
        self.assertEqual(len(gate.SEQUENCE), 60)
        self.assertEqual(len(gate.positions_of(gate.SEQUENCE)), 60)

    def test_the_first_position_is_the_start_position(self):
        self.assertEqual(gate.positions_of(gate.SEQUENCE)[0], ())

    def test_every_position_is_the_one_before_it_plus_a_move(self):
        found = gate.positions_of(gate.SEQUENCE)
        for earlier, later in zip(found, found[1:]):
            self.assertEqual(later[:-1], earlier)

    def test_the_moves_are_legal_and_reach_an_endgame(self):
        try:
            import chess
        except ImportError:
            self.skipTest("python-chess is not installed")
        board = chess.Board()
        for move in gate.SEQUENCE:
            parsed = chess.Move.from_uci(move)
            self.assertIn(parsed, board.legal_moves, f"{move} is not legal here")
            board.push(parsed)
        # The gate is worthless if the sequence never leaves the opening: neither hash table fills
        # up, and 10.8 needs repeated pawn structures to show at all.
        self.assertLessEqual(len(board.piece_map()), 16)


if __name__ == "__main__":
    unittest.main(verbosity=2)
