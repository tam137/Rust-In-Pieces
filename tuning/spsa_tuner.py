#!/usr/bin/env python3
"""SPSA tuner for the Suprah engine, in the form Fishtest uses (Matt-Magie step 94, `task.md` 23.4).

One iteration is one game pair: both games start from the same opening line, colours swapped,
theta+ = theta + c_k * delta against theta- = theta - c_k * delta. After the pair

    theta_i += (a_k / c_k_i) * (wins - losses of theta+) * delta_i

with the schedules

    c_k_i = c_i / k^gamma,              c_i = c_end_i * N^gamma
    a_k_i = a_i / (A + k)^alpha,        a_i = r_end * c_end_i^2 * (A + N)^alpha,   A = 0.1 * N

so that at the last iteration the perturbation is `c_end` and the step per unit of result is
`r_end * c_end`. `c_end` is absolute and per parameter (`parameters.json`), which is what the old
tuner lacked: its step was a percentage of the value, so a parameter at 0 could never move.

Pairs run asynchronously: `workers` pairs are in flight, each drawn from the theta current when it
was issued, and each update is applied as soon as its pair finishes. A pair with a game that has
no result is discarded (no update) and still uses up its iteration; the 1% abort rule counts over
the whole run, resumed sessions included. After an interruption the run resumes at
`k_done + 1`: pairs that were in flight are dropped, so a few schedule indices may be played twice
and others not at all - harmless for the schedules, visible as repeated `k` in the history.
"""

import argparse
import concurrent.futures
import csv
import json
import math
import os
import random
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import threading
import time

GAMMA = 0.101
ALPHA = 0.602
STATE_VERSION = 2
MAX_WORKERS = 12  # owner 2026-11-07: 12 of the 16 cores of the local host; one game uses about one core
MAX_REDRAWS = 100


# --------------------------------------------------------------------------------------------
# Parameters and schedules
# --------------------------------------------------------------------------------------------

class ConfigError(Exception):
    pass


def load_param_defs(path):
    with open(path, "r") as f:
        return json.load(f)


def validate_active(defs, active):
    """Errors for the active parameters, warnings for values on a bound."""
    errors, warnings = [], []
    for name in active:
        if name not in defs:
            errors.append(f"{name}: not in parameters.json")
            continue
        d = defs[name]
        c_end = d.get("c_end")
        if c_end is None:
            errors.append(f"{name}: no c_end")
            continue
        if c_end < 1:
            errors.append(f"{name}: c_end {c_end} < 1 (integer parameters)")
        if d["max"] - d["min"] < 4 * c_end:
            errors.append(f"{name}: range [{d['min']}, {d['max']}] narrower than 4 * c_end ({c_end})")
        if not d["min"] <= d["value"] <= d["max"]:
            errors.append(f"{name}: value {d['value']} outside [{d['min']}, {d['max']}]")
        elif d["value"] in (d["min"], d["max"]):
            warnings.append(f"{name}: value {d['value']} sits on a bound [{d['min']}, {d['max']}]")
    return errors, warnings


class Schedule:
    def __init__(self, c_end, r_end, n_iters):
        self.n = n_iters
        self.big_a = 0.1 * n_iters
        self.c = c_end * n_iters ** GAMMA
        self.a = r_end * c_end ** 2 * (self.big_a + n_iters) ** ALPHA

    def c_k(self, k):
        return self.c / k ** GAMMA

    def a_k(self, k):
        return self.a / (self.big_a + k) ** ALPHA

    def step(self, k):
        """Update per unit of result (wins - losses of theta+) at iteration k."""
        return self.a_k(k) / self.c_k(k)


def clamp(v, lo, hi):
    return max(lo, min(hi, v))


# --------------------------------------------------------------------------------------------
# PGN
# --------------------------------------------------------------------------------------------

RESULT_RE = re.compile(r'\[Result "([^"]*)"\]')
TERM_RE = re.compile(r'\[Termination "([^"]*)"\]')


def parse_pgn(text):
    """(result, termination) of the first game in `text`; result None if there is none."""
    m = RESULT_RE.search(text)
    if not m or m.group(1) not in ("1-0", "0-1", "1/2-1/2"):
        return None, None
    t = TERM_RE.search(text)
    return m.group(1), (t.group(1) if t else "")


def score_for_white(result):
    return {"1-0": 1, "0-1": -1, "1/2-1/2": 0}[result]


# --------------------------------------------------------------------------------------------
# Tuner
# --------------------------------------------------------------------------------------------

class Job:
    __slots__ = ("k", "delta", "plus", "minus", "opening_idx", "theta_at_issue")

    def __init__(self, k, delta, plus, minus, opening_idx):
        self.k, self.delta, self.plus, self.minus, self.opening_idx = k, delta, plus, minus, opening_idx


class SPSATuner:
    def __init__(self, defs, active, n_iters, r_end, seed, group,
                 state_path, history_path, engine_path="", num_openings=1, start=""):
        if not active:
            raise ConfigError("no active parameters")
        errors, self.warnings = validate_active(defs, active)
        if errors:
            raise ConfigError("; ".join(errors))
        self.defs = defs
        self.names = list(defs.keys())
        self.active = list(active)
        self.n = n_iters
        self.r_end = r_end
        self.group = group
        self.state_path = state_path
        self.history_path = history_path
        self.engine_path = engine_path
        self.num_openings = num_openings
        self.start = start
        self.sched = {p: Schedule(defs[p]["c_end"], r_end, n_iters) for p in self.active}
        self.theta = {p: float(defs[p]["value"]) for p in self.names}
        self.seed = seed
        self.k_next = 1
        self.k_done = 0  # pairs finished (applied or discarded); in-flight pairs are not counted
        self.counters = {"pairs": 0, "discarded": 0, "wins": 0, "draws": 0, "losses": 0,
                         "time_losses": 0, "illegal": 0, "redraws": 0}
        self.lock = threading.Lock()
        self._load_or_init()
        # The draw sequence continues where it stopped, but a resumed run need not repeat
        # draws exactly: in-flight pairs at the interruption are reissued with fresh draws.
        self.rng = random.Random(f"{self.seed}:{self.k_next}")

    # -- state ---------------------------------------------------------------------------------

    def _fingerprint(self):
        return {"group": self.group, "active": self.active, "n": self.n, "r_end": self.r_end,
                "c_end": {p: self.defs[p]["c_end"] for p in self.active},
                "engine": os.path.basename(self.engine_path), "start": self.start}

    def _load_or_init(self):
        if os.path.exists(self.state_path):
            with open(self.state_path) as f:
                state = json.load(f)
            if state.get("version") != STATE_VERSION:
                old = self.state_path[:-5] + ".v1.json" if self.state_path.endswith(".json") else self.state_path + ".v1"
                os.replace(self.state_path, old)
                print(f"old state format moved to {old}; starting fresh")
            else:
                fp = state["fingerprint"]
                mine = self._fingerprint()
                diff = [k for k in mine if fp.get(k) != mine[k]]
                if diff:
                    raise ConfigError(
                        f"state {self.state_path} belongs to another run (differs in {', '.join(diff)}); "
                        "move it aside or hot-patch it by hand")
                self.k_done = state["k_done"]
                self.k_next = self.k_done + 1
                self.seed = state["seed"]
                self.counters.update(state["counters"])
                for p, v in state["theta"].items():
                    if p in self.theta:
                        self.theta[p] = float(v)
                print(f"resumed at iteration {self.k_next} of {self.n}")
                return
        if os.path.exists(self.history_path):
            backup = f"{self.history_path}.{time.strftime('%Y%m%d-%H%M%S')}.bak"
            os.replace(self.history_path, backup)
            print(f"history of an earlier run moved to {backup}")
        with open(self.history_path, "w", newline="") as f:
            csv.writer(f).writerow(["k", "opening", "result", "w", "d", "l"] + self.active)
        self._save()

    def _save(self):
        state = {"version": STATE_VERSION, "fingerprint": self._fingerprint(), "seed": self.seed,
                 "k_done": self.k_done, "counters": self.counters, "theta": self.theta}
        tmp = self.state_path + ".tmp"
        with open(tmp, "w") as f:
            json.dump(state, f, indent=2)
        os.replace(tmp, self.state_path)

    # -- iterations ----------------------------------------------------------------------------

    def rounded(self, values):
        return {p: int(round(clamp(values[p], self.defs[p]["min"], self.defs[p]["max"])))
                for p in self.names}

    def done(self):
        return self.k_next > self.n

    def next_job(self):
        """Draw the next pair from the current theta. Must hold the lock."""
        k = self.k_next
        self.k_next += 1
        for _ in range(MAX_REDRAWS):
            delta = {p: self.rng.choice((-1, 1)) for p in self.active}
            plus, minus = dict(self.theta), dict(self.theta)
            for p in self.active:
                ck = self.sched[p].c_k(k)
                plus[p] = self.theta[p] + ck * delta[p]
                minus[p] = self.theta[p] - ck * delta[p]
            plus, minus = self.rounded(plus), self.rounded(minus)
            if any(plus[p] != minus[p] for p in self.active):
                break
            self.counters["redraws"] += 1
        else:
            raise RuntimeError(f"iteration {k}: theta+ and theta- round to the same values {MAX_REDRAWS} times")
        return Job(k, delta, plus, minus, self.rng.randrange(self.num_openings))

    def apply(self, job, games):
        """`games`: two (result, termination) tuples, game 0 with theta+ as White, game 1 with
        theta+ as Black. Returns False if the pair was discarded. Must hold the lock."""
        self.counters["pairs"] += 1
        self.k_done += 1
        if any(r is None for r, _ in games):
            self.counters["discarded"] += 1
            self._save()
            return False
        w = d = l = 0
        for i, (res, term) in enumerate(games):
            s = score_for_white(res) * (1 if i == 0 else -1)
            w, d, l = w + (s > 0), d + (s == 0), l + (s < 0)
            self.counters["time_losses"] += "ByTime" in term
            self.counters["illegal"] += "IllegalMove" in term
        result = w - l
        self.counters["wins"] += w
        self.counters["draws"] += d
        self.counters["losses"] += l
        for p in self.active:
            dlt = self.sched[p].step(job.k) * result * job.delta[p]
            self.theta[p] = clamp(self.theta[p] + dlt, self.defs[p]["min"], self.defs[p]["max"])
        self._save()
        with open(self.history_path, "a", newline="") as f:
            csv.writer(f).writerow([job.k, job.opening_idx, result, w, d, l] +
                                   [round(self.theta[p], 3) for p in self.active])
        return True


# --------------------------------------------------------------------------------------------
# Games
# --------------------------------------------------------------------------------------------

def format_options(values, extra):
    return ",".join([f"{k}={v}" for k, v in values.items()] + extra)


class MattMagieRunner:
    """Plays a pair with Matt-Magie (arguments as `mm.sh` passes them)."""

    def __init__(self, mm_path, engine_path, time_ms, inc_ms, hash_mb, openings, run_dir, logpath=""):
        self.mm = mm_path
        self.engine = engine_path
        self.time_ms, self.inc_ms = time_ms, inc_ms
        self.hash_mb = hash_mb
        self.openings = openings
        self.run_dir = run_dir
        self.logpath = logpath
        self.procs = set()
        self.plock = threading.Lock()
        self.stopping = False
        os.makedirs(run_dir, exist_ok=True)

    def _game(self, tag, white_opts, black_opts, opening):
        pgn = os.path.join(self.run_dir, f"{tag}.pgn")
        if os.path.exists(pgn):
            os.remove(pgn)
        cmd = [self.mm, self.engine, self.engine, "/dev/null", pgn, "SPSA", "Local", tag,
               str(self.time_ms), str(self.inc_ms), "log_off", "debug_off", white_opts, black_opts, opening]
        with self.plock:
            if self.stopping:
                return None, None
            proc = subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                                    start_new_session=True)
            self.procs.add(proc)
        proc.wait()
        with self.plock:
            self.procs.discard(proc)
        if self.stopping or not os.path.exists(pgn):
            return None, None
        with open(pgn) as f:
            text = f.read()
        os.remove(pgn)
        return parse_pgn(text)

    def play_pair(self, job):
        extra = [f"Hash={self.hash_mb}"] + ([f"logpath={self.logpath}"] if self.logpath else [])
        plus = format_options(job.plus, extra)
        minus = format_options(job.minus, extra)
        opening = self.openings[job.opening_idx] if self.openings else ""
        # The pid keeps two tuners on one run directory, and a Matt-Magie left over from an
        # earlier process of this run (k repeats after a resume), out of each other's PGNs.
        tag = f"p{os.getpid()}_k{job.k}"
        g0 = self._game(f"{tag}_a", plus, minus, opening)   # theta+ White
        g1 = self._game(f"{tag}_b", minus, plus, opening)   # theta+ Black
        return [g0, g1]

    def stop(self):
        with self.plock:
            self.stopping = True
            for p in self.procs:
                try:
                    os.killpg(p.pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass


def run(tuner, runner, workers, max_consecutive_failures=3, max_failure_rate=0.01,
        max_time_loss_rate=0.02, progress=True):
    """Keeps `workers` pairs in flight until N iterations are issued and finished."""
    consecutive = 0
    t0 = time.time()
    pairs_at_start = tuner.counters["pairs"]
    with concurrent.futures.ThreadPoolExecutor(max_workers=workers) as pool:
        inflight = {}

        def submit():
            with tuner.lock:
                if tuner.done():
                    return
                job = tuner.next_job()
            inflight[pool.submit(runner.play_pair, job)] = job

        try:
            for _ in range(workers):
                submit()
            while inflight:
                finished, _ = concurrent.futures.wait(inflight, return_when=concurrent.futures.FIRST_COMPLETED)
                for fut in finished:
                    job = inflight.pop(fut)
                    games = fut.result()
                    with tuner.lock:
                        ok = tuner.apply(job, games)
                        c = tuner.counters
                    consecutive = 0 if ok else consecutive + 1
                    if consecutive >= max_consecutive_failures:
                        raise RuntimeError(f"{consecutive} pairs in a row without a result; last k={job.k}")
                    if c["pairs"] >= 100 and c["discarded"] > max_failure_rate * c["pairs"]:
                        raise RuntimeError(f"{c['discarded']} of {c['pairs']} pairs discarded (> {max_failure_rate:.0%})")
                    if c["pairs"] >= 50 and c["time_losses"] > max_time_loss_rate * 2 * c["pairs"]:
                        raise RuntimeError(f"{c['time_losses']} time losses in {2 * c['pairs']} games "
                                           f"(> {max_time_loss_rate:.0%}): the time control is too short")
                    if progress:
                        rate = (c["pairs"] - pairs_at_start) / max(1e-9, time.time() - t0) * 3600
                        shown = " ".join(f"{p}={tuner.theta[p]:.1f}" for p in tuner.active[:6])
                        print(f"\rk {job.k}/{tuner.n} +{c['wins']} ={c['draws']} -{c['losses']} "
                              f"disc {c['discarded']} time {c['time_losses']} ill {c['illegal']} "
                              f"{rate:.0f} pairs/h | {shown}   ", end="", flush=True)
                    submit()
        except BaseException:
            runner.stop()
            for fut in inflight:
                fut.cancel()
            raise
    if progress:
        print()


# --------------------------------------------------------------------------------------------
# Preflight: every option the tuner sends must reach the engine
# --------------------------------------------------------------------------------------------

def preflight(engine_path, defs, active, hash_mb, timeout=20, logpath=""):
    """Starts the engine once with a log directory and sends every option a game will send, each
    active one at a value other than its default. Raises if one is unknown or not logged."""
    probe = {}
    for p in defs:
        d = defs[p]
        v = int(round(d["value"]))
        if p in active:
            v = v + 1 if v + 1 <= d["max"] else v - 1
        probe[p] = v
    probe["Hash"] = hash_mb
    if logpath:
        probe["logpath"] = logpath  # sent with every game; checked last, it redirects the log
    logdir = tempfile.mkdtemp(prefix="spsa_preflight_")
    try:
        checked = {k: v for k, v in probe.items() if k != "logpath"}
        lines = ["uci", f"setoption name logpath value {logdir}"]
        lines += [f"setoption name {k} value {v}" for k, v in checked.items()]
        lines += ["isready"]
        proc = subprocess.Popen([engine_path], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                stderr=subprocess.DEVNULL, text=True)
        ready = threading.Event()

        def reader():
            for line in proc.stdout:
                if line.strip() == "readyok":
                    ready.set()
            proc.stdout.close()

        threading.Thread(target=reader, daemon=True).start()
        proc.stdin.write("\n".join(lines) + "\n")
        proc.stdin.flush()
        if not ready.wait(timeout):
            proc.kill()
            proc.wait()
            raise ConfigError(f"preflight: no readyok from {engine_path} within {timeout}s")
        time.sleep(0.5)  # the engine logs through a buffer thread
        proc.stdin.write("quit\n")
        proc.stdin.close()
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait()
        text = ""
        for name in os.listdir(logdir):
            with open(os.path.join(logdir, name)) as f:
                text += f.read()
        problems = []
        if logpath and not os.path.isdir(logpath):
            try:
                os.makedirs(logpath, exist_ok=True)
            except OSError as e:
                problems.append(f"logpath {logpath}: {e}")
        for k, v in checked.items():
            if k == "Hash":  # handled by the UCI thread, not by `Config`, and logged differently
                if f"Hash size dynamically set to {v} MB" not in text:
                    problems.append("Hash: not confirmed in the engine log")
                continue
            if f"Unknown option ignored: {k} = " in text:
                problems.append(f"{k}: unknown to the engine")
            elif f"Received option: {k} = {v}" not in text:
                problems.append(f"{k}: not confirmed in the engine log")
        if problems:
            raise ConfigError("preflight failed: " + "; ".join(problems))
        return len(checked) + bool(logpath)
    finally:
        shutil.rmtree(logdir, ignore_errors=True)


# --------------------------------------------------------------------------------------------
# CLI
# --------------------------------------------------------------------------------------------

def main(argv=None):
    here = os.path.dirname(os.path.abspath(__file__))
    ap = argparse.ArgumentParser(description="SPSA tuner for Suprah (Fishtest form, game pairs)")
    ap.add_argument("--group", required=True, help="group from groups.json; with --params only a label")
    ap.add_argument("--params", default="", help="comma-separated parameters, overrides the group's list")
    ap.add_argument("--engine", required=True)
    ap.add_argument("--mm", required=True, help="Matt-Magie binary")
    ap.add_argument("--openings", required=True, help="opening lines, one line of UCI moves each")
    ap.add_argument("--iters", type=int, required=True, help="N, the number of game pairs")
    ap.add_argument("--r-end", type=float, default=0.002)
    ap.add_argument("--workers", type=int, default=12)
    ap.add_argument("--time", type=int, default=1000, help="time per game in ms")
    ap.add_argument("--inc", type=int, default=50,
                    help="increment per move in ms; at 10 ms the engine loses most games on time (step 94)")
    ap.add_argument("--hash", type=int, default=16)
    ap.add_argument("--seed", type=int, default=None)
    ap.add_argument("--param-file", default=os.path.join(here, "parameters.json"))
    ap.add_argument("--start", default="", help="name=value,... starting theta (e.g. a displaced value)")
    ap.add_argument("--state", default=os.path.join(here, "spsa_state.json"))
    ap.add_argument("--history", default=os.path.join(here, "spsa_history.csv"))
    ap.add_argument("--run-dir", default=os.path.join(here, "run"))
    ap.add_argument("--logpath", default="", help="engine log directory (off by default)")
    ap.add_argument("--dry-run", action="store_true", help="preflight and schedule only")
    args = ap.parse_args(argv)

    if args.workers > MAX_WORKERS:
        sys.exit(f"--workers {args.workers}: at most {MAX_WORKERS} games at a time on this host")

    defs = load_param_defs(args.param_file)
    if args.params:
        active = [p.strip() for p in args.params.split(",") if p.strip()]
    else:
        with open(os.path.join(here, "groups.json")) as f:
            groups = json.load(f)
        if args.group not in groups:
            sys.exit(f"unknown group {args.group}; available: {', '.join(groups)}")
        active = groups[args.group]
    if args.start:
        for item in args.start.split(","):
            name, val = item.split("=")
            if name not in defs:
                sys.exit(f"--start: {name} not in parameters.json")
            defs[name]["value"] = float(val)

    with open(args.openings) as f:
        openings = [ln.strip() for ln in f if ln.strip()]
    seed = args.seed if args.seed is not None else random.randrange(1 << 30)

    try:
        tuner = SPSATuner(defs, active, args.iters, args.r_end, seed, args.group,
                          args.state, args.history, args.engine, len(openings), args.start)
    except ConfigError as e:
        sys.exit(f"error: {e}")
    for w in tuner.warnings:
        print(f"warning: {w}")
    try:
        n_opts = preflight(args.engine, defs, active, args.hash, logpath=args.logpath)
    except ConfigError as e:
        sys.exit(f"error: {e}")
    print(f"preflight: {n_opts} options confirmed by the engine")
    print(f"group {args.group}: {len(active)} parameters, N={args.iters}, r_end={args.r_end}, seed={tuner.seed}")
    for p in active:
        s = tuner.sched[p]
        print(f"  {p:40s} theta {tuner.theta[p]:8.2f}  c_1 {s.c_k(1):7.2f}  c_N {s.c_k(args.iters):6.2f}  "
              f"step_1 {s.step(1):.4f}  step_N {s.step(args.iters):.4f}")
    if args.dry_run:
        return
    def as_interrupt(signum, frame):
        raise KeyboardInterrupt(f"signal {signum}")

    # SIGTERM and a lost terminal stop the run like Ctrl-C: Matt-Magie runs in its own session
    # and would otherwise keep playing (and keep appending to its PGN) after the tuner is gone.
    signal.signal(signal.SIGTERM, as_interrupt)
    signal.signal(signal.SIGHUP, as_interrupt)
    runner = MattMagieRunner(args.mm, args.engine, args.time, args.inc, args.hash, openings,
                             args.run_dir, args.logpath)
    try:
        run(tuner, runner, args.workers)
    except KeyboardInterrupt:
        print("\ninterrupted; in-flight pairs discarded, state saved")
    print("final theta:")
    for p in active:
        print(f"  {p:40s} {tuner.theta[p]:.2f}")


if __name__ == "__main__":
    main()
