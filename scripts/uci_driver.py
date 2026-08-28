#!/usr/bin/env python3
"""Drives a Suprah binary over UCI and waits for the search to actually finish.

The earlier measurement scripts start a search and then `sleep` for a fixed number of seconds
before sending `quit`. That is unsafe for anything but the smallest positions: a search that has
not reached the requested depth when `quit` arrives is killed mid-iteration, and the numbers that
survive on stderr are then whatever the last *completed* iteration happened to be. The totals move
with machine load rather than with the change under test.

This driver instead reads stdout until `bestmove` appears, so a fixed-depth search is always
compared against the same fixed-depth search. Timings come from the engine's own `info ... time`
field rather than from a wall clock around the process, so process start-up is not counted.
"""
import os
import subprocess
import tempfile


class SearchResult:
    __slots__ = ("depth", "score", "best_move", "time_ms", "stderr", "info_signature")

    def __init__(self, depth, score, best_move, time_ms, stderr, info_signature):
        self.depth = depth
        self.score = score
        self.best_move = best_move
        self.time_ms = time_ms
        self.stderr = stderr
        self.info_signature = info_signature

    @property
    def agrees_with(self):
        return (self.depth, self.score, self.best_move)


def search(binary, fen, depth, options=(), timeout=300, cwd=None):
    """Runs one fixed-depth search to completion and returns a `SearchResult`.

    `options` is a sequence of `(name, value)` pairs sent as `setoption` before the search.
    """
    with tempfile.TemporaryFile(mode="w+") as err:
        proc = subprocess.Popen(
            [binary], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=err,
            text=True, bufsize=1, cwd=cwd,
        )
        try:
            commands = ["uci", "setoption name OwnBook value false"]
            commands += [f"setoption name {name} value {value}" for name, value in options]
            commands += ["isready", f"position fen {fen}", f"go depth {depth}"]
            proc.stdin.write("\n".join(commands) + "\n")
            proc.stdin.flush()

            info = []
            best_move = ""
            # `communicate` cannot be used here: the engine only emits `bestmove` in response to
            # a `go` that has run to completion, and closing stdin first would race the search.
            for line in proc.stdout:
                line = line.strip()
                if line.startswith("info depth"):
                    info.append(line)
                elif line.startswith("bestmove"):
                    parts = line.split()
                    best_move = parts[1] if len(parts) > 1 else ""
                    break
            proc.stdin.write("quit\n")
            proc.stdin.flush()
            proc.wait(timeout=10)
        finally:
            if proc.poll() is None:
                proc.kill()
                proc.wait()
            err.seek(0)
            stderr = err.read()

    reached, score, time_ms = "", "", 0
    signature = []
    for line in info:
        tokens = line.split()
        entry = {}
        for key in ("depth", "score", "nodes", "pv"):
            if key in tokens:
                idx = tokens.index(key)
                entry[key] = " ".join(tokens[idx + 1:]) if key == "pv" else tokens[idx + 1]
        signature.append(tuple(sorted(entry.items())))
        reached = tokens[tokens.index("depth") + 1]
        if "score" in tokens:
            score = " ".join(tokens[tokens.index("score") + 1:tokens.index("score") + 3])
        if "time" in tokens:
            time_ms = int(tokens[tokens.index("time") + 1])

    return SearchResult(reached, score, best_move, time_ms, stderr, signature)
