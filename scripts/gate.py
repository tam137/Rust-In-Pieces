#!/usr/bin/env python3
"""One acceptance run for a candidate engine: census, smoke gauntlet, SPRT, control at a longer
time control - with stop rules that can be changed while it runs and a compact status.

Usage (the Matt-Magie working directory `<mm>` is given by --mm or MM_DIR):

    scripts/gate.py start <name> --cand <engine> --base <engine> [options]
    scripts/gate.py set <name> [--min N] [--max N] [--elo0 E] [--elo1 E] [--guard E]
    scripts/gate.py status [<name>]
    scripts/gate.py stop <name>
    scripts/gate.py resume <name> [--skip census|gauntlet|sprt|ltc]
    scripts/gate.py archive <name>

Engines are file names in `<mm>/engines/`. The run lives in tmux `gate-<name>`; its files are
`<mm>/<name>.gate.json` (rules, written by start and set), `<name>.gate.state.json` (progress,
written by the run), `<name>.gate.log` (one line per minute in the SPRT) and `<name>_<stage>.*`.

Stages (each can be switched off): the census (time to depth 10 against the base, information
only), the smoke gauntlet (1s + 100ms, 100 games per pairing, >= 45% against every opponent), the
SPRT (default [0, 5] with a [0, 10] guard at 1s + 150ms; no stop below --min; then a verdict of the
main test, H1 of the guard, or H0 of the guard with Elo <= 0; at --max a fixed stop) and the
control at a longer time control (default 10s + 100ms, 3000 games; a paired 95% interval entirely
below 0 blocks). A stage that fails halts the run; `resume` continues it. The run re-reads the rules
every minute, so `set` changes a running SPRT without touching the tournament
(`skills/matchplay_measurement_procedure.md`).

Everything is reported from the candidate's side.
"""

import argparse
import glob
import json
import math
import os
import re
import shutil
import signal
import subprocess
import sys
import tarfile
import tempfile
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import sprt  # noqa: E402
from pairing_elo import parse_games, score_for, elo  # noqa: E402

STAGES = ("census", "gauntlet", "sprt", "ltc")
MAX_WORKERS = 12          # owner's rule for this host class: 12 games at a time
MAX_GAMES = 20000         # the cap of every SPRT
ALPHA = BETA = 0.05
LOWER = math.log(BETA / (1.0 - ALPHA))
UPPER = math.log((1.0 - BETA) / ALPHA)
POLL = float(os.environ.get("GATE_POLL", 60))


# ------------------------------------------------------------------------------------------------
# Files
# ------------------------------------------------------------------------------------------------

def mm_dir(args_mm):
    mm = args_mm or os.environ.get("MM_DIR")
    if not mm:
        sys.exit("give the Matt-Magie working directory with --mm or MM_DIR")
    if not os.path.isfile(os.path.join(mm, "mm.sh")):
        sys.exit(f"{mm}: no mm.sh there")
    return os.path.abspath(mm)


def path(mm, name, suffix):
    return os.path.join(mm, f"{name}{suffix}")


def read_json(p, default=None):
    try:
        with open(p) as f:
            return json.load(f)
    except (OSError, ValueError):
        return default


def write_json(p, data):
    tmp = p + ".tmp"
    with open(tmp, "w") as f:
        json.dump(data, f, indent=2)
    os.replace(tmp, p)


def rules_of(mm, name):
    r = read_json(path(mm, name, ".gate.json"))
    if r is None:
        sys.exit(f"no run '{name}' in {mm}")
    return r


def state_of(mm, name):
    return read_json(path(mm, name, ".gate.state.json"), {"phase": "pending", "done": {}})


def save_state(mm, name, state):
    write_json(path(mm, name, ".gate.state.json"), state)


def stamp():
    return time.strftime("%m-%d %H:%M")


def note(mm, name, text):
    with open(path(mm, name, ".gate.status"), "a") as f:
        f.write(f"{stamp()} {text}\n")


# ------------------------------------------------------------------------------------------------
# Measurement
# ------------------------------------------------------------------------------------------------

def evaluate(pgns, cand, base, elo0, elo1, guard):
    """Paired result of `cand` against `base` over `pgns`, from the candidate's side."""
    pgns = [p for p in pgns if os.path.isfile(p)]
    out = {"games": 0, "pairs": 0, "w": 0, "d": 0, "l": 0, "time_losses": 0, "illegal": 0,
           "elo": None, "lo": None, "hi": None, "counts": [0] * 5,
           "llr": None, "llr_guard": None, "code": 2, "code_guard": 2}
    if not pgns:
        return out
    for p in pgns:
        for white, black, result, _ in parse_games(p):
            if {white, black} != {cand, base}:
                continue
            s = score_for(white, black, result, cand)
            if s is None:
                continue
            out["games"] += 1
            out["w" if s == 1 else "d" if s == 0.5 else "l"] += 1
        with open(p, errors="ignore") as f:
            text = f.read()
        out["time_losses"] += len(re.findall(r'\[Termination "\w*ByTime"\]', text))
        out["illegal"] += len(re.findall(r'\[Termination "\w*ByIllegalMove"\]', text))
    pairs = sprt.collect_pairs(pgns, cand, base)
    n = len(pairs)
    out["pairs"] = n
    if n >= 2:
        m = sum(pairs) / n
        se = math.sqrt(sum((x - m) ** 2 for x in pairs) / (n - 1) / n)
        clamp = lambda s: min(max(s, 1e-9), 1 - 1e-9)
        out["elo"], out["lo"], out["hi"] = elo(clamp(m)), elo(clamp(m - 1.96 * se)), elo(clamp(m + 1.96 * se))
        out["counts"] = sprt.bucket(pairs)
        out["llr"] = sprt.log_likelihood_ratio(out["counts"], elo0, elo1)
        out["llr_guard"] = sprt.log_likelihood_ratio(out["counts"], elo0, guard) if guard else None
        out["code"] = sprt.verdict(out["llr"], LOWER, UPPER)[1]
        out["code_guard"] = sprt.verdict(out["llr_guard"], LOWER, UPPER)[1] if guard else 2
    return out


def decide(games, rules, code, code_guard, elo_value):
    """Stop rule of the SPRT. Returns (verdict, passed) or None to keep playing."""
    if games < rules["min"]:
        return None
    if games >= rules["max"]:
        return (f"fixed stop at {rules['max']} games", False)
    e0, e1, g = rules["elo0"], rules["elo1"], rules.get("guard")
    if code == 0:
        return (f"[{e0:g}, {e1:g}] H1", True)
    if code == 1:
        return (f"[{e0:g}, {e1:g}] H0", False)
    if g and code_guard == 0:
        return (f"[{e0:g}, {g:g}] H1", True)
    if g and code_guard == 1 and elo_value is not None and elo_value <= 0:
        return (f"[{e0:g}, {g:g}] H0 with Elo {elo_value:+.1f} <= 0", False)
    return None


def id_name(binary, timeout=20):
    """The engine's `id name`. `quit` is sent only after `uciok`: an engine may handle a `quit` that
    arrives with the `uci` before it has answered."""
    try:
        proc = subprocess.Popen([binary], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                stderr=subprocess.DEVNULL, text=True)
    except OSError:
        return None
    name, deadline = None, time.time() + timeout
    try:
        proc.stdin.write("uci\n")
        proc.stdin.flush()
        while time.time() < deadline:
            line = proc.stdout.readline()
            if not line:
                break
            if line.startswith("id name "):
                name = line[len("id name "):].strip()
            if line.strip() == "uciok":
                break
        proc.stdin.write("quit\n")
        proc.stdin.flush()
    except (BrokenPipeError, OSError):
        pass
    finally:
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait()
    return name


def fmt(v, digits=1):
    return "n/a" if v is None else f"{v:+.{digits}f}"


# ------------------------------------------------------------------------------------------------
# Tournaments
# ------------------------------------------------------------------------------------------------

def write_trn(mm, name, stage, engines, tc, rounds, mode, workers, hash_mb, comment):
    base_ms, inc_ms = (int(x) for x in tc.split("+"))
    p = path(mm, name, f"_{stage}.trn")
    with open(p, "w") as f:
        f.write(f"# gate.py {name}, stage {stage}: {comment}\n")
        f.write(f"engines = {', '.join(engines)}\ntime_control = {base_ms}\nincrement = {inc_ms}\n"
                f"openings = openings_wide.txt\nrounds = {rounds}\npgn = {name}_{stage}.pgn\n"
                f"mode = {mode}\nengine_options = Hash={hash_mb}, Threads=1, OwnBook=false\n"
                f"concurrency = {workers}\n")
    return os.path.basename(p)


def alive(pgid):
    try:
        os.killpg(pgid, 0)
        return True
    except (ProcessLookupError, PermissionError, TypeError):
        return False


def kill_group(pgid):
    for sig in (signal.SIGTERM, signal.SIGKILL):
        try:
            os.killpg(pgid, sig)
        except ProcessLookupError:
            return
        for _ in range(20):
            if not alive(pgid):
                return
            time.sleep(0.5)


def run_tournament(mm, name, stage, trn, on_poll=None):
    """Plays `trn` in its own process group (or re-attaches to a running one); `on_poll` may return
    a verdict, which ends the tournament. Returns the verdict or None when it ended by itself."""
    state = state_of(mm, name)
    pgid = state.get("pgid") if state.get("phase") == stage else None
    if not (pgid and alive(pgid)):
        with open(path(mm, name, f"_{stage}.out"), "a") as out:
            proc = subprocess.Popen(["./mm.sh", "-t", trn], cwd=mm, stdout=out, stderr=out,
                                    start_new_session=True)
        pgid = proc.pid
        state = state_of(mm, name)
        state["pgid"] = pgid
        save_state(mm, name, state)
    else:
        proc = None
    while alive(pgid):
        if proc is not None and proc.poll() is not None:
            break
        time.sleep(POLL)
        if on_poll:
            v = on_poll()
            if v:
                kill_group(pgid)
                return v
    if proc is not None:
        proc.wait()
    return None


# ------------------------------------------------------------------------------------------------
# Stages
# ------------------------------------------------------------------------------------------------

def stage_census(mm, name, r):
    script = os.path.join(os.path.dirname(os.path.abspath(__file__)), "measure_tree_size.py")
    procs, lines = [], []
    for pool in ("wide", "mixed"):
        out = open(path(mm, name, f"_census_{pool}.txt"), "w")
        procs.append((pool, out, subprocess.Popen(
            [sys.executable, script, os.path.join(mm, "engines", r["base"]),
             os.path.join(mm, "engines", r["cand"]), "--openings",
             os.path.join(mm, f"openings_{pool}.txt"), "--positions", "300", "--depth", "10"],
            stdout=out, stderr=subprocess.STDOUT)))
    for pool, out, p in procs:
        p.wait()
        out.close()
        text = open(path(mm, name, f"_census_{pool}.txt")).read()
        t = re.search(r"^Time\s+(\d+) ms -> (\d+) ms", text, re.M)
        n = re.search(r"^Nodes\s+(\d+) -> (\d+)", text, re.M)
        if t and n:
            tb, tc = int(t.group(1)), int(t.group(2))
            nb, nc = int(n.group(1)), int(n.group(2))
            lines.append(f"{pool}: time {100 * (tc - tb) / tb:+.1f}%, generated moves {100 * (nc - nb) / nb:+.1f}%")
        else:
            lines.append(f"{pool}: no result (see {name}_census_{pool}.txt)")
    return True, "census (candidate against base, depth 10): " + "; ".join(lines)


def stage_gauntlet(mm, name, r):
    opponents = [r["base"]] + r.get("opponents", [])
    trn = write_trn(mm, name, "gauntlet", [r["cand"]] + opponents, r["tc_gauntlet"], 50, "gauntlet",
                    r["workers"], r["hash"], ">= 45% against every opponent, 100 games each")
    run_tournament(mm, name, "gauntlet", trn)
    pgn = path(mm, name, "_gauntlet.pgn")
    cand = r["cand_id"]
    parts, ok = [], True
    for opp in opponents:
        oid = r["ids"][opp]
        ev = evaluate([pgn], cand, oid, 0, 5, None)
        if not ev["games"]:
            parts.append(f"{opp}: no games"); ok = False; continue
        pct = 100 * (ev["w"] + 0.5 * ev["d"]) / ev["games"]
        ok &= pct >= 45
        parts.append(f"{opp} {pct:.1f}% of {ev['games']}")
    return ok, f"gauntlet {'passed' if ok else 'FAILED (< 45%)'}: " + ", ".join(parts)


def stage_sprt(mm, name, r0):
    trn = write_trn(mm, name, "sprt", [r0["cand"], r0["base"]], r0["tc_sprt"], MAX_GAMES // 2,
                    "round_robin", r0["workers"], r0["hash"], "SPRT, rules in the .gate.json")
    pgn = path(mm, name, "_sprt.pgn")
    result = {}

    def poll():
        rules = rules_of(mm, name)                      # re-read every minute
        ev = evaluate([pgn], rules["cand_id"], rules["base_id"], rules["elo0"], rules["elo1"], rules.get("guard"))
        with open(path(mm, name, ".gate.log"), "a") as f:
            f.write(f"{time.strftime('%H:%M:%S')} games {ev['games']} Elo {fmt(ev['elo'])} "
                    f"[{fmt(ev['lo'])}, {fmt(ev['hi'])}] LLR {fmt(ev['llr'], 2)} guard {fmt(ev['llr_guard'], 2)} "
                    f"(min {rules['min']} max {rules['max']})\n")
        d = decide(ev["games"], rules, ev["code"], ev["code_guard"], ev["elo"])
        if d:
            result["verdict"], result["passed"], result["ev"] = d[0], d[1], ev
        return d
    v = run_tournament(mm, name, "sprt", trn, poll)
    rules = rules_of(mm, name)
    ev = result.get("ev") or evaluate([pgn], rules["cand_id"], rules["base_id"], rules["elo0"], rules["elo1"], rules.get("guard"))
    verdict = result.get("verdict", "tournament ended without a verdict")
    passed = result.get("passed", False)
    return passed, (f"SPRT {verdict} after {ev['games']} games: Elo {fmt(ev['elo'])}, paired 95% "
                    f"[{fmt(ev['lo'])}, {fmt(ev['hi'])}], time losses {ev['time_losses']}")


def stage_ltc(mm, name, r):
    trn = write_trn(mm, name, "ltc", [r["cand"], r["base"]], r["tc_ltc"], max(1, r["ltc_games"] // 2),
                    "round_robin", r["workers"], r["hash"], "control at a longer time control")
    run_tournament(mm, name, "ltc", trn)
    ev = evaluate([path(mm, name, "_ltc.pgn")], r["cand_id"], r["base_id"], 0, 5, None)
    blocked = ev["hi"] is not None and ev["hi"] < 0
    ok = ev["games"] > 0 and not blocked
    return ok, (f"control {r['tc_ltc']} {'BLOCKS' if blocked else 'passed' if ok else 'no games'}: "
                f"{ev['games']} games, Elo {fmt(ev['elo'])}, paired 95% [{fmt(ev['lo'])}, {fmt(ev['hi'])}], "
                f"time losses {ev['time_losses']}")


STAGE_FN = {"census": stage_census, "gauntlet": stage_gauntlet, "sprt": stage_sprt, "ltc": stage_ltc}


def run(mm, name):
    for stage in STAGES:
        r = rules_of(mm, name)
        state = state_of(mm, name)
        if not r["stages"].get(stage) or stage in state["done"]:
            continue
        if state.get("phase") != stage:
            state.update({"phase": stage, "phase_started": time.time(), "pgid": None})
            save_state(mm, name, state)
            note(mm, name, f"{stage} start")
        ok, summary = STAGE_FN[stage](mm, name, r)
        state = state_of(mm, name)
        note(mm, name, summary)
        if not ok:
            state.update({"phase": f"halted:{stage}", "pgid": None, "halt": summary})
            save_state(mm, name, state)
            note(mm, name, f"HALT at {stage}: the owner decides (resume, or resume --skip {stage})")
            return
        state["done"][stage] = summary
        state["pgid"] = None
        save_state(mm, name, state)
    state = state_of(mm, name)
    state["phase"] = "finished"
    save_state(mm, name, state)
    note(mm, name, "FINISHED - release only on the owner's word")


# ------------------------------------------------------------------------------------------------
# Commands
# ------------------------------------------------------------------------------------------------

def launch(mm, name):
    session = f"gate-{name}"
    if subprocess.run(["tmux", "has-session", "-t", session], capture_output=True).returncode == 0:
        sys.exit(f"tmux {session} is already running")
    subprocess.run(["tmux", "new-session", "-d", "-s", session, sys.executable,
                    os.path.abspath(__file__), "--mm", mm, "run", name], check=True)


def cmd_start(mm, a):
    if os.path.exists(path(mm, a.name, ".gate.json")):
        sys.exit(f"a run '{a.name}' exists already; archive it or choose another name")
    if a.workers > MAX_WORKERS:
        sys.exit(f"--workers {a.workers}: at most {MAX_WORKERS} games at a time")
    if not a.elo0 < a.elo1 or (a.guard and not a.elo0 < a.guard):
        sys.exit("need elo0 < elo1 and elo0 < guard")
    if not 0 < a.min <= a.max <= MAX_GAMES:
        sys.exit(f"need 0 < min <= max <= {MAX_GAMES}")
    engines = [a.cand, a.base] + a.opponents
    if len(set(engines)) != len(engines):
        sys.exit("candidate, base and opponents must be different engines")
    ids = {}
    for e in engines:
        b = os.path.join(mm, "engines", e)
        if not os.access(b, os.X_OK):
            sys.exit(f"{b}: not an executable engine")
        ids[e] = id_name(b)
        if not ids[e]:
            sys.exit(f"{e}: no 'id name' on uci")
    if len(set(ids.values())) != len(ids):
        sys.exit(f"engines must report distinct id names, got {ids} - tag the candidate's version first")
    stages = {s: not getattr(a, f"no_{s}") for s in STAGES}
    rules = {"name": a.name, "cand": a.cand, "base": a.base, "opponents": a.opponents, "ids": ids,
             "cand_id": ids[a.cand], "base_id": ids[a.base], "stages": stages,
             "elo0": a.elo0, "elo1": a.elo1, "guard": a.guard, "min": a.min, "max": a.max,
             "tc_sprt": a.tc, "tc_gauntlet": a.tc_gauntlet, "tc_ltc": a.ltc, "ltc_games": a.ltc_games,
             "workers": a.workers, "hash": a.hash, "created": stamp()}
    write_json(path(mm, a.name, ".gate.json"), rules)
    save_state(mm, a.name, {"phase": "pending", "done": {}})
    note(mm, a.name, f"start: {ids[a.cand]} against {ids[a.base]}, stages "
         f"{', '.join(s for s in STAGES if stages[s])}, SPRT [{a.elo0:g}, {a.elo1:g}] guard {a.guard} "
         f"min {a.min} max {a.max}")
    if a.foreground:
        run(mm, a.name)
    else:
        launch(mm, a.name)
        print(f"started in tmux gate-{a.name}; status: scripts/gate.py status {a.name}")


def cmd_set(mm, a):
    r = rules_of(mm, a.name)
    changes = {k: getattr(a, k) for k in ("min", "max", "elo0", "elo1", "guard") if getattr(a, k) is not None}
    if not changes:
        sys.exit("nothing to set")
    new = dict(r, **changes)
    if not 0 < new["min"] <= new["max"] <= MAX_GAMES:
        sys.exit(f"need 0 < min <= max <= {MAX_GAMES}")
    if not new["elo0"] < new["elo1"] or (new["guard"] and not new["elo0"] < new["guard"]):
        sys.exit("need elo0 < elo1 and elo0 < guard")
    write_json(path(mm, a.name, ".gate.json"), new)
    text = ", ".join(f"{k} {r.get(k)} -> {v}" for k, v in changes.items())
    note(mm, a.name, f"set: {text} (read by the run within a minute)")
    print(f"{a.name}: {text}")


def status_lines(mm, name):
    r = rules_of(mm, name)
    st = state_of(mm, name)
    phase = st.get("phase", "pending")
    since = time.strftime("%H:%M", time.localtime(st["phase_started"])) if st.get("phase_started") else "-"
    lines = [f"{name}  phase {phase}  since {since}  {r['cand_id']} against {r['base_id']}"]
    for s in STAGES:
        if s in st["done"]:
            lines.append(f"  done  {st['done'][s]}")
    if phase.startswith("halted"):
        lines.append(f"  HALT  {st.get('halt', '')}")
    stage = phase if phase in ("gauntlet", "sprt", "ltc") else None
    if stage in ("sprt", "ltc"):
        ev = evaluate([path(mm, name, f"_{stage}.pgn")], r["cand_id"], r["base_id"], r["elo0"], r["elo1"], r.get("guard"))
        tc = r["tc_sprt"] if stage == "sprt" else r["tc_ltc"]
        cap = r["max"] if stage == "sprt" else r["ltc_games"]
        lines.append(f"  {stage} {tc}: games {ev['games']}"
                     + (f" (min {r['min']}, max {r['max']})" if stage == "sprt" else f" of {r['ltc_games']}")
                     + f"   +{ev['w']} ={ev['d']} -{ev['l']}   time losses {ev['time_losses']}")
        lines.append(f"  Elo {fmt(ev['elo'])}   paired 95% [{fmt(ev['lo'])}, {fmt(ev['hi'])}]   "
                     f"pairs 0/0.5/1/1.5/2: {'/'.join(map(str, ev['counts']))}")
        if stage == "sprt":
            g = r.get("guard")
            lines.append(f"  LLR [{r['elo0']:g}, {r['elo1']:g}] {fmt(ev['llr'], 2)} of +-{UPPER:.2f}"
                         + (f"   guard [{r['elo0']:g}, {g:g}] {fmt(ev['llr_guard'], 2)}" if g else ""))
        if st.get("phase_started") and ev["games"]:
            rate = ev["games"] / max(1.0, time.time() - st["phase_started"])
            left = max(0, cap - ev["games"]) / rate
            what = "the cap" if stage == "sprt" else "the end"
            lines.append(f"  {rate * 60:.0f} games/min; {what} in about {left / 3600:.1f} h")
    elif stage == "gauntlet":
        n = sum(1 for _ in parse_games(path(mm, name, "_gauntlet.pgn"))) if os.path.isfile(path(mm, name, "_gauntlet.pgn")) else 0
        lines.append(f"  gauntlet: {n} of {100 * (1 + len(r.get('opponents', [])))} games")
    return lines


def cmd_status(mm, a):
    names = [a.name] if a.name else sorted(
        os.path.basename(p)[:-len(".gate.json")] for p in glob.glob(os.path.join(mm, "*.gate.json")))
    if not names:
        print("no gate runs")
    for n in names:
        print("\n".join(status_lines(mm, n)))


def cmd_stop(mm, a):
    st = state_of(mm, a.name)
    if st.get("pgid"):
        kill_group(st["pgid"])
    subprocess.run(["tmux", "kill-session", "-t", f"gate-{a.name}"], capture_output=True)
    st.update({"phase": f"halted:{st.get('phase')}", "pgid": None, "halt": "stopped by the owner"})
    save_state(mm, a.name, st)
    note(mm, a.name, "stopped by the owner")


def cmd_resume(mm, a):
    st = state_of(mm, a.name)
    phase = st.get("phase", "")
    if phase.startswith("halted:"):
        st["phase"] = phase.split(":", 1)[1]
    if a.skip:
        st["done"][a.skip] = f"{a.skip} skipped by the owner"
        note(mm, a.name, f"resume, {a.skip} skipped by the owner")
    else:
        note(mm, a.name, f"resume at {st['phase']}")
    st.pop("halt", None)
    save_state(mm, a.name, st)
    if a.foreground:
        run(mm, a.name)
    else:
        launch(mm, a.name)


def cmd_archive(mm, a):
    st = state_of(mm, a.name)
    if st.get("pgid") and alive(st["pgid"]):
        sys.exit("the run is still playing; stop it first")
    files = sorted(set(glob.glob(path(mm, a.name, ".gate*")) + glob.glob(path(mm, a.name, "_*"))))
    if not files:
        sys.exit("nothing to archive")
    os.makedirs(os.path.join(mm, "archive"), exist_ok=True)
    target = os.path.join(mm, "archive", f"{a.name}-{time.strftime('%Y-%m-%d')}.tar.xz")
    with tarfile.open(target, "w:xz") as t:
        for f in files:
            t.add(f, arcname=os.path.basename(f))
    tmp = tempfile.mkdtemp()
    try:
        with tarfile.open(target) as t:
            t.extractall(tmp, filter="data")
        for f in files:
            if os.path.isfile(f) and open(f, "rb").read() != open(os.path.join(tmp, os.path.basename(f)), "rb").read():
                sys.exit(f"archive check failed for {f}; nothing deleted")
    finally:
        shutil.rmtree(tmp)
    for f in files:
        shutil.rmtree(f) if os.path.isdir(f) else os.remove(f)
    print(f"{len(files)} files archived to {target} and checked; originals deleted")


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--mm", default=None, help="Matt-Magie working directory (or MM_DIR)")
    sub = ap.add_subparsers(dest="cmd", required=True)
    s = sub.add_parser("start")
    s.add_argument("name")
    s.add_argument("--cand", required=True)
    s.add_argument("--base", required=True)
    s.add_argument("--opponents", default="", type=lambda v: [x for x in v.split(",") if x])
    s.add_argument("--elo0", type=float, default=0.0)
    s.add_argument("--elo1", type=float, default=5.0)
    s.add_argument("--guard", type=float, default=10.0, help="second alternative; 0 switches it off")
    s.add_argument("--min", type=int, default=2500)
    s.add_argument("--max", type=int, default=MAX_GAMES)
    s.add_argument("--tc", default="1000+150", help="SPRT time control, ms+ms")
    s.add_argument("--tc-gauntlet", default="1000+100")
    s.add_argument("--ltc", default="10000+100", help="control time control, ms+ms")
    s.add_argument("--ltc-games", type=int, default=3000)
    s.add_argument("--workers", type=int, default=12)
    s.add_argument("--hash", type=int, default=64)
    for st in STAGES:
        s.add_argument(f"--no-{st}", action="store_true")
    s.add_argument("--foreground", action="store_true", help="run here instead of in tmux")
    t = sub.add_parser("set")
    t.add_argument("name")
    for k in ("min", "max"):
        t.add_argument(f"--{k}", type=int)
    for k in ("elo0", "elo1", "guard"):
        t.add_argument(f"--{k}", type=float)
    st = sub.add_parser("status")
    st.add_argument("name", nargs="?")
    for c in ("stop", "archive", "run"):
        sub.add_parser(c).add_argument("name")
    rs = sub.add_parser("resume")
    rs.add_argument("name")
    rs.add_argument("--skip", choices=STAGES)
    rs.add_argument("--foreground", action="store_true")
    a = ap.parse_args(argv)
    mm = mm_dir(a.mm)
    if a.cmd == "start" and a.guard == 0:
        a.guard = None
    {"start": cmd_start, "set": cmd_set, "status": cmd_status, "stop": cmd_stop, "resume": cmd_resume,
     "archive": cmd_archive, "run": lambda m, x: run(m, x.name)}[a.cmd](mm, a)


if __name__ == "__main__":
    main()
