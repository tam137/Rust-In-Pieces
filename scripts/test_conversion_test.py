#!/usr/bin/env python3
"""Tests for `scripts/conversion_test.py`.

Run with `python3 scripts/test_conversion_test.py`. Needs `python-chess` but no engine, so it costs
nothing to run while a match occupies the machine.
"""

import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import chess  # noqa: E402

import conversion_test  # noqa: E402


class McNemar(unittest.TestCase):
    def test_no_disagreement_proves_nothing(self):
        self.assertEqual(conversion_test.mcnemar_one_sided(0, 0), 1.0)

    def test_five_to_nothing_is_one_in_thirty_two(self):
        self.assertAlmostEqual(conversion_test.mcnemar_one_sided(5, 0), 1 / 32)

    def test_a_tie_is_the_upper_tail_of_a_fair_coin(self):
        # P(X >= 3) for X ~ Bin(6, 1/2) = (20 + 15 + 6 + 1) / 64.
        self.assertAlmostEqual(conversion_test.mcnemar_one_sided(3, 3), 42 / 64)

    def test_losing_every_disagreement_is_certain_under_the_null(self):
        self.assertAlmostEqual(conversion_test.mcnemar_one_sided(0, 7), 1.0)


class AttackerOf(unittest.TestCase):
    def test_queen_against_a_bare_king(self):
        board = chess.Board("4k3/8/8/8/8/8/8/3QK3 w - - 0 1")
        self.assertEqual(conversion_test.attacker_of(board), chess.WHITE)

    def test_rook_and_pawns_for_black(self):
        board = chess.Board("8/8/8/8/8/2k5/1p6/r6K b - - 0 1")
        self.assertEqual(conversion_test.attacker_of(board), chess.BLACK)

    def test_a_king_with_a_pawn_is_not_bare(self):
        board = chess.Board("4k3/4p3/8/8/8/8/8/3QK3 w - - 0 1")
        self.assertIsNone(conversion_test.attacker_of(board))

    def test_minor_pieces_alone_do_not_count(self):
        board = chess.Board("4k3/8/8/8/8/8/8/2BNK3 w - - 0 1")
        self.assertIsNone(conversion_test.attacker_of(board))


class GameOver(unittest.TestCase):
    def test_mate(self):
        board = chess.Board("k7/1Q6/1K6/8/8/8/8/8 b - - 0 1")
        self.assertEqual(conversion_test.game_over(board, 1, 200), "mate")

    def test_stalemate(self):
        board = chess.Board("k7/2Q5/1K6/8/8/8/8/8 b - - 0 1")
        self.assertEqual(conversion_test.game_over(board, 1, 200), "stalemate")

    def test_the_king_took_the_last_piece(self):
        board = chess.Board("8/8/8/3k4/8/8/8/4K3 w - - 0 1")
        self.assertEqual(conversion_test.game_over(board, 3, 200), "insufficient")

    def test_threefold(self):
        board = chess.Board("4k3/8/8/8/8/8/8/R3K3 w - - 0 1")
        for _ in range(2):
            for uci in ("a1a2", "e8d8", "a2a1", "d8e8"):
                board.push_uci(uci)
        self.assertEqual(conversion_test.game_over(board, 8, 200), "threefold")

    def test_cap(self):
        board = chess.Board("4k3/8/8/8/8/8/8/R3K3 w - - 0 1")
        self.assertEqual(conversion_test.game_over(board, 200, 200), "cap")
        self.assertIsNone(conversion_test.game_over(board, 199, 200))


class LoadPositions(unittest.TestCase):
    def test_tags_and_counters_survive(self):
        line = '6K1/8/8/8/5k2/8/2R5/8 b - - hmvc 7; fmvn 80; id "r18-145"; c0 "drawn";\n'
        with tempfile.NamedTemporaryFile("w", suffix=".epd", delete=False) as handle:
            handle.write(line)
            handle.write("\n")
            path = handle.name
        try:
            positions = conversion_test.load_positions(path)
        finally:
            os.unlink(path)
        self.assertEqual(len(positions), 1)
        position = positions[0]
        self.assertEqual((position.ident, position.origin), ("r18-145", "drawn"))
        self.assertEqual(position.board.halfmove_clock, 7)
        self.assertEqual(position.board.fullmove_number, 80)
        self.assertEqual(position.attacker, chess.WHITE)


class Summary(unittest.TestCase):
    def test_counts_horizon_and_discordant_positions(self):
        def record(ident, origin, who, reason, plies):
            return {"id": ident, "origin": origin, "attacker": who, "reason": reason, "plies": plies}

        records = [
            record("a", "drawn", "candidate", "mate", 30),
            record("a", "drawn", "baseline", "threefold", 41),
            record("b", "converted", "candidate", "mate", 12),
            record("b", "converted", "baseline", "mate", 14),
            record("c", "drawn", "candidate", "mate", 150),
            record("c", "drawn", "baseline", "cap", 200),
        ]
        summary = conversion_test.summarize(records, horizon=100)
        self.assertEqual(summary["candidate"]["horizon"], 2)
        self.assertEqual(summary["candidate"]["cap"], 3)
        self.assertEqual(summary["baseline"]["horizon"], 1)
        self.assertEqual(summary["baseline"]["cap"], 1)
        # Only "a" separates the two inside the horizon: "c" was mated after it.
        self.assertEqual((summary["candidate_only"], summary["baseline_only"]), (1, 0))
        self.assertAlmostEqual(summary["p"], 0.5)


if __name__ == "__main__":
    unittest.main()
