#!/usr/bin/env python3
"""Sample opening lines from the engine's PolyGlot book for paired matchplay.

Every line is a space separated list of UCI moves that the book actually contains. Matt-Magie
plays one line twice, once with each colour assignment, which removes the strength of the
opening from a comparison instead of leaving it as noise: `task.md` 2.2.6 records three matches
that disagree by 41 Elo purely because each of them sampled whichever openings the clock
happened to produce.

The sampler drives the engine over UCI rather than parsing the book itself. That keeps it free
of dependencies and makes it an end-to-end check of the PolyGlot key at the same time: a wrong
key produces no book moves at all.

A book move is returned instantly and without any `info` output, while a searched move is
preceded by `info depth` lines. That difference is how a line that has left the book early is
detected and discarded.

Usage:
    scripts/make_opening_lines.py --plies 8 --count 250 --out openings/book_8ply.txt
"""

import argparse
import os
import subprocess
import sys
import threading
import time

REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), os.pardir))


class Engine:
    """A UCI engine subprocess with a line reader thread."""

    def __init__(self, path):
        self.proc = subprocess.Popen(
            [path],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
            bufsize=1,
        )
        self.lines = []
        self.lock = threading.Lock()
        self.reader = threading.Thread(target=self._read, daemon=True)
        self.reader.start()

    def _read(self):
        for line in self.proc.stdout:
            with self.lock:
                self.lines.append(line.strip())

    def send(self, command):
        self.proc.stdin.write(command + "\n")
        self.proc.stdin.flush()

    def drain(self):
        with self.lock:
            out, self.lines = self.lines, []
            return out

    def wait_for(self, prefix, timeout):
        """Collects output until a line starting with `prefix` appears."""
        collected = []
        deadline = time.time() + timeout
        while time.time() < deadline:
            for line in self.drain():
                collected.append(line)
                if line.startswith(prefix):
                    return collected
            time.sleep(0.002)
        raise TimeoutError("engine did not answer with '%s' in %.1fs" % (prefix, timeout))

    def quit(self):
        try:
            self.send("quit")
            self.proc.wait(timeout=3)
        except Exception:
            self.proc.kill()


def sample_line(engine, plies, move_time_ms):
    """Plays `plies` book moves and returns them, or None if the book ran out early."""
    engine.send("ucinewgame")
    engine.send("isready")
    engine.wait_for("readyok", timeout=10)

    moves = []
    for _ in range(plies):
        position = "position startpos"
        if moves:
            position += " moves " + " ".join(moves)
        engine.send(position)
        engine.send("go wtime %d btime %d winc 0 binc 0" % (move_time_ms, move_time_ms))
        output = engine.wait_for("bestmove", timeout=15)

        # A searched move is preceded by info output; a book move is not.
        if any(line.startswith("info depth") for line in output):
            return None

        best = [line for line in output if line.startswith("bestmove")][0].split()
        if len(best) < 2 or best[1] in ("(none)", "0000"):
            return None
        moves.append(best[1])

    return " ".join(moves)


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--engine", default=os.path.join("target", "release", "suprah"),
                        help="engine binary, relative to the repository root")
    parser.add_argument("--plies", type=int, default=8,
                        help="length of each opening line in half-moves (even keeps White to move)")
    parser.add_argument("--count", type=int, default=250, help="number of unique lines to collect")
    parser.add_argument("--out", default=None, help="output file, relative to the repository root")
    parser.add_argument("--max-attempts", type=int, default=0,
                        help="give up after this many samples (0 = 40x count)")
    parser.add_argument("--move-time", type=int, default=100,
                        help="clock in ms handed to the engine per probe")
    args = parser.parse_args()

    engine_path = os.path.join(REPO_ROOT, args.engine)
    if not os.path.isfile(engine_path):
        sys.exit("engine not found at %s - build it first" % args.engine)

    out_path = args.out or os.path.join("openings", "book_%dply_%d.txt" % (args.plies, args.count))
    out_path = os.path.join(REPO_ROOT, out_path)
    os.makedirs(os.path.dirname(out_path), exist_ok=True)

    max_attempts = args.max_attempts or args.count * 40

    engine = Engine(engine_path)
    engine.send("uci")
    engine.wait_for("uciok", timeout=10)

    seen = []
    unique = set()
    attempts = 0
    short_lines = 0
    try:
        while len(unique) < args.count and attempts < max_attempts:
            attempts += 1
            line = sample_line(engine, args.plies, args.move_time)
            if line is None:
                short_lines += 1
                continue
            if line not in unique:
                unique.add(line)
                seen.append(line)
                if len(seen) % 25 == 0:
                    print("  %d/%d unique lines after %d samples"
                          % (len(seen), args.count, attempts), flush=True)
    finally:
        engine.quit()

    if not seen:
        sys.exit("no book lines were produced - is the book reaching the engine?")

    with open(out_path, "w") as handle:
        handle.write("\n".join(seen) + "\n")

    print("wrote %d unique %d-ply lines to %s"
          % (len(seen), args.plies, os.path.relpath(out_path, REPO_ROOT)))
    print("%d samples, %d left the book before %d plies"
          % (attempts, short_lines, args.plies))
    if len(unique) < args.count:
        print("note: the book did not yield %d distinct lines at this depth" % args.count)


if __name__ == "__main__":
    main()
