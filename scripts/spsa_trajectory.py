#!/usr/bin/env python3
"""How the parameters of an SPSA run have moved, as a table (Matt-Magie step 99).

`check_spsa_status.py` shows where theta stands now; this shows the path it took. The history of
`tuning/spsa_tuner.py` holds one row per finished game pair with theta after that pair's update.
The rows are written in the order the pairs finish, so the file order is the time axis (the `k`
column jumps, because pairs run in parallel).

Output:
- a header: progress k/N, pairs per hour and the expected end (from the last progress line of the
  tuner's log), W/D/L, time losses, illegal moves, discarded pairs, and the watcher's alert file;
- a table of about `--rows` evenly spaced checkpoints, each the mean of theta over the window of
  pairs since the previous checkpoint (a single row of SPSA theta is mostly noise), with the start
  value as the first row;
- per parameter: the current theta, its start, the change and the change in units of `c_end`, the
  parameter's absolute final perturbation, which is the scale the tuner's schedule is built on;
- and `z`, the change against a random walk made of the same per-pair steps: the drift divided by
  the square root of the sum of the squared steps. Without a gradient the signs of the steps are
  random and `z` stays around +-1; a real gradient makes it grow with the run. The windowed table
  looks smooth either way, so it cannot tell the two apart; `|z| >= 2` is the reading for a signal.

The start is the `value` of the parameter file. A run started with `--start` is shown against
`value`, not against its displaced start.

Usage (paths default to files next to the state that share its prefix, `<prefix>_state.json`):
    scripts/spsa_trajectory.py <prefix>_state.json [--param-file P] [--history H] [--rows 10]
                               [--log L] [--alert A]
"""
import argparse
import csv
import datetime
import json
import math
import os
import re
import sys

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DEFAULT_PARAMS = os.path.join(REPO, "tuning", "parameters.json")


def sibling(state_path, suffix):
    """`<prefix>_<suffix>` next to `<prefix>_state.json`; `spsa_<suffix>` for `spsa_state.json`."""
    base = os.path.basename(state_path)
    prefix = base[:-len("_state.json")] if base.endswith("_state.json") else os.path.splitext(base)[0]
    return os.path.join(os.path.dirname(os.path.abspath(state_path)), f"{prefix}_{suffix}")


def checkpoints(n_pairs, rows):
    """About `rows` evenly spaced pair counts in 1..n_pairs; the last one is always n_pairs."""
    if n_pairs <= 0:
        return []
    if n_pairs <= rows:
        return list(range(1, n_pairs + 1))
    step = n_pairs // rows
    points = list(range(step, n_pairs + 1, step))
    if points[-1] != n_pairs:
        if n_pairs - points[-1] < step / 2:
            points[-1] = n_pairs
        else:
            points.append(n_pairs)
    return points


def trajectory(series, points):
    """(checkpoint, mean of series over the pairs since the previous checkpoint) per checkpoint."""
    out, prev = [], 0
    for p in points:
        window = series[prev:p]
        out.append((p, sum(window) / len(window)))
        prev = p
    return out


def drift(theta, start, c_end):
    """The change from the start in units of the final perturbation `c_end`."""
    return (theta - start) / c_end


def z_score(series, start):
    """The drift of `series` (theta after each pair) from `start` over the random-walk sigma of
    its own steps, or None without history or without any movement."""
    if not series:
        return None
    previous, squares = start, 0.0
    for value in series:
        squares += (value - previous) ** 2
        previous = value
    if squares == 0.0:
        return None
    return (series[-1] - start) / math.sqrt(squares)


def read_history(path, active):
    """Theta per active parameter, one value per finished pair, in file order."""
    with open(path, newline="") as f:
        reader = csv.DictReader(f)
        series = {p: [] for p in active}
        for row in reader:
            for p in active:
                series[p].append(float(row[p]))
    return series


def last_rate(log_path):
    """Pairs per hour from the last progress line of the tuner's log, or None."""
    try:
        with open(log_path, "rb") as f:
            f.seek(max(0, os.path.getsize(log_path) - 8192))
            tail = f.read().decode("utf-8", "replace")
    except OSError:
        return None
    found = re.findall(r"(\d+) pairs/h", tail)
    return int(found[-1]) if found else None


def fmt(value, width):
    return f"{value:>{width}.2f}"


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("state")
    ap.add_argument("--param-file", default=DEFAULT_PARAMS)
    ap.add_argument("--history")
    ap.add_argument("--rows", type=int, default=10)
    ap.add_argument("--log")
    ap.add_argument("--alert")
    args = ap.parse_args(argv)
    history = args.history or sibling(args.state, "history.csv")
    log = args.log or sibling(args.state, "spsa.log")
    alert = args.alert or sibling(args.state, "alert.txt")

    with open(args.state) as f:
        state = json.load(f)
    with open(args.param_file) as f:
        defs = json.load(f)
    fp = state.get("fingerprint", {})
    active = fp.get("active", [])
    c_end = fp.get("c_end", {})
    n, k = fp.get("n", 0), state.get("k_done", 0)
    c = state.get("counters", {})

    # Header.
    rate = last_rate(log)
    eta = ""
    if rate and n > k:
        end = datetime.datetime.now() + datetime.timedelta(hours=(n - k) / rate)
        eta = f"   end about {end:%a %H:%M}"
    pct = 100.0 * k / n if n else 0.0
    rate_s = f"{rate} pairs/h" if rate else "rate n/a"
    print(f"{fp.get('group')}  {fp.get('engine')}  k {k}/{n} ({pct:.1f}%)  {rate_s}{eta}")
    print(f"  +{c.get('wins', 0)} ={c.get('draws', 0)} -{c.get('losses', 0)} (theta+ vs theta-)   "
          f"time losses {c.get('time_losses', 0)}   illegal {c.get('illegal', 0)}   "
          f"discarded {c.get('discarded', 0)}")
    if os.path.exists(alert):
        with open(alert) as f:
            print(f"  ALERT: {f.read().strip()}")
    else:
        print("  watcher: no alert")

    # Table.
    names = active
    widths = [max(9, len(p)) for p in names]
    series = {}
    if not os.path.exists(history):
        print("\n  no history")
    else:
        series = read_history(history, active)
        n_pairs = len(series[active[0]]) if active else 0
        if n_pairs == 0:
            print("\n  no pairs in the history yet")
        else:
            points = checkpoints(n_pairs, args.rows)
            columns = {p: trajectory(series[p], points) for p in active}
            print(f"\n  mean of theta over the pairs since the previous row")
            print("  " + f"{'pairs':>7s}  " + "  ".join(f"{p:>{w}s}" for p, w in zip(names, widths)))
            print("  " + f"{'start':>7s}  " + "  ".join(
                fmt(float(defs[p]["value"]), w) for p, w in zip(names, widths)))
            for i, point in enumerate(points):
                print("  " + f"{point:>7d}  " + "  ".join(
                    fmt(columns[p][i][1], w) for p, w in zip(names, widths)))

    # Current theta and the drift in units of c_end.
    theta = state.get("theta", {})
    name_w = max([9] + [len(p) for p in names])
    print(f"\n  {'parameter':<{name_w}s}  {'start':>7s}  {'now':>9s}  {'change':>8s}  {'in c_end':>8s}  "
          f"{'z':>6s}  {'c_end':>5s}  range")
    for p in names:
        d = defs.get(p, {})
        start, now, ce = d.get("value"), theta.get(p), c_end.get(p, d.get("c_end"))
        if start is None or now is None or not ce:
            print(f"  {p:<{name_w}s}  (no start, theta or c_end)")
            continue
        flag = "  on bound" if now <= d.get("min") or now >= d.get("max") else ""
        z = z_score(series.get(p, []), start)
        z_s = f"{z:>+6.2f}" if z is not None else f"{'n/a':>6s}"
        print(f"  {p:<{name_w}s}  {start:>7}  {now:>9.2f}  {now - start:>+8.2f}  "
              f"{drift(now, start, ce):>+8.2f}  {z_s}  {ce:>5}  [{d.get('min')}, {d.get('max')}]{flag}")
    print("\n  z: the change over a random walk of the same per-pair steps; |z| >= 2 reads as a signal")
    return 0


if __name__ == "__main__":
    sys.exit(main())
