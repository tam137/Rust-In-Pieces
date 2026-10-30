#!/usr/bin/env python3
"""Replays the games of a Matt-Magie PGN as one engine lived them and reports how full and how old
its Transposition Table is, move by move (Matt-Magie plan step 81).

For each chosen game the engine searches only the positions it had to move in, in one process,
with `ucinewgame` only between games, so the table carries over from move to move as it did in the
game. The clock is rebuilt: `--start` ms at the first own move, `--inc` ms added after each, the
time the search really took subtracted. The opening plies that came from the openings file were not
searched by the engine and are skipped.

The binary has to be a `search-diag` build: after every search it prints `SEARCHDIAGTT` with the
table counters (cumulative) and an occupancy sample over every 64th slot by the age of its entry, in
searches (0 = this search, 1, 2, 3 and more).

Usage:
    scripts/replay_games_tt.py <search-diag binary> <pgn> --engine "RIP V0.49.7-NNUE" \
        [--games 20] [--hash 64] [--openings openings_wide.txt] [--start 1000] [--inc 100]
"""
import argparse
import re
import subprocess
import sys
import threading
import time

import chess


def parse_games(path):
    games, headers, moves = [], {}, []
    for line in open(path):
        line = line.strip()
        if line.startswith("["):
            if moves:
                games.append((headers, " ".join(moves)))
                headers, moves = {}, []
            key, value = re.match(r'\[(\w+) "(.*)"\]', line).groups()
            headers[key] = value
        elif line:
            moves.append(line)
    if moves:
        games.append((headers, " ".join(moves)))
    return games


def to_uci(board, token):
    """Matt-Magie writes `e2e4`, `Ng1f3`, `0-0`, and promotions as `d8=Q`."""
    plain = token[1:] if token[0] in "NBRQK" and len(token) >= 5 else token
    try:
        move = chess.Move.from_uci(plain.lower())
        if move in board.legal_moves:
            return move
    except ValueError:
        pass
    return board.parse_san(token.replace("0", "O"))


def game_moves(text):
    tokens = [t for t in text.split() if not re.match(r"^\d+\.$", t) and t not in ("1-0", "0-1", "1/2-1/2", "*")]
    board, moves = chess.Board(), []
    for token in tokens:
        move = to_uci(board, token)
        board.push(move)
        moves.append(move.uci())
    return moves


def opening_length(moves, openings):
    best = 0
    for line in openings:
        if len(line) > best and moves[:len(line)] == line:
            best = len(line)
    return best


class Engine:
    def __init__(self, binary, hash_mb):
        self.proc = subprocess.Popen([binary], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                     stderr=subprocess.PIPE, text=True, bufsize=1)
        self.diag = []
        self.lock = threading.Lock()
        threading.Thread(target=self._read_stderr, daemon=True).start()
        self.send("uci")
        self.wait("uciok")
        for name, value in (("Hash", hash_mb), ("Threads", 1), ("OwnBook", "false")):
            self.send(f"setoption name {name} value {value}")
        self.send("isready")
        self.wait("readyok")

    def _read_stderr(self):
        for line in self.proc.stderr:
            if line.startswith("SEARCHDIAGTT "):
                with self.lock:
                    self.diag.append(line.split()[1:])

    def send(self, command):
        self.proc.stdin.write(command + "\n")
        self.proc.stdin.flush()

    def wait(self, token):
        for line in self.proc.stdout:
            if line.startswith(token):
                return line

    def search(self, moves, clock, inc):
        with self.lock:
            seen = len(self.diag)
        self.send("position startpos moves " + " ".join(moves) if moves else "position startpos")
        start = time.monotonic()
        self.send(f"go wtime {clock} btime {clock} winc {inc} binc {inc}")
        self.wait("bestmove")
        spent = int((time.monotonic() - start) * 1000)
        # One SEARCHDIAGTT per iteration; the last one before `bestmove` is the finished search.
        # stderr is not ordered with stdout, so wait until it has been quiet for a moment.
        # A forced move (one legal reply) or a book move comes back without a search and without a
        # line: then there is nothing to record for this move.
        deadline, last_count, quiet_since = time.monotonic() + 0.5, -1, time.monotonic()
        while time.monotonic() < deadline:
            with self.lock:
                count = len(self.diag)
            if count != last_count:
                last_count, quiet_since = count, time.monotonic()
            elif count > seen and time.monotonic() - quiet_since > 0.03:
                with self.lock:
                    return spent, dict(f.split("=", 1) for f in self.diag[-1])
            time.sleep(0.005)
        return spent, None

    def close(self):
        self.send("quit")
        self.proc.wait(timeout=10)


def ints(value):
    return [int(v) for v in value.split(",")]


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("binary")
    parser.add_argument("pgn")
    parser.add_argument("--engine", required=True, help="the White/Black name of the engine in the PGN")
    parser.add_argument("--games", type=int, default=20)
    parser.add_argument("--hash", type=int, default=64)
    parser.add_argument("--openings", default="openings_wide.txt")
    parser.add_argument("--start", type=int, default=1000)
    parser.add_argument("--inc", type=int, default=100)
    args = parser.parse_args()

    openings = [line.split() for line in open(args.openings) if line.split()]
    chosen = [(h, t) for h, t in parse_games(args.pgn) if args.engine in (h.get("White"), h.get("Black"))][:args.games]
    engine = Engine(args.binary, args.hash)
    rows = []  # (own move number, occupied share, age shares, stores, evictions, deeper evictions)
    for game_no, (headers, text) in enumerate(chosen, 1):
        moves = game_moves(text)
        white = headers["White"] == args.engine
        skip = opening_length(moves, openings)
        engine.send("ucinewgame")
        engine.send("isready")
        engine.wait("readyok")
        clock, own, previous = args.start, 0, None
        unsearched = 0
        for ply in range(skip, len(moves)):
            if (ply % 2 == 0) != white:
                continue
            spent, diag = engine.search(moves[:ply], max(clock, 1), args.inc)
            clock = clock - spent + args.inc
            own += 1
            if diag is None:
                unsearched += 1
                continue
            store = ints(diag["store"])
            sample = ints(diag["sample_age"])
            empty = int(diag["sample_empty"])
            total = empty + sum(sample)
            if previous is not None:
                stores = [a - b for a, b in zip(store, previous["store"])]
                evict_deeper = int(diag["evict_deeper"]) - previous["evict_deeper"]
                rows.append((own, 1 - empty / total, [s / total for s in sample], sum(stores), stores, evict_deeper))
            previous = {"store": store, "evict_deeper": int(diag["evict_deeper"])}
        print(f"game {game_no:2d}: {headers['White']} - {headers['Black']}, {own} own moves searched "
              f"(opening {skip} plies skipped, {unsearched} answered without a search)", file=sys.stderr)
    engine.close()

    print(f"\n{args.engine}, Hash={args.hash} MB, {len(chosen)} games, clock {args.start} ms + {args.inc} ms")
    print("own moves | occupied | this search | 1 ago | 2 ago | 3+ ago | stores/search | evicted other key | of those deeper")
    bins = [(1, 10), (11, 20), (21, 30), (31, 40), (41, 60), (61, 999)]
    for lo, hi in bins:
        part = [r for r in rows if lo <= r[0] <= hi]
        if not part:
            continue
        n = len(part)
        occ = sum(r[1] for r in part) / n
        ages = [sum(r[2][k] for r in part) / n for k in range(4)]
        stores = sum(r[3] for r in part) / n
        evict = sum(r[4][3] for r in part) / n
        deeper = sum(r[5] for r in part) / n
        label = f"{lo}-{hi}" if hi < 999 else f"{lo}+"
        print(f"{label:>9} | {occ:7.1%} | {ages[0]:10.1%} | {ages[1]:5.1%} | {ages[2]:5.1%} | {ages[3]:6.1%} | "
              f"{stores:13,.0f} | {evict:17,.0f} | {deeper:15,.0f}   (n={n})")


if __name__ == "__main__":
    main()
