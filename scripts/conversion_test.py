#!/usr/bin/env python3
"""Plays won endings out: does a binary convert a queen or rook against a bare king?

`task.md` rule 1 prices search changes in games, and a sequential test is the gate for anything
that moves the tree. Some changes act only in a corner of the game that a match visits too rarely
to resolve. The stop rule of iterative deepening changes nothing until a mate score reaches the
root: in the R18 run 30% of the games reached a queen or rook against a bare king, and 6.9% of
those were then drawn. A match between two binaries prices that at a few Elo at most - out of
reach of a sequential test - while the positions themselves answer the question directly.

Each position of an EPD file is played twice, by the candidate and by the baseline as the side
with the material, the baseline defending in both games. Clocks start fresh at the given time
control; the hash tables are kept between moves and cleared between games, as in a real game. A
game ends at mate, stalemate, threefold repetition, insufficient material, a flag, or the ply
cap. What comes back is the number of positions each binary mated in - inside the fifty-move
horizon (100 plies) and inside the cap - and an exact one-sided McNemar test on the positions
where the two disagree inside the horizon.

    python3 scripts/conversion_test.py <baseline> <candidate> [--workers 5] [--out games.jsonl]
"""

import argparse
import json
import math
import os
import queue
import sys
import threading
import time
from collections import namedtuple

import chess
import chess.engine

DEFAULT_POSITIONS = os.path.join(
    os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "openings", "bare_king_endings.epd")

Position = namedtuple("Position", "board ident origin attacker")


def attacker_of(board):
    """The side with a queen or rook when the other side has nothing but its king, else None."""
    for side in (chess.WHITE, chess.BLACK):
        bare = chess.popcount(board.occupied_co[not side]) == 1
        if bare and (board.pieces(chess.QUEEN, side) or board.pieces(chess.ROOK, side)):
            return side
    return None


def load_positions(path):
    positions = []
    with open(path) as handle:
        for line in handle:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            board, operations = chess.Board.from_epd(line)
            positions.append(Position(board, operations.get("id", str(len(positions) + 1)),
                                      operations.get("c0", ""), attacker_of(board)))
    return positions


def game_over(board, plies, cap):
    """Why the game is over after `plies` half-moves from the start position, or None."""
    if board.is_checkmate():
        return "mate"
    if board.is_stalemate():
        return "stalemate"
    if board.is_insufficient_material():
        return "insufficient"
    if board.is_repetition(3):
        return "threefold"
    if plies >= cap:
        return "cap"
    return None


def mcnemar_one_sided(candidate_only, baseline_only):
    """P(at least `candidate_only` of the discordant positions go the candidate's way) under H0."""
    total = candidate_only + baseline_only
    if total == 0:
        return 1.0
    tail = sum(math.comb(total, k) for k in range(candidate_only, total + 1))
    return tail / 2 ** total


def summarize(records, horizon):
    """Per attacker: mates inside the horizon and inside the cap, flags; the discordant pairs."""
    summary = {}
    mated = {}
    for who in ("candidate", "baseline"):
        mine = [r for r in records if r["attacker"] == who]
        summary[who] = {
            "games": len(mine),
            "horizon": sum(1 for r in mine if r["reason"] == "mate" and r["plies"] <= horizon),
            "cap": sum(1 for r in mine if r["reason"] == "mate"),
            "flags": sum(1 for r in mine if r["reason"] == "flag"),
            "errors": sum(1 for r in mine if r["reason"] == "error"),
        }
        for origin in sorted({r["origin"] for r in records}):
            subset = [r for r in mine if r["origin"] == origin]
            summary[who][origin] = (
                sum(1 for r in subset if r["reason"] == "mate" and r["plies"] <= horizon),
                len(subset))
        mated[who] = {r["id"]: r["reason"] == "mate" and r["plies"] <= horizon for r in mine}
    both = set(mated["candidate"]) & set(mated["baseline"])
    summary["candidate_only"] = sum(1 for i in both if mated["candidate"][i] and not mated["baseline"][i])
    summary["baseline_only"] = sum(1 for i in both if mated["baseline"][i] and not mated["candidate"][i])
    summary["p"] = mcnemar_one_sided(summary["candidate_only"], summary["baseline_only"])
    return summary


def open_engine(binary, hash_mb):
    engine = chess.engine.SimpleEngine.popen_uci(binary)
    wanted = {"Hash": hash_mb, "Threads": 1, "OwnBook": False}
    engine.configure({name: value for name, value in wanted.items() if name in engine.options})
    return engine


def play_game(position, attacker, defender, args, game_key):
    """Plays one game from `position`; `attacker` moves for the side with the material."""
    board = position.board.copy(stack=False)
    clocks = {chess.WHITE: args.time, chess.BLACK: args.time}
    plies = 0
    while True:
        reason = game_over(board, plies, args.cap)
        if reason:
            return reason, plies
        engine = attacker if board.turn == position.attacker else defender
        limit = chess.engine.Limit(white_clock=clocks[chess.WHITE], black_clock=clocks[chess.BLACK],
                                   white_inc=args.inc, black_inc=args.inc)
        started = time.monotonic()
        result = engine.play(board, limit, game=game_key)
        clocks[board.turn] -= time.monotonic() - started
        if clocks[board.turn] < 0:
            return ("flag" if board.turn == position.attacker else "defender flag"), plies
        clocks[board.turn] += args.inc
        if result.move is None or result.move not in board.legal_moves:
            return "error", plies
        board.push(result.move)
        plies += 1


def worker(jobs, records, lock, args, out):
    engines = {
        "candidate": open_engine(args.candidate, args.hash),
        "baseline": open_engine(args.baseline, args.hash),
        "defender": open_engine(args.baseline, args.hash),
    }
    try:
        while True:
            try:
                index, position = jobs.get_nowait()
            except queue.Empty:
                return
            # Alternate which binary goes first, so a host that slows down meets both equally.
            order = ("candidate", "baseline") if index % 2 == 0 else ("baseline", "candidate")
            for who in order:
                try:
                    reason, plies = play_game(position, engines[who], engines["defender"], args,
                                              (position.ident, who))
                except chess.engine.EngineError:
                    reason, plies = "error", 0
                record = {"id": position.ident, "origin": position.origin, "attacker": who,
                          "reason": reason, "plies": plies}
                with lock:
                    records.append(record)
                    if out:
                        out.write(json.dumps(record) + "\n")
                        out.flush()
            with lock:
                done = len(records) // 2
                if done % 25 == 0:
                    print(f"  {done}/{args.total} positions", flush=True)
    finally:
        for engine in engines.values():
            engine.quit()


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("baseline")
    parser.add_argument("candidate")
    parser.add_argument("--positions", default=DEFAULT_POSITIONS)
    parser.add_argument("--limit", type=int, default=0, help="play only the first N positions")
    parser.add_argument("--time", type=float, default=1.0, help="seconds on each clock")
    parser.add_argument("--inc", type=float, default=0.15, help="increment in seconds")
    parser.add_argument("--cap", type=int, default=200, help="plies before the game is drawn")
    parser.add_argument("--horizon", type=int, default=100, help="plies the primary count allows")
    parser.add_argument("--hash", type=int, default=64)
    parser.add_argument("--workers", type=int, default=5)
    parser.add_argument("--out", help="JSON lines, one record per game")
    args = parser.parse_args()

    positions = [p for p in load_positions(args.positions) if p.attacker is not None]
    if args.limit:
        positions = positions[:args.limit]
    args.total = len(positions)
    names = {}
    for who, binary in (("baseline", args.baseline), ("candidate", args.candidate)):
        engine = open_engine(binary, args.hash)
        names[who] = engine.id.get("name", binary)
        engine.quit()
    print(f"{len(positions)} positions from {os.path.basename(args.positions)}, "
          f"{args.time:g}s + {args.inc * 1000:g}ms, cap {args.cap} plies, horizon {args.horizon}")
    print(f"candidate {names['candidate']}, baseline and defender {names['baseline']}\n")

    jobs = queue.Queue()
    for index, position in enumerate(positions):
        jobs.put((index, position))
    records, lock = [], threading.Lock()
    out = open(args.out, "w") if args.out else None
    try:
        threads = [threading.Thread(target=worker, args=(jobs, records, lock, args, out))
                   for _ in range(max(1, args.workers))]
        for thread in threads:
            thread.start()
        for thread in threads:
            thread.join()
    finally:
        if out:
            out.close()

    summary = summarize(records, args.horizon)
    print()
    print(f"             mated <= {args.horizon} plies   mated <= {args.cap} plies   flags   errors")
    for who in ("candidate", "baseline"):
        s = summary[who]
        print(f"  {who:10s} {s['horizon']:5d} / {s['games']:<5d}       {s['cap']:5d}          "
              f"{s['flags']:5d}   {s['errors']:5d}")
    origins = sorted({r["origin"] for r in records})
    for origin in origins:
        cand, base = summary["candidate"][origin], summary["baseline"][origin]
        print(f"  from {origin or 'untagged'} games: candidate {cand[0]}/{cand[1]}, "
              f"baseline {base[0]}/{base[1]}")
    print(f"\n  inside the horizon only the candidate mated in {summary['candidate_only']}, "
          f"only the baseline in {summary['baseline_only']}")
    print(f"  exact one-sided McNemar p = {summary['p']:.3g}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
