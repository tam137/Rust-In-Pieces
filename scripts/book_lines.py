#!/usr/bin/env python3
"""Walk a PolyGlot book directly and build opening pools from it.

`scripts/make_opening_lines.py` samples lines by asking the engine for a book move. That is a
good end-to-end check of the PolyGlot key, and it inherits the one property of the book that
makes it a poor sampler: `get_random_book_move` picks **weighted by popularity**, and popularity
is concentrated. Six hundred lines drawn that way from `books/Performance.bin` contain only
seventeen distinct four-ply starts.

This reads the book file itself, so it needs no engine, no CPU worth mentioning, and it can
choose moves **uniformly** among the ones the book offers instead of by weight. Uniform choice at
every ply is what produces breadth at the root, which is the thing the engine's own sampler
cannot give.

It also makes every book in `books/` usable, not just the one compiled into the binary.

    scripts/book_lines.py --survey                       # what each book can offer
    scripts/book_lines.py --book books/komodo.bin --plies 10 --count 1200 \\
        --out openings/book_komodo_10ply.txt

The move generator is the book: every position is looked up by its PolyGlot key, and the moves
stored there are the only ones followed. Applying them needs no legality check, only the rules
for castling, en passant and promotion that the format encodes.

The Zobrist table is read out of `src/polyglot.rs`, so there is one copy of it in the repository
and this script cannot drift from the engine. `--self-test` checks the key against the nine
published vectors before trusting anything else.
"""

import argparse
import os
import random
import re
import sys

REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), os.pardir))

START_KEY = 0x463B96181691FC9C

# PolyGlot's piece order: black pawn 0, white pawn 1, black knight 2, ... white king 11.
PIECE_INDEX = {"p": 0, "P": 1, "n": 2, "N": 3, "b": 4, "B": 5,
               "r": 6, "R": 7, "q": 8, "Q": 9, "k": 10, "K": 11}
PROMOTION_PIECE = {1: "n", 2: "b", 3: "r", 4: "q"}

# Castling is stored as the king capturing its own rook, which is also how Chess960 encodes it.
CASTLE_MOVES = {
    ("e1", "h1"): ("e1", "g1", "h1", "f1", "K"),
    ("e1", "a1"): ("e1", "c1", "a1", "d1", "Q"),
    ("e8", "h8"): ("e8", "g8", "h8", "f8", "k"),
    ("e8", "a8"): ("e8", "c8", "a8", "d8", "q"),
}

START_RANKS = "RNBQKBNR"


def load_random64():
    """The 781 PolyGlot constants, read out of the engine's own source."""
    path = os.path.join(REPO_ROOT, "src", "polyglot.rs")
    with open(path, "r", encoding="utf-8") as handle:
        text = handle.read()
    start = text.index("POLYGLOT_RANDOM64: [u64; 781] = [")
    end = text.index("];", start)
    values = [int(token, 16) for token in re.findall(r"0x([0-9A-Fa-f]{16})", text[start:end])]
    if len(values) != 781:
        sys.exit("expected 781 PolyGlot constants in src/polyglot.rs, found %d" % len(values))
    return values


RANDOM64 = load_random64()


def square(name):
    return (int(name[1]) - 1) * 8 + (ord(name[0]) - ord("a"))


def square_name(index):
    return "%s%d" % (chr(ord("a") + index % 8), index // 8 + 1)


class Position:
    """Just enough of a board to compute PolyGlot keys and replay book moves."""

    def __init__(self):
        self.board = [None] * 64
        for file_index, piece in enumerate(START_RANKS):
            self.board[square("%s1" % chr(ord("a") + file_index))] = piece
            self.board[square("%s8" % chr(ord("a") + file_index))] = piece.lower()
            self.board[square("%s2" % chr(ord("a") + file_index))] = "P"
            self.board[square("%s7" % chr(ord("a") + file_index))] = "p"
        self.white_to_move = True
        self.castling = {"K": True, "Q": True, "k": True, "q": True}
        self.en_passant = None          # target square index, or None

    def clone(self):
        copy = Position.__new__(Position)
        copy.board = self.board[:]
        copy.white_to_move = self.white_to_move
        copy.castling = dict(self.castling)
        copy.en_passant = self.en_passant
        return copy

    def key(self):
        value = 0
        for index, piece in enumerate(self.board):
            if piece is not None:
                value ^= RANDOM64[64 * PIECE_INDEX[piece] + index]
        for offset, right in enumerate("KQkq"):
            if self.castling[right]:
                value ^= RANDOM64[768 + offset]
        # The en passant file counts only when a pawn of the side to move could actually take,
        # which is the distinction a naive implementation gets wrong and the published vectors
        # are built to catch.
        if self.en_passant is not None:
            file_index = self.en_passant % 8
            rank = self.en_passant // 8
            attacker, source_rank = ("P", rank - 1) if self.white_to_move else ("p", rank + 1)
            for neighbour in (file_index - 1, file_index + 1):
                if 0 <= neighbour < 8 and self.board[source_rank * 8 + neighbour] == attacker:
                    value ^= RANDOM64[772 + file_index]
                    break
        if self.white_to_move:
            value ^= RANDOM64[780]
        return value

    def apply(self, encoded):
        """Plays one encoded book move and returns its UCI string."""
        to_index = (encoded & 7) | (((encoded >> 3) & 7) << 3)
        from_index = ((encoded >> 6) & 7) | (((encoded >> 9) & 7) << 3)
        promotion = (encoded >> 12) & 7

        origin, target = square_name(from_index), square_name(to_index)
        piece = self.board[from_index]
        if piece is None:
            return None

        castle = CASTLE_MOVES.get((origin, target))
        if castle and piece in "Kk":
            king_from, king_to, rook_from, rook_to, right = castle
            self.board[square(king_from)] = None
            self.board[square(rook_from)] = None
            self.board[square(king_to)] = piece
            self.board[square(rook_to)] = "R" if piece == "K" else "r"
            self._drop_castling_for_king(piece)
            self.en_passant = None
            self.white_to_move = not self.white_to_move
            return king_from + king_to

        captured_en_passant = (piece in "Pp" and to_index == self.en_passant
                               and self.board[to_index] is None)

        self.board[from_index] = None
        if captured_en_passant:
            self.board[to_index + (-8 if piece == "P" else 8)] = None
        if promotion:
            replacement = PROMOTION_PIECE[promotion]
            self.board[to_index] = replacement.upper() if piece == "P" else replacement
        else:
            self.board[to_index] = piece

        # A double pawn push is the only move that sets an en passant target.
        if piece in "Pp" and abs(to_index - from_index) == 16:
            self.en_passant = (from_index + to_index) // 2
        else:
            self.en_passant = None

        if piece in "Kk":
            self._drop_castling_for_king(piece)
        for index in (from_index, to_index):
            corner = square_name(index)
            if corner == "a1":
                self.castling["Q"] = False
            elif corner == "h1":
                self.castling["K"] = False
            elif corner == "a8":
                self.castling["q"] = False
            elif corner == "h8":
                self.castling["k"] = False

        self.white_to_move = not self.white_to_move
        return origin + target + (PROMOTION_PIECE[promotion] if promotion else "")

    def _drop_castling_for_king(self, piece):
        if piece == "K":
            self.castling["K"] = self.castling["Q"] = False
        else:
            self.castling["k"] = self.castling["q"] = False


class Book:
    """A PolyGlot book held as raw bytes, searched in place."""

    ENTRY = 16

    def __init__(self, path):
        with open(path, "rb") as handle:
            self.data = handle.read()
        self.valid = len(self.data) % self.ENTRY == 0 and len(self.data) >= self.ENTRY
        self.count = len(self.data) // self.ENTRY
        self.path = path

    def _key_at(self, index):
        offset = index * self.ENTRY
        return int.from_bytes(self.data[offset:offset + 8], "big")

    def moves(self, key):
        """(encoded move, weight) for a position, in the order the book stores them."""
        low, high = 0, self.count
        while low < high:
            middle = (low + high) // 2
            if self._key_at(middle) < key:
                low = middle + 1
            else:
                high = middle
        found = []
        while low < self.count and self._key_at(low) == key:
            offset = low * self.ENTRY
            encoded = int.from_bytes(self.data[offset + 8:offset + 10], "big")
            weight = int.from_bytes(self.data[offset + 10:offset + 12], "big")
            found.append((encoded, weight))
            low += 1
        return found

    def unique_keys(self):
        seen = 0
        previous = None
        for index in range(self.count):
            key = self._key_at(index)
            if key != previous:
                seen += 1
                previous = key
        return seen


def sample_line(book, plies, rng, temperature, min_weight):
    """One line of `plies` book moves, or None when the book runs out early.

    `temperature` interpolates between the two ways of reading a book. At 0 every move the book
    offers is equally likely, which is what produces breadth and also makes 1.g3 as frequent as
    1.e4. At 1 moves are picked in proportion to their weight, which is what the engine's own
    `get_random_book_move` does and what leaves the pool seventeen openings wide. Values in
    between pick proportional to weight raised to that power, so a mainline stays commoner than a
    sideline without swamping it.
    """
    position = Position()
    moves = []
    for _ in range(plies):
        options = [(encoded, weight) for encoded, weight in book.moves(position.key())
                   if weight >= min_weight]
        if not options:
            return None
        if temperature <= 0.0:
            encoded, _ = rng.choice(options)
        else:
            weights = [max(1.0, float(weight)) ** temperature for _, weight in options]
            encoded = rng.choices([option for option, _ in options], weights=weights)[0]
        played = position.apply(encoded)
        if played is None:
            return None
        moves.append(played)
    return " ".join(moves)


def distinct_prefixes(lines, plies):
    return len({" ".join(line.split()[:plies]) for line in lines
                if len(line.split()) >= plies})


def survey(paths, plies, samples, min_weight, seed, temperature):
    print("%-24s %9s %9s %6s   %s" % (
        "book", "entries", "positions", "root",
        "distinct prefixes of %d samples at temperature %g" % (samples, temperature)))
    print("%-24s %9s %9s %6s   %s" % ("", "", "", "moves",
                                      "  ".join("@%d" % p for p in (4, 6, 8, plies))))
    for path in paths:
        book = Book(path)
        if not book.valid:
            print("%-24s %9s   not a PolyGlot book (size is not a multiple of 16 bytes)"
                  % (os.path.basename(path), len(book.data)))
            continue
        root = book.moves(START_KEY)
        if not root:
            print("%-24s %9d %9s %6s   the initial position is not in this book"
                  % (os.path.basename(path), book.count, "?", "-"))
            continue

        rng = random.Random(seed)
        lines = set()
        for _ in range(samples * 3):
            if len(lines) >= samples:
                break
            line = sample_line(book, plies, rng, temperature, min_weight)
            if line:
                lines.add(line)
        lines = sorted(lines)
        print("%-24s %9d %9d %6d   %s" % (
            os.path.basename(path), book.count, book.unique_keys(), len(root),
            "  ".join("%d" % distinct_prefixes(lines, p) for p in (4, 6, 8, plies))
            + ("   (%d lines)" % len(lines))))


def self_test():
    """The nine published vectors, plus the two that separate a real en passant from a set square."""
    vectors = [
        ("start", [], 0x463B96181691FC9C),
        ("1.e4", ["e2e4"], 0x823C9B50FD114196),
        ("1.e4 d5", ["e2e4", "d7d5"], 0x0756B94461C50FB0),
        ("1.e4 d5 2.e5", ["e2e4", "d7d5", "e4e5"], 0x662FAFB965DB29D4),
        ("1.e4 d5 2.e5 f5", ["e2e4", "d7d5", "e4e5", "f7f5"], 0x22A48B5A8E47FF78),
        ("...3.Ke2", ["e2e4", "d7d5", "e4e5", "f7f5", "e1e2"], 0x652A607CA3F242C1),
        ("...3...Kf7", ["e2e4", "d7d5", "e4e5", "f7f5", "e1e2", "e8f7"], 0x00FDD303C946BDD9),
        ("1.a4 b5 2.h4 b4 3.c4", ["a2a4", "b7b5", "h2h4", "b5b4", "c2c4"], 0x3C8123EA7B067637),
        ("...3...bxc3 4.Ra3", ["a2a4", "b7b5", "h2h4", "b5b4", "c2c4", "b4c3", "a1a3"],
         0x5C3F9B829B279560),
    ]

    def encode(uci):
        origin, target = square(uci[:2]), square(uci[2:4])
        value = (target % 8) | ((target // 8) << 3) | ((origin % 8) << 6) | ((origin // 8) << 9)
        if len(uci) > 4:
            value |= {"n": 1, "b": 2, "r": 3, "q": 4}[uci[4]] << 12
        return value

    failures = 0
    for name, moves, expected in vectors:
        position = Position()
        for uci in moves:
            position.apply(encode(uci))
        actual = position.key()
        status = "ok" if actual == expected else "FAIL"
        if actual != expected:
            failures += 1
        print("  %-24s %016x  %s" % (name, actual, status))

    # Castling has no published vector, so the board it produces is asserted instead. PolyGlot
    # always stores a castle as the king capturing its own rook, so "e1g1" is a plain king move
    # in this format and must not be treated as one.
    played = Position()
    for uci in ["e2e4", "e7e5", "g1f3", "b8c6", "f1c4", "f8c5"]:
        played.apply(encode(uci))

    castled = played.clone()
    if castled.apply(encode("e1h1")) != "e1g1":
        print("  %-24s  FAIL (wrong UCI string)" % "castling")
        failures += 1
    elif (castled.board[square("g1")], castled.board[square("f1")],
          castled.board[square("e1")], castled.board[square("h1")]) != ("K", "R", None, None):
        print("  %-24s  FAIL (wrong board)" % "castling")
        failures += 1
    elif castled.castling["K"] or castled.castling["Q"]:
        print("  %-24s  FAIL (rights kept)" % "castling")
        failures += 1
    else:
        print("  %-24s %016x  ok" % ("castling e1h1 -> e1g1", castled.key()))

    # The same square pair with a piece that is not a king is an ordinary move.
    rook_move = Position()
    for uci in ["a2a4", "a7a5", "a1a3"]:
        rook_move.apply(encode(uci))
    if rook_move.board[square("a3")] != "R" or rook_move.castling["Q"]:
        print("  %-24s  FAIL" % "rook move drops one right")
        failures += 1
    else:
        print("  %-24s %016x  ok" % ("rook move, a-side right", rook_move.key()))

    print("\n%s" % ("all key vectors pass" if not failures else "%d FAILURES" % failures))
    return 1 if failures else 0


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--book", help="book file, relative to the repository root")
    parser.add_argument("--survey", action="store_true",
                        help="compare every book in books/ for breadth")
    parser.add_argument("--self-test", action="store_true",
                        help="check the PolyGlot key against the published vectors")
    parser.add_argument("--plies", type=int, default=10,
                        help="line length in half-moves; keep it even (default 10)")
    parser.add_argument("--count", type=int, default=1000, help="lines to collect")
    parser.add_argument("--min-weight", type=int, default=1,
                        help="ignore book moves below this weight (default 1)")
    parser.add_argument("--temperature", type=float, default=0.0,
                        help="0 picks uniformly among the book's moves and gives the broadest "
                             "pool; 1 picks by weight, as the engine's own sampler does, and "
                             "gives the narrowest. Values between trade the two (default 0)")
    parser.add_argument("--seed", type=int, default=137, help="random seed")
    parser.add_argument("--out", help="output file, relative to the repository root")
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    if args.survey:
        books = sorted(os.path.join(REPO_ROOT, "books", name)
                       for name in os.listdir(os.path.join(REPO_ROOT, "books"))
                       if name.endswith(".bin"))
        survey(books, args.plies, min(args.count, 400), args.min_weight, args.seed,
               args.temperature)
        return 0

    if not args.book:
        parser.error("give --book, --survey or --self-test")
    if args.plies % 2:
        print("warning: an odd ply count hands over with Black to move, unlike every earlier "
              "measurement in task.md", file=sys.stderr)

    book = Book(os.path.join(REPO_ROOT, args.book))
    if not book.valid:
        sys.exit("%s is not a PolyGlot book: its size is not a multiple of 16 bytes" % args.book)
    rng = random.Random(args.seed)
    lines = set()
    attempts = 0
    while len(lines) < args.count and attempts < args.count * 60:
        attempts += 1
        line = sample_line(book, args.plies, rng, args.temperature, args.min_weight)
        if line:
            lines.add(line)

    if not lines:
        sys.exit("no lines of %d plies could be drawn from %s" % (args.plies, args.book))

    ordered = sorted(lines)
    rng.shuffle(ordered)

    if args.out:
        out_path = os.path.join(REPO_ROOT, args.out)
        os.makedirs(os.path.dirname(out_path), exist_ok=True)
        with open(out_path, "w") as handle:
            handle.write("\n".join(ordered) + "\n")
        print("wrote %d lines of %d plies to %s" % (len(ordered), args.plies, args.out))
    else:
        print("\n".join(ordered))

    print("distinct prefixes:  %s" % "  ".join(
        "@%d %d" % (p, distinct_prefixes(ordered, p)) for p in (2, 4, 6, 8, args.plies)),
        file=sys.stderr)
    print("%d samples drawn, %d unique" % (attempts, len(ordered)), file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
