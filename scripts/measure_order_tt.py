#!/usr/bin/env python3
"""Measures the move order and the Transposition Table, Matt-Magie steps 50 and 51.

The engine must be built with the `search-diag` feature, which prints a `SEARCHDIAGORDER` and a
`SEARCHDIAGTT` line after every search. The counters are process-global and cumulative; the
table's occupancy by age is a sample of every 64th slot taken at the end of each search. A build
that serves static evaluations from the entry (step 51) also reports how many it served and how
many differed from a fresh full evaluation, which must be none.

Two modes:

* `depth` - the pool to a fixed depth, `ucinewgame` before each position, as
  `scripts/measure_tree_size.py` does. The move-order figures come from here.
* `game` - self-play from the first moves of the pool at a real clock, one engine process per
  colour, so each table lives through a game the way it does in a match. The table figures come
  from here: how full it gets and what a store evicts depends on the time control.

Usage:
    scripts/measure_order_tt.py <search-diag binary> depth [--openings F] [--positions 300] [--depth 10]
    scripts/measure_order_tt.py <search-diag binary> game [--openings F] [--games 20] [--plies 60]
        [--time 1000] [--inc 150] [--hash 64]
"""
import argparse
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import uci_driver  # noqa: E402
from measure_tree_size import DEFAULT_OPENINGS, load_positions  # noqa: E402

ORDER = re.compile(r"SEARCHDIAGORDER (.*)")
TT = re.compile(r"SEARCHDIAGTT (.*)")
CLASSES = ("tt/pv", "capture", "quiet check", "killer/counter", "quiet")
INDEX = ("1", "2", "3", "4", "5-8", "9-16", "17+")
STORES = ("empty slot", "same key, replaced", "same key, refused", "other key evicted", "other key kept")


def fields(line):
    """`key=value` pairs; a value with commas is a list of integers."""
    out = {}
    for item in line.split():
        key, _, value = item.partition("=")
        out[key] = [int(v) for v in value.split(",")] if "," in value else int(value)
    return out


def last(pattern, text):
    matches = pattern.findall(text)
    return fields(matches[-1]) if matches else None


def merge(totals, counters):
    """Adds the cumulative counters of one process to the totals of all."""
    for key, value in counters.items():
        if isinstance(value, list):
            current = totals.setdefault(key, [0] * len(value))
            totals[key] = [a + b for a, b in zip(current, value)]
        else:
            totals[key] = totals.get(key, 0) + value
    return totals


def occupancy(text):
    """Mean share of empty slots and of entries by age over every search of one process."""
    rows = [fields(m) for m in TT.findall(text)]
    if not rows:
        return None
    shares = []
    for row in rows:
        total = row["sample_empty"] + sum(row["sample_age"])
        shares.append([row["sample_empty"] / total] + [a / total for a in row["sample_age"]])
    return [sum(s[i] for s in shares) / len(shares) for i in range(5)]


def pct(part, whole):
    return 100.0 * part / whole if whole else 0.0


def report_order(order):
    total = order["cutoffs"]
    print(f"  cutoffs {total:,}; on the first move {pct(order['first'], total):.1f}%")
    print("  index of the move that cut: " + ", ".join(
        f"{label} {pct(n, total):.1f}%" for label, n in zip(INDEX, order["index"])))
    print("  class of the move that cut: " + ", ".join(
        f"{label} {pct(n, total):.1f}%" for label, n in zip(CLASSES, order["class"])))
    later = total - order["first"]
    print(f"  later cutoffs after a TT move that did not cut: {pct(order['late_after_tt'], later):.1f}% of the later ones")
    rows = [(d, c, f) for d, (c, f) in enumerate(zip(order["by_depth"], order["first_by_depth"])) if c]
    print("  first-move share by remaining depth: " + ", ".join(f"{d}: {pct(f, c):.0f}%" for d, c, f in rows))


def report_tt(tt, occ):
    print(f"  probes main {tt['probe_main']:,}, key hit {pct(tt['hit_main'], tt['probe_main']):.1f}%; "
          f"QS {tt['probe_qs']:,}, hit {pct(tt['hit_qs'], tt['probe_qs']):.1f}%")
    print(f"  static evaluations at a key hit: main {pct(tt['eval_main_hit'], tt['eval_main']):.1f}% of "
          f"{tt['eval_main']:,}, QS {pct(tt['eval_qs_hit'], tt['eval_qs']):.1f}% of {tt['eval_qs']:,}")
    if "served_main" in tt:
        evaluations = tt["eval_main"] + tt["eval_qs"]
        print(f"  served from the entry: main {pct(tt['served_main'], tt['eval_main']):.1f}%, "
              f"QS {pct(tt['served_qs'], tt['eval_qs']):.1f}%, all "
              f"{pct(tt['served_main'] + tt['served_qs'], evaluations):.1f}%; "
              f"fresh evaluation differs {tt['served_mismatch']:,}")
    stores = sum(tt["store"])
    print("  stores: " + ", ".join(f"{label} {pct(n, stores):.1f}%" for label, n in zip(STORES, tt["store"])))
    evicted = tt["store"][3]
    print(f"  evictions of another key: victim deeper {pct(tt['evict_deeper'], evicted):.1f}%; victim age "
          + ", ".join(f"{a} {pct(n, evicted):.1f}%" for a, n in zip(("0", "1", "2", "3+"), tt["evict_age"]))
          + "; deeper victims by age " + ", ".join(
              f"{a} {pct(n, evicted):.1f}%" for a, n in zip(("0", "1", "2", "3+"), tt["evict_deeper_age"])))
    if occ:
        print("  mean occupancy over searches: empty {:.1f}%, age 0 {:.1f}%, 1 {:.1f}%, 2 {:.1f}%, 3+ {:.1f}%".format(
            *[100.0 * x for x in occ]))


def run_depth(binary, positions, depth):
    with uci_driver.Session(binary, options=(("Hash", "64"), ("Threads", "1"))) as session:
        for index, moves in enumerate(positions, start=1):
            session.new_game()
            session.search(moves, depth)
            if index % 50 == 0:
                print(f"  {index}/{len(positions)}", file=sys.stderr, flush=True)
    return [session.stderr]


def run_games(binary, openings, plies, base, inc, hash_mb):
    texts = []
    for number, opening in enumerate(openings, start=1):
        sessions = [uci_driver.Session(binary, options=(("Hash", str(hash_mb)), ("Threads", "1"))) for _ in range(2)]
        try:
            for s in sessions:
                s.new_game()
            moves = list(opening)
            clock = [float(base), float(base)]
            for _ in range(plies):
                side = len(moves) % 2
                best, used = sessions[side].go(moves, f"wtime {int(clock[0])} btime {int(clock[1])} winc {inc} binc {inc}")
                if not best or best == "0000":
                    break
                clock[side] = max(clock[side] - used, 1.0) + inc
                moves.append(best)
        finally:
            for s in sessions:
                s.close()
                texts.append(s.stderr)
        print(f"  game {number}/{len(openings)}: {len(moves)} plies", file=sys.stderr, flush=True)
    return texts


def summarize(texts):
    order, tt = {}, {}
    for text in texts:
        o, t = last(ORDER, text), last(TT, text)
        if o is None or t is None:
            raise SystemExit("no SEARCHDIAGORDER / SEARCHDIAGTT line -- is it a search-diag build?")
        merge(order, {k: v for k, v in o.items()})
        merge(tt, {k: v for k, v in t.items() if not k.startswith("sample")})
    occs = [occupancy(text) for text in texts]
    occs = [o for o in occs if o]
    occ = [sum(o[i] for o in occs) / len(occs) for i in range(5)] if occs else None
    return order, tt, occ


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("binary")
    parser.add_argument("mode", choices=("depth", "game"))
    parser.add_argument("--openings", default=DEFAULT_OPENINGS)
    parser.add_argument("--positions", type=int, default=300)
    parser.add_argument("--depth", type=int, default=10)
    parser.add_argument("--games", type=int, default=20)
    parser.add_argument("--plies", type=int, default=60)
    parser.add_argument("--time", type=int, default=1000)
    parser.add_argument("--inc", type=int, default=150)
    parser.add_argument("--hash", type=int, default=64)
    args = parser.parse_args()
    if args.mode == "depth":
        texts = run_depth(args.binary, load_positions(args.openings, args.positions), args.depth)
        title = f"{os.path.basename(args.openings)}, {args.positions} positions, depth {args.depth}"
    else:
        texts = run_games(args.binary, load_positions(args.openings, args.games), args.plies,
                          args.time, args.inc, args.hash)
        title = (f"{os.path.basename(args.openings)}, {args.games} games x {args.plies} plies, "
                 f"{args.time}ms + {args.inc}ms, Hash {args.hash}")
    order, tt, occ = summarize(texts)
    print(title)
    print("move order")
    report_order(order)
    print("transposition table")
    report_tt(tt, occ)


if __name__ == "__main__":
    main()
