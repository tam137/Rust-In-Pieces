#!/usr/bin/env python3
"""UCI smoke test for a single engine binary.

Starts the engine, walks it through the handshake and two searches (a fixed depth and a
classical time control with `movestogo`), then checks that it quits cleanly. Used by the
release-binaries workflow on every target before a binary is attached to a release, and
usable locally on any build:

    python3 scripts/uci_smoke_test.py <engine-binary> [--expect-name "Rust-In-Pieces V0.52.0"]

Exits 0 when every check passes and 1 on the first failure, printing the engine's output
for the failing step.
"""

import argparse
import os
import queue
import re
import subprocess
import sys
import threading
import time

# A middlegame position well outside any opening book, so both searches really search.
MIDGAME = "r1bq1rk1/pp2bppp/2n1pn2/3p4/2PP4/2N1PN2/PP3PPP/R2QKB1R w KQ - 0 9"

BESTMOVE = re.compile(r"^bestmove ([a-h][1-8][a-h][1-8][qrbn]?)\b")


class Engine:
    def __init__(self, path):
        self.proc = subprocess.Popen(
            [os.path.abspath(path)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            bufsize=1,
        )
        self.lines = queue.Queue()
        threading.Thread(target=self._pump, daemon=True).start()

    def _pump(self):
        for line in self.proc.stdout:
            self.lines.put(line.rstrip("\r\n"))
        self.lines.put(None)

    def send(self, command):
        self.proc.stdin.write(command + "\n")
        self.proc.stdin.flush()

    def expect(self, predicate, timeout):
        """Return (matching line, lines read) or (None, lines read) on timeout or EOF."""
        seen = []
        deadline = time.monotonic() + timeout
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                return None, seen
            try:
                line = self.lines.get(timeout=remaining)
            except queue.Empty:
                return None, seen
            if line is None:
                return None, seen
            seen.append(line)
            if predicate(line):
                return line, seen


def fail(step, seen):
    print(f"FAIL: {step}")
    for line in seen[-20:]:
        print(f"  < {line}")
    return 1


def run(path, expect_name):
    engine = Engine(path)

    engine.send("uci")
    _, seen = engine.expect(lambda l: l == "uciok", 10)
    if not seen or seen[-1] != "uciok":
        return fail("no uciok after uci", seen)
    names = [l[len("id name "):] for l in seen if l.startswith("id name ")]
    if not names:
        return fail("no id name", seen)
    if expect_name is not None and names[0] != expect_name:
        return fail(f"id name is '{names[0]}', expected '{expect_name}'", seen)
    print(f"ok: id name {names[0]}")

    engine.send("isready")
    line, seen = engine.expect(lambda l: l == "readyok", 30)
    if line is None:
        return fail("no readyok", seen)
    print("ok: readyok")

    engine.send(f"position fen {MIDGAME}")
    engine.send("go depth 10")
    line, seen = engine.expect(lambda l: l.startswith("bestmove"), 120)
    if line is None or not BESTMOVE.match(line):
        return fail("no legal-looking bestmove after go depth 10", seen)
    print(f"ok: depth 10 -> {line}")

    engine.send("ucinewgame")
    engine.send("isready")
    line, seen = engine.expect(lambda l: l == "readyok", 30)
    if line is None:
        return fail("no readyok after ucinewgame", seen)
    engine.send(f"position fen {MIDGAME} moves a2a3")
    engine.send("go wtime 60000 btime 60000 movestogo 40")
    started = time.monotonic()
    line, seen = engine.expect(lambda l: l.startswith("bestmove"), 30)
    if line is None or not BESTMOVE.match(line):
        return fail("no legal-looking bestmove after go wtime/btime/movestogo", seen)
    print(f"ok: movestogo 40 -> {line} in {time.monotonic() - started:.1f}s")

    engine.send("quit")
    try:
        engine.proc.wait(timeout=10)
    except subprocess.TimeoutExpired:
        engine.proc.kill()
        return fail("engine did not exit after quit", [])
    print("ok: quit")
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("engine", help="path to the engine binary")
    parser.add_argument("--expect-name", help="exact UCI id name the engine must report")
    args = parser.parse_args()
    sys.exit(run(args.engine, args.expect_name))


if __name__ == "__main__":
    main()
