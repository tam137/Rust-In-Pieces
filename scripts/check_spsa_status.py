#!/usr/bin/env python3
"""Status of an SPSA tuning run (tuning/spsa_tuner.py, state version 2).

Usage: python3 scripts/check_spsa_status.py [state.json]
The state path can also be given as SPSA_STATE; default tuning/spsa_state.json in the repository.
The history is read from spsa_history.csv next to the state (or SPSA_HISTORY).
"""
import csv
import json
import os
import subprocess
import sys

GREEN = "\033[0;32m"
RED = "\033[0;31m"
YELLOW = "\033[0;33m"
CYAN = "\033[0;36m"
BOLD = "\033[1m"
NC = "\033[0m"

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TUNING = os.path.join(REPO, "tuning")
HISTORY_ROWS = 5


def sh(cmd):
    res = subprocess.run(["bash", "-c", cmd], capture_output=True, text=True)
    return res.stdout.strip()


def header(title):
    print(f"\n{YELLOW}{BOLD}{title}{NC}")


def main():
    state_path = sys.argv[1] if len(sys.argv) > 1 else os.environ.get(
        "SPSA_STATE", os.path.join(TUNING, "spsa_state.json"))
    history_path = os.environ.get(
        "SPSA_HISTORY", os.path.join(os.path.dirname(os.path.abspath(state_path)), "spsa_history.csv"))
    params_path = os.path.join(TUNING, "parameters.json")

    print(f"{CYAN}{BOLD}SPSA TUNING STATUS{NC}")
    print(f"state:   {state_path}\nhistory: {history_path}")

    header("Process")
    procs = [l for l in sh("ps -eo args").splitlines()
             if l.split() and "python" in os.path.basename(l.split()[0]) and "spsa_tuner.py" in l]
    if procs:
        for line in procs:
            print(f"  {line}")
            if "--workers" in line:
                w = line.split("--workers")[1].split()[0]
                if w.isdigit() and int(w) > 4:
                    print(f"  {RED}--workers {w} exceeds 4{NC}")
    else:
        print(f"  {YELLOW}no spsa_tuner.py process running{NC}")
    pane = sh("tmux capture-pane -t spsa_tuning -p 2>/dev/null")
    lines = [l for l in pane.splitlines() if l.strip()]
    if lines:
        print("  tmux spsa_tuning (last 5 lines):")
        for l in lines[-5:]:
            print(f"    > {l}")

    header("State")
    if not os.path.exists(state_path):
        print(f"  {YELLOW}no state at {state_path}{NC}")
        return
    try:
        with open(state_path) as f:
            state = json.load(f)
    except (OSError, ValueError) as e:
        print(f"  {RED}cannot read state: {e}{NC}")
        return
    if state.get("version") != 2:
        print(f"  {RED}state version {state.get('version')} is not 2 (old tuner format){NC}")
        return
    fp = state.get("fingerprint", {})
    n = fp.get("n", 0)
    k_done = state.get("k_done", 0)
    pct = 100.0 * k_done / n if n else 0.0
    print(f"  group {fp.get('group')}  engine {fp.get('engine')}  r_end {fp.get('r_end')}  seed {state.get('seed')}")
    print(f"  iteration {BOLD}{k_done}/{n}{NC} ({pct:.1f}%)")

    c = state.get("counters", {})
    pairs = c.get("pairs", 0)
    w, d, l = c.get("wins", 0), c.get("draws", 0), c.get("losses", 0)
    disc = c.get("discarded", 0)
    print(f"  pairs {pairs}  discarded {disc}  W/D/L {w}/{d}/{l}  "
          f"time losses {c.get('time_losses', 0)}  illegal {c.get('illegal', 0)}  redraws {c.get('redraws', 0)}")
    if disc and pairs and disc > 0.01 * pairs:
        print(f"  {RED}discarded > 1% of pairs{NC}")
    if c.get("illegal", 0):
        print(f"  {RED}illegal moves recorded{NC}")

    header("Theta of active parameters (vs value in parameters.json)")
    try:
        with open(params_path) as f:
            defs = json.load(f)
    except (OSError, ValueError):
        defs = {}
    theta = state.get("theta", {})
    c_end = fp.get("c_end", {})
    print(f"  {'parameter':40s} {'value':>7s} {'theta':>9s} {'diff':>8s} {'c_end':>6s}  range")
    for p in fp.get("active", []):
        d_ = defs.get(p, {})
        base = d_.get("value")
        t = theta.get(p)
        diff = f"{t - base:+8.2f}" if t is not None and base is not None else f"{'?':>8s}"
        flag = ""
        if t is not None and d_ and t in (d_.get("min"), d_.get("max")):
            flag = f" {RED}on bound{NC}"
        print(f"  {p:40s} {str(base):>7s} {t if t is None else round(t, 2):>9} {diff} "
              f"{str(c_end.get(p)):>6s}  [{d_.get('min')}, {d_.get('max')}]{flag}")

    header(f"History (last {HISTORY_ROWS} rows)")
    if not os.path.exists(history_path):
        print(f"  {YELLOW}no history{NC}")
        return
    with open(history_path, newline="") as f:
        rows = list(csv.reader(f))
    if not rows:
        print(f"  {YELLOW}empty history{NC}")
        return
    print("  " + ", ".join(rows[0]))
    for r in rows[1:][-HISTORY_ROWS:]:
        print("  " + ", ".join(r))


if __name__ == "__main__":
    main()
