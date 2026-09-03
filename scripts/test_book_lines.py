#!/usr/bin/env python3
"""Tests for `scripts/book_lines.py`.

Run with `python3 scripts/test_book_lines.py`. Standard library only, so it costs nothing to run
while a match occupies the machine.

The PolyGlot key itself is checked by `book_lines.py --self-test` against the nine published
vectors; this covers everything around it - decoding a stored move, replaying it on the board,
searching the file, and the sampling rules.
"""

import os
import random
import struct
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import book_lines as bl  # noqa: E402


def encode(uci):
    """A move in PolyGlot's 16-bit encoding, as the book stores it."""
    origin, target = bl.square(uci[:2]), bl.square(uci[2:4])
    value = (target % 8) | ((target // 8) << 3) | ((origin % 8) << 6) | ((origin // 8) << 9)
    if len(uci) > 4:
        value |= {"n": 1, "b": 2, "r": 3, "q": 4}[uci[4]] << 12
    return value


def play(moves):
    position = bl.Position()
    for uci in moves:
        position.apply(encode(uci))
    return position


class Squares(unittest.TestCase):
    def test_names_and_indices_round_trip(self):
        for name in ("a1", "h1", "e4", "d5", "a8", "h8"):
            self.assertEqual(bl.square_name(bl.square(name)), name)

    def test_index_zero_is_a1(self):
        self.assertEqual(bl.square("a1"), 0)
        self.assertEqual(bl.square("h8"), 63)


class MoveDecoding(unittest.TestCase):
    def test_a_plain_move_returns_its_uci_string(self):
        position = bl.Position()
        self.assertEqual(position.apply(encode("e2e4")), "e2e4")
        self.assertEqual(position.board[bl.square("e4")], "P")
        self.assertIsNone(position.board[bl.square("e2")])

    def test_the_side_to_move_alternates(self):
        position = bl.Position()
        self.assertTrue(position.white_to_move)
        position.apply(encode("e2e4"))
        self.assertFalse(position.white_to_move)

    def test_a_move_from_an_empty_square_is_refused(self):
        # A book entry that does not match the position it was looked up under would otherwise
        # corrupt the board silently and every key after it.
        self.assertIsNone(bl.Position().apply(encode("e4e5")))


class Castling(unittest.TestCase):
    def test_the_rook_moves_too_and_the_uci_string_is_the_king_move(self):
        # PolyGlot stores a castle as the king capturing its own rook. Matt-Magie replays UCI,
        # where the same move is the king stepping two files, so the translation has to happen
        # here or every castling line in a pool is illegal.
        position = play(["e2e4", "e7e5", "g1f3", "b8c6", "f1c4", "f8c5"])
        self.assertEqual(position.apply(encode("e1h1")), "e1g1")
        self.assertEqual(position.board[bl.square("g1")], "K")
        self.assertEqual(position.board[bl.square("f1")], "R")
        self.assertIsNone(position.board[bl.square("e1")])
        self.assertIsNone(position.board[bl.square("h1")])

    def test_queenside(self):
        position = play(["d2d4", "d7d5", "b1c3", "b8c6", "c1f4", "c8f5", "d1d2", "d8d7"])
        self.assertEqual(position.apply(encode("e1a1")), "e1c1")
        self.assertEqual(position.board[bl.square("c1")], "K")
        self.assertEqual(position.board[bl.square("d1")], "R")

    def test_black_castles(self):
        position = play(["e2e4", "e7e5", "g1f3", "g8f6", "f1c4", "f8c5"])
        self.assertEqual(position.apply(encode("e8h8")), "e8g8")
        self.assertEqual(position.board[bl.square("g8")], "k")
        self.assertEqual(position.board[bl.square("f8")], "r")

    def test_moving_the_king_drops_both_rights(self):
        position = play(["e2e4", "e7e5", "e1e2"])
        self.assertFalse(position.castling["K"])
        self.assertFalse(position.castling["Q"])
        self.assertTrue(position.castling["k"])

    def test_moving_a_rook_drops_one_right(self):
        position = play(["a2a4", "a7a5", "a1a3"])
        self.assertFalse(position.castling["Q"])
        self.assertTrue(position.castling["K"])

    def test_capturing_a_rook_on_its_home_square_drops_the_right(self):
        # The right belongs to the rook, so it goes when the rook is taken and not only when it
        # moves. A key that keeps it is wrong for every position after the capture, and the book
        # lookup then misses - which looks like a book that simply ran out.
        position = bl.Position()
        position.board[bl.square("g7")] = "B"
        position.apply(encode("g7h8"))
        self.assertEqual(position.board[bl.square("h8")], "B")
        self.assertFalse(position.castling["k"], "the h8 rook is gone")
        self.assertTrue(position.castling["q"], "the a8 rook is untouched")

    def test_the_same_holds_on_the_queenside_and_for_white(self):
        black_queenside = bl.Position()
        black_queenside.board[bl.square("b7")] = "B"
        black_queenside.apply(encode("b7a8"))
        self.assertFalse(black_queenside.castling["q"])
        self.assertTrue(black_queenside.castling["k"])

        white_kingside = bl.Position()
        white_kingside.board[bl.square("g2")] = "b"
        white_kingside.white_to_move = False
        white_kingside.apply(encode("g2h1"))
        self.assertFalse(white_kingside.castling["K"])
        self.assertTrue(white_kingside.castling["Q"])


class EnPassant(unittest.TestCase):
    def test_a_double_push_sets_the_target_square(self):
        position = bl.Position()
        position.apply(encode("e2e4"))
        self.assertEqual(position.en_passant, bl.square("e3"))

    def test_any_other_move_clears_it(self):
        position = play(["e2e4", "a7a6"])
        self.assertIsNone(position.en_passant)

    def test_the_capture_removes_the_pawn_that_passed(self):
        position = play(["e2e4", "a7a6", "e4e5", "d7d5"])
        self.assertEqual(position.en_passant, bl.square("d6"))
        self.assertEqual(position.apply(encode("e5d6")), "e5d6")
        self.assertEqual(position.board[bl.square("d6")], "P")
        self.assertIsNone(position.board[bl.square("d5")],
                          "the captured pawn stays on the board")

    def test_the_key_ignores_a_target_no_pawn_can_reach(self):
        # This is the distinction the published vectors exist to catch: 1.e4 sets e3, but no
        # black pawn attacks it, so the file must not enter the key.
        after_e4 = play(["e2e4"])
        self.assertEqual(after_e4.key(), 0x823C9B50FD114196)

    def test_the_key_counts_a_target_a_pawn_can_reach(self):
        reachable = play(["a2a4", "b7b5", "h2h4", "b5b4", "c2c4"])
        self.assertEqual(reachable.key(), 0x3C8123EA7B067637)


class Promotion(unittest.TestCase):
    def test_the_piece_and_the_uci_suffix_follow_the_encoding(self):
        position = play(["a2a4", "b7b5", "a4b5", "a7a6", "b5a6", "g8f6", "a6b7", "f6g8"])
        self.assertEqual(position.board[bl.square("b7")], "P")
        self.assertEqual(position.apply(encode("b7a8q")), "b7a8q")
        self.assertEqual(position.board[bl.square("a8")], "Q")

    def test_a_black_promotion_produces_a_lower_case_piece(self):
        position = bl.Position()
        position.board[bl.square("b2")] = "p"
        position.board[bl.square("a1")] = None
        position.white_to_move = False
        self.assertEqual(position.apply(encode("b2a1n")), "b2a1n")
        self.assertEqual(position.board[bl.square("a1")], "n")


def write_book(entries):
    """A PolyGlot file from (key, move, weight) triples, sorted as the format requires."""
    handle = tempfile.NamedTemporaryFile("wb", suffix=".bin", delete=False)
    for key, move, weight in sorted(entries):
        handle.write(struct.pack(">QHHI", key, move, weight, 0))
    handle.close()
    return handle.name


class BookLookup(unittest.TestCase):
    def setUp(self):
        self.paths = []

    def tearDown(self):
        for path in self.paths:
            os.unlink(path)

    def _book(self, entries):
        path = write_book(entries)
        self.paths.append(path)
        return bl.Book(path)

    def test_every_move_stored_for_a_position_is_returned(self):
        book = self._book([
            (bl.START_KEY, encode("e2e4"), 100),
            (bl.START_KEY, encode("d2d4"), 80),
            (bl.START_KEY + 1, encode("c2c4"), 60),
        ])
        found = book.moves(bl.START_KEY)
        self.assertEqual(len(found), 2)
        self.assertEqual({weight for _, weight in found}, {100, 80})

    def test_a_position_the_book_does_not_hold_returns_nothing(self):
        book = self._book([(bl.START_KEY, encode("e2e4"), 100)])
        self.assertEqual(book.moves(bl.START_KEY + 99), [])

    def test_the_first_and_last_entries_are_reachable(self):
        # A binary search that is off by one at either end silently truncates the book.
        keys = [10, 20, 30, 40, 50]
        book = self._book([(key, encode("e2e4"), 1) for key in keys])
        for key in keys:
            self.assertEqual(len(book.moves(key)), 1, "key %d not found" % key)

    def test_a_file_that_is_not_a_multiple_of_sixteen_bytes_is_rejected(self):
        handle = tempfile.NamedTemporaryFile("wb", suffix=".bin", delete=False)
        handle.write(b"not a polyglot book")
        handle.close()
        self.paths.append(handle.name)
        self.assertFalse(bl.Book(handle.name).valid)


class Sampling(unittest.TestCase):
    def setUp(self):
        self.paths = []

    def tearDown(self):
        for path in self.paths:
            os.unlink(path)

    def _book(self, entries):
        path = write_book(entries)
        self.paths.append(path)
        return bl.Book(path)

    def test_a_line_ends_when_the_book_does(self):
        book = self._book([(bl.START_KEY, encode("e2e4"), 100)])
        self.assertIsNone(bl.sample_line(book, 2, random.Random(1), 0.0, 1))
        self.assertEqual(bl.sample_line(book, 1, random.Random(1), 0.0, 1), "e2e4")

    def test_a_weight_floor_removes_moves(self):
        book = self._book([
            (bl.START_KEY, encode("e2e4"), 100),
            (bl.START_KEY, encode("h2h4"), 1),
        ])
        drawn = {bl.sample_line(book, 1, random.Random(seed), 0.0, 50) for seed in range(20)}
        self.assertEqual(drawn, {"e2e4"})

    def test_temperature_zero_reaches_the_rare_move_and_temperature_one_rarely_does(self):
        # The whole point of the script: the engine's own sampler is the temperature-1 case, and
        # it is why a pool built from Performance.bin is seventeen openings wide.
        book = self._book([
            (bl.START_KEY, encode("e2e4"), 10000),
            (bl.START_KEY, encode("h2h4"), 1),
        ])
        uniform = [bl.sample_line(book, 1, random.Random(seed), 0.0, 1) for seed in range(200)]
        weighted = [bl.sample_line(book, 1, random.Random(seed), 1.0, 1) for seed in range(200)]
        self.assertGreater(uniform.count("h2h4"), 50)
        self.assertLess(weighted.count("h2h4"), 5)

    def test_distinct_prefixes_counts_shared_starts_once(self):
        lines = ["e2e4 e7e5 g1f3 b8c6", "e2e4 e7e5 f1c4 g8f6", "d2d4 d7d5 c2c4 e7e6"]
        self.assertEqual(bl.distinct_prefixes(lines, 2), 2)
        self.assertEqual(bl.distinct_prefixes(lines, 4), 3)

    def test_a_line_shorter_than_the_prefix_is_not_counted(self):
        self.assertEqual(bl.distinct_prefixes(["e2e4 e7e5"], 4), 0)


class RandomTable(unittest.TestCase):
    def test_the_table_comes_from_the_engine_source(self):
        # Duplicating 781 constants in this script would let it drift from the engine without
        # anything failing, and the failure would look like a book that has no entries.
        self.assertEqual(len(bl.RANDOM64), 781)
        self.assertEqual(bl.RANDOM64[0], 0x9D39247E33776D41)


if __name__ == "__main__":
    unittest.main(verbosity=2)
