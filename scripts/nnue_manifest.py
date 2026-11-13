#!/usr/bin/env python3
"""Provenance manifests for NNUE training data, training sets and networks.

Every object of the NNUE pipeline gets a small JSON description so that a network can always be
traced back to the exact data it was trained on:

- a **data batch** (a directory of `part_<i>.txt` written by `suprah datagen`, plus the
  `part_<i>.log` counters): settings, seeds, game/position counters, line count and sha256 per
  file, or - for a batch derived from other batches - its sources and the derivation rule;
- a **training set** (a shuffled bullet `.data` file): its source batches, record count, sha256;
- a **network** (a trainer checkpoint directory): training settings parsed from the trainer log,
  the training set's sha256, the sha256 of `quantised.bin`, and results added later.

`register` renders all manifests below a match-manager directory into one markdown table.

Usage:
    scripts/nnue_manifest.py data <batch dir> [--derived-from NAME ...] [--rule TEXT] [--note TEXT]
                                  [--binary FILE | --binary-sha256 HASH] [--engine-commit HASH] [--arch NAME]
    scripts/nnue_manifest.py set <file.data> --source <batch dir> ... [--tool TEXT] [--note TEXT]
    scripts/nnue_manifest.py net <checkpoint dir> --log <trainer log> --set <file.data>
                                 [--trainer-commit HASH] [--engine NAME] [--note TEXT]
    scripts/nnue_manifest.py result <checkpoint dir> <kind> <value> [--date YYYY-MM-DD]
    scripts/nnue_manifest.py register <mm dir> [--out FILE]

Manifests are written next to the object: `<batch dir>/MANIFEST.json`, `<file.data>.json`,
`<checkpoint dir>/MANIFEST.json`. Paths are given on the command line; nothing is hard-coded.
Pure standard library.
"""

import argparse
import datetime
import glob
import hashlib
import json
import os
import re
import sys

SCHEMA = 1
RECORD_BYTES = 32  # bullet's ChessBoard record
ANSI = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")


# --- parsing ----------------------------------------------------------------------------------

def parse_config_line(line):
    """`datagen: DatagenConfig { key: value, ... }` (Rust Debug) -> dict; `out` keeps the file name."""
    body = line[line.index("{") + 1: line.rindex("}")]
    cfg = {}
    for key, value in re.findall(r'(\w+): ("(?:[^"\\]|\\.)*"|[^,]+)', body):
        value = value.strip()
        if value.startswith('"'):
            value = value[1:-1]
        else:
            try:
                value = int(value)
            except ValueError:
                value = float(value)
        cfg[key] = value
    if "out" in cfg:
        cfg["out"] = os.path.basename(cfg["out"])
    return cfg


def parse_counter_line(line):
    """A datagen progress or final line -> dict, or None for other lines."""
    line = line.strip()
    done = line.startswith("done: ")
    text = line[len("done: "):] if done else line
    if not text.startswith("games "):
        return None
    out = {"done": done}
    patterns = {
        "games": r"games (\d+)",
        "dropped": r"dropped (\d+)",
        "positions": r"positions (\d+)",
        "filtered_percent": r"filtered ([\d.]+) %",
        "sampled_out_percent": r"sampled out ([\d.]+) %",
        "plies_per_game": r"(?<!book )plies/game (\d+)",
        "book_plies_per_game": r"book plies/game ([\d.]+)",
        "positions_per_second": r"(\d+) pos/s",
    }
    for key, pattern in patterns.items():
        m = re.search(pattern, text)
        if m:
            out[key] = float(m.group(1)) if "." in m.group(1) else int(m.group(1))
    m = re.search(r"W/D/L ([\d.]+)/([\d.]+)/([\d.]+) %", text)
    if m:
        out["wdl_percent"] = [float(x) for x in m.groups()]
    return out


def parse_trainer_log(text):
    """Header of a `suprah-nnue-trainer` run log -> training settings."""
    text = ANSI.sub("", text)
    fields = {}
    for line in text.splitlines():
        if ":" in line:
            key, _, value = line.partition(":")
            fields.setdefault(key.strip(), value.strip())
    out = {}
    if "Data" in fields:
        out["data"] = [os.path.basename(p.strip()) for p in fields["Data"].split(",")]
        if len(out["data"]) == 1:
            out["data"] = out["data"][0]
    m = re.match(r"(\d+) to (\d+) \((\d+) batches of (\d+)\)", fields.get("Superbatches", ""))
    if m:
        out["superbatch_start"], out["superbatch_end"], out["batches_per_superbatch"], out["batch_size"] = (
            int(g) for g in m.groups())
    m = re.search(r"-> (\d+)\)x2", fields.get("Layout", ""))
    if m:
        out["hidden"] = int(m.group(1))
    if "Layout" in fields:
        out["layout"] = fields["Layout"]
    m = re.match(r"([\d.]+) / ([\d.]+)", fields.get("WDL / scale", ""))
    if m:
        out["wdl"], out["eval_scale"] = float(m.group(1)), float(m.group(2))
    if "Learning rate" in fields:
        out["learning_rate"] = fields["Learning rate"]
    if "Resume from" in fields:
        out["resumed_from"] = fields["Resume from"]
    return out


# --- helpers ----------------------------------------------------------------------------------

def sha256_file(path, chunk=1 << 22):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        while True:
            block = f.read(chunk)
            if not block:
                break
            h.update(block)
    return h.hexdigest()


def count_lines(path, chunk=1 << 22):
    n = 0
    with open(path, "rb") as f:
        while True:
            block = f.read(chunk)
            if not block:
                break
            n += block.count(b"\n")
    return n


def mtime_iso(path):
    return datetime.datetime.fromtimestamp(os.path.getmtime(path)).isoformat(timespec="seconds")


def write_json(path, obj):
    tmp = path + ".tmp"
    with open(tmp, "w") as f:
        json.dump(obj, f, indent=2, ensure_ascii=False)
        f.write("\n")
    os.replace(tmp, path)


def read_json(path):
    with open(path) as f:
        return json.load(f)


def part_index(path):
    m = re.search(r"(\d+)\.txt$", path)
    return int(m.group(1)) if m else 0


# --- manifests --------------------------------------------------------------------------------

def data_manifest(batch_dir, derived_from=None, rule=None, note=None, binary=None, engine_commit=None,
                  arch=None, binary_sha256=None):
    """Describe a data batch directory (parts written by `suprah datagen` or derived from others)."""
    texts = sorted(glob.glob(os.path.join(batch_dir, "*.txt")), key=lambda p: (part_index(p), p))
    if not texts:
        raise ValueError(f"{batch_dir}: no *.txt parts")
    parts, settings_seen = [], {}
    total_games, any_log, complete = 0, False, True
    for txt in texts:
        part = {"file": os.path.basename(txt), "lines": count_lines(txt), "sha256": sha256_file(txt),
                "modified": mtime_iso(txt)}
        log = txt[:-4] + ".log"
        if os.path.exists(log):
            any_log = True
            counters, cfg = None, None
            with open(log) as f:
                for line in f:
                    if line.startswith("datagen: DatagenConfig"):
                        cfg = parse_config_line(line)
                    c = parse_counter_line(line)
                    if c:
                        counters = c
            if cfg:
                part["seed"] = cfg.get("seed")
                for key, value in cfg.items():
                    if key not in ("out", "seed", "games", "report_every"):
                        settings_seen.setdefault(key, set()).add(value)
                part["games_target"] = cfg.get("games")
            if counters:
                part["counters"] = counters
                total_games += counters.get("games", 0)
                complete = complete and counters["done"]
            else:
                complete = False
        parts.append(part)

    settings = {k: next(iter(v)) for k, v in settings_seen.items() if len(v) == 1}
    differ = sorted(k for k, v in settings_seen.items() if len(v) > 1)
    manifest = {
        "schema": SCHEMA,
        "kind": "data",
        "name": os.path.basename(os.path.normpath(batch_dir)),
        "created": datetime.datetime.now().isoformat(timespec="seconds"),
        "positions": sum(p["lines"] for p in parts),
        "games": total_games if any_log else None,
        "complete": complete if any_log else None,
        "settings": settings,
        "settings_differ": differ,
        "parts": parts,
    }
    if any_log:
        dropped = sum(p.get("counters", {}).get("dropped", 0) for p in parts)
        manifest["dropped_games"] = dropped
        manifest["counters_note"] = ("games/dropped come from each part's last log line; for an "
                                     "interrupted part that is the last progress report")
    if derived_from:
        manifest["derived_from"] = list(derived_from)
        manifest["rule"] = rule
    if binary or binary_sha256:
        manifest["datagen_binary_sha256"] = binary_sha256 or sha256_file(binary)
    if engine_commit:
        manifest["engine_commit"] = engine_commit
    if arch:
        manifest["arch"] = arch
    if note:
        manifest["note"] = note
    return manifest


def set_manifest(data_path, sources, tool=None, note=None):
    """Describe a shuffled bullet `.data` training set."""
    size = os.path.getsize(data_path)
    if size % RECORD_BYTES:
        raise ValueError(f"{data_path}: {size} bytes is no multiple of {RECORD_BYTES}")
    manifest = {
        "schema": SCHEMA,
        "kind": "set",
        "name": os.path.basename(data_path),
        "created": datetime.datetime.now().isoformat(timespec="seconds"),
        "file_modified": mtime_iso(data_path),
        "positions": size // RECORD_BYTES,
        "sha256": sha256_file(data_path),
        "sources": sources,
    }
    if tool:
        manifest["tool"] = tool
    if note:
        manifest["note"] = note
    return manifest


def net_manifest(checkpoint_dir, log_path, set_sha256=None, set_name=None, trainer_commit=None, engine=None,
                 note=None):
    """Describe a trainer checkpoint (one network)."""
    quantised = os.path.join(checkpoint_dir, "quantised.bin")
    with open(log_path, errors="replace") as f:
        training = parse_trainer_log(f.read())
    manifest = {
        "schema": SCHEMA,
        "kind": "net",
        "name": os.path.basename(os.path.normpath(checkpoint_dir)),
        "created": datetime.datetime.now().isoformat(timespec="seconds"),
        "quantised_sha256": sha256_file(quantised),
        "quantised_bytes": os.path.getsize(quantised),
        "checkpoint_modified": mtime_iso(quantised),
        "training": training,
        "set_name": set_name or training.get("data"),
        "set_sha256": set_sha256,
        "results": [],
    }
    if trainer_commit:
        manifest["trainer_commit"] = trainer_commit
    if engine:
        manifest["engine"] = engine
    if note:
        manifest["note"] = note
    return manifest


def add_result(manifest, kind, value, when=None):
    manifest.setdefault("results", []).append(
        {"kind": kind, "value": value, "date": when or datetime.date.today().isoformat()})


# --- register ---------------------------------------------------------------------------------

def fmt_count(n):
    if n is None:
        return "–"
    return f"{n / 1e6:.1f} M" if n >= 1e6 else str(n)


def render_register(objs):
    data = [o for o in objs if o.get("kind") == "data"]
    sets = [o for o in objs if o.get("kind") == "set"]
    nets = [o for o in objs if o.get("kind") == "net"]
    lines = ["# NNUE register", "", "Generated by `scripts/nnue_manifest.py register`; edit the manifests, "
             "not this file.", "", "## Data batches", "",
             "| name | positions | games | complete | sample | derived from | note |",
             "| :--- | ---: | ---: | :--- | :--- | :--- | :--- |"]
    for o in data:
        s = o.get("settings", {})
        lines.append(f"| {o['name']} | {fmt_count(o.get('positions'))} | {fmt_count(o.get('games'))} | "
                     f"{o.get('complete')} | {s.get('sample', 1.0)} | {', '.join(o.get('derived_from', [])) or '–'} | "
                     f"{o.get('note', '')} |")
    lines += ["", "## Training sets", "", "| name | positions | sources | sha256 | note |",
              "| :--- | ---: | :--- | :--- | :--- |"]
    for o in sets:
        src = ", ".join(s["name"] for s in o.get("sources", []))
        lines.append(f"| {o['name']} | {fmt_count(o.get('positions'))} | {src} | {o.get('sha256', '')[:12]} | "
                     f"{o.get('note', '')} |")
    lines += ["", "## Networks", "", "| name | set | WDL | superbatches | quantised sha256 | results | note |",
              "| :--- | :--- | ---: | ---: | :--- | :--- | :--- |"]
    for o in nets:
        t = o.get("training", {})
        results = "; ".join(f"{r['kind']}: {r['value']}" for r in o.get("results", []))
        lines.append(f"| {o['name']} | {o.get('set_name', '')} | {t.get('wdl', '')} | {t.get('superbatch_end', '')} | "
                     f"{o.get('quantised_sha256', '')[:12]} | {results} | {o.get('note', '')} |")
    return "\n".join(lines) + "\n"


def collect(mm_dir):
    objs = []
    for path in sorted(glob.glob(os.path.join(mm_dir, "nnue_data", "*", "MANIFEST.json"))):
        objs.append(read_json(path))
    for path in sorted(glob.glob(os.path.join(mm_dir, "nnue_data", "*.data.json"))):
        objs.append(read_json(path))
    for path in sorted(glob.glob(os.path.join(mm_dir, "nnue_nets", "**", "MANIFEST.json"), recursive=True)):
        objs.append(read_json(path))
    return objs


# --- command line -----------------------------------------------------------------------------

def main(argv=None):
    p = argparse.ArgumentParser(description="Provenance manifests for NNUE data, sets and networks.")
    sub = p.add_subparsers(dest="cmd", required=True)

    d = sub.add_parser("data")
    d.add_argument("dir")
    d.add_argument("--derived-from", action="append")
    d.add_argument("--rule")
    d.add_argument("--note")
    d.add_argument("--binary")
    d.add_argument("--binary-sha256", help="hash of a datagen binary that is not on this machine")
    d.add_argument("--engine-commit")
    d.add_argument("--arch")

    s = sub.add_parser("set")
    s.add_argument("data")
    s.add_argument("--source", action="append", required=True, help="batch directory with a MANIFEST.json")
    s.add_argument("--tool")
    s.add_argument("--note")

    n = sub.add_parser("net")
    n.add_argument("checkpoint")
    n.add_argument("--log", required=True)
    n.add_argument("--set", required=True, help="the .data file (its .data.json must exist)")
    n.add_argument("--trainer-commit")
    n.add_argument("--engine")
    n.add_argument("--note")

    r = sub.add_parser("result")
    r.add_argument("checkpoint")
    r.add_argument("kind")
    r.add_argument("value")
    r.add_argument("--date")

    g = sub.add_parser("register")
    g.add_argument("mm")
    g.add_argument("--out")

    a = p.parse_args(argv)
    if a.cmd == "data":
        m = data_manifest(a.dir, a.derived_from, a.rule, a.note, a.binary, a.engine_commit, a.arch,
                          a.binary_sha256)
        write_json(os.path.join(a.dir, "MANIFEST.json"), m)
        print(f"{m['name']}: {m['positions']} positions, games {m['games']}, complete {m['complete']}")
    elif a.cmd == "set":
        sources = []
        for src in a.source:
            sm = read_json(os.path.join(src, "MANIFEST.json"))
            sources.append({"name": sm["name"], "positions": sm["positions"], "games": sm.get("games"),
                            "manifest_sha256": sha256_file(os.path.join(src, "MANIFEST.json"))})
        m = set_manifest(a.data, sources, a.tool, a.note)
        write_json(a.data + ".json", m)
        print(f"{m['name']}: {m['positions']} positions from {len(sources)} batches")
    elif a.cmd == "net":
        sm = read_json(a.set + ".json")
        m = net_manifest(a.checkpoint, a.log, sm["sha256"], sm["name"], a.trainer_commit, a.engine, a.note)
        write_json(os.path.join(a.checkpoint, "MANIFEST.json"), m)
        print(f"{m['name']}: WDL {m['training'].get('wdl')}, set {m['set_name']}")
    elif a.cmd == "result":
        path = os.path.join(a.checkpoint, "MANIFEST.json")
        m = read_json(path)
        add_result(m, a.kind, a.value, a.date)
        write_json(path, m)
        print(f"{m['name']}: {len(m['results'])} results")
    elif a.cmd == "register":
        out = a.out or os.path.join(a.mm, "nnue_register.md")
        with open(out, "w") as f:
            f.write(render_register(collect(a.mm)))
        print(f"wrote {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
