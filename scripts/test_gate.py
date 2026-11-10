"""Tests for gate.py (Matt-Magie step 96). Run from the repository root:

    python3 -m unittest scripts/test_gate.py

The checks against real tournament PGNs run only when GATE_TEST_PGN_DIR names a directory holding
them (`hc95b_sprt.pgn`, step 95b); they are skipped otherwise.
"""
import json
import os
import random
import stat
import sys
import tempfile
import threading
import time
import unittest

os.environ["GATE_POLL"] = "0.05"
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gate  # noqa: E402

RULES = {"min": 100, "max": 1000, "elo0": 0.0, "elo1": 5.0, "guard": 10.0}

FAKE_MM = r'''#!/usr/bin/env python3
"""Fake mm.sh: plays the .trn's pairing(s) with results drawn from FAKE_P (candidate win rate)."""
import os, random, re, sys, time
trn = dict(l.split(" = ", 1) for l in open(sys.argv[2]).read().splitlines() if " = " in l and not l.startswith("#"))
engines = [e.strip() for e in trn["engines"].split(",")]
names = {e: open("engines/" + e).read().split("id name ")[1].split("'")[0].strip() for e in engines}
p = float(os.environ.get("FAKE_P", "0.6")); rng = random.Random(int(os.environ.get("FAKE_SEED", "1")))
rounds = int(trn["rounds"]); pgn = trn["pgn"]
opps = engines[1:]
total = rounds * 2 * len(opps)
n = 0
for r in range(rounds):
    for o in opps:
        for swap in (False, True):
            n += 1
            cand_white = not swap
            u = rng.random()
            cand_score = 1 if u < p * 0.6 else 0.5 if u < p * 0.6 + 0.4 else 0
            res = {1: "1-0", 0.5: "1/2-1/2", 0: "0-1"}[cand_score] if cand_white else {1: "0-1", 0.5: "1/2-1/2", 0: "1-0"}[cand_score]
            w, b = (names[engines[0]], names[o]) if cand_white else (names[o], names[engines[0]])
            with open(pgn, "a") as f:
                f.write(f'[Event "x"]\n[Round "{n}/{total}"]\n[White "{w}"]\n[Black "{b}"]\n[Result "{res}"]\n[Termination "Normal"]\n\n1. e4 {res}\n\n')
            time.sleep(float(os.environ.get("FAKE_DELAY", "0.002")))
'''


def make_mm(tmp, engines):
    os.makedirs(os.path.join(tmp, "engines"))
    for fname, idname in engines.items():
        p = os.path.join(tmp, "engines", fname)
        with open(p, "w") as f:
            f.write(f"#!/bin/sh\necho 'id name {idname}'\necho uciok\n")
        os.chmod(p, os.stat(p).st_mode | stat.S_IEXEC)
    p = os.path.join(tmp, "mm.sh")
    with open(p, "w") as f:
        f.write(FAKE_MM)
    os.chmod(p, os.stat(p).st_mode | stat.S_IEXEC)
    open(os.path.join(tmp, "openings_wide.txt"), "w").write("e2e4\n")


def write_pgn(p, games):
    """games: (white, black, result) in play order, numbered consecutively."""
    with open(p, "w") as f:
        for i, (w, b, r) in enumerate(games, 1):
            f.write(f'[Event "x"]\n[Round "{i}/{len(games)}"]\n[White "{w}"]\n[Black "{b}"]\n'
                    f'[Result "{r}"]\n[Termination "Normal"]\n\n1. e4 {r}\n\n')


class Decide(unittest.TestCase):
    def test_never_stops_below_min(self):
        self.assertIsNone(gate.decide(99, RULES, 0, 0, 30))

    def test_main_test_verdicts(self):
        self.assertEqual(gate.decide(100, RULES, 0, 2, 3), ("[0, 5] H1", True))
        self.assertEqual(gate.decide(100, RULES, 1, 2, 3), ("[0, 5] H0", False))

    def test_guard_h1_ships_and_guard_h0_rejects_only_at_elo_le_0(self):
        self.assertEqual(gate.decide(500, RULES, 2, 0, 8), ("[0, 10] H1", True))
        self.assertIsNone(gate.decide(500, RULES, 2, 1, 1.5))
        self.assertEqual(gate.decide(500, RULES, 2, 1, -0.8)[1], False)

    def test_fixed_stop_at_max(self):
        self.assertEqual(gate.decide(1000, RULES, 2, 2, 1), ("fixed stop at 1000 games", False))

    def test_no_guard(self):
        self.assertIsNone(gate.decide(500, dict(RULES, guard=None), 2, 0, 8))


class Evaluate(unittest.TestCase):
    def test_reported_from_the_candidates_side_even_when_it_sorts_second(self):
        # "Z-cand" sorts after "A-base"; it wins every pair 1.5-0.5.
        games = []
        for _ in range(50):
            games += [("Z-cand", "A-base", "1-0"), ("A-base", "Z-cand", "1/2-1/2")]
        with tempfile.TemporaryDirectory() as tmp:
            p = os.path.join(tmp, "a.pgn")
            write_pgn(p, games)
            ev = gate.evaluate([p], "Z-cand", "A-base", 0, 5, 10)
        self.assertEqual((ev["w"], ev["d"], ev["l"], ev["games"], ev["pairs"]), (50, 50, 0, 100, 50))
        self.assertGreater(ev["elo"], 100)
        self.assertEqual(ev["counts"], [0, 0, 0, 50, 0])

    def test_pairs_are_formed_per_file(self):
        with tempfile.TemporaryDirectory() as tmp:
            a, b = os.path.join(tmp, "a.pgn"), os.path.join(tmp, "b.pgn")
            write_pgn(a, [("C", "B", "1-0"), ("B", "C", "0-1")])
            write_pgn(b, [("C", "B", "0-1"), ("B", "C", "1-0")])
            ev = gate.evaluate([a, b], "C", "B", 0, 5, None)
        self.assertEqual((ev["pairs"], ev["counts"]), (2, [1, 0, 0, 0, 1]))

    @unittest.skipUnless(os.environ.get("GATE_TEST_PGN_DIR"), "GATE_TEST_PGN_DIR not set")
    def test_step_95b_sprt_reproduces_its_published_reading(self):
        p = os.path.join(os.environ["GATE_TEST_PGN_DIR"], "hc95b_sprt.pgn")
        ev = gate.evaluate([p], "Rust-In-Pieces V0.52.0-rc2", "Rust-In-Pieces V0.51.0", 0, 5, 10)
        self.assertEqual(ev["games"], 15716)
        self.assertAlmostEqual(ev["elo"], 5.4, delta=0.1)
        self.assertAlmostEqual(ev["lo"], 0.9, delta=0.6)
        self.assertEqual(ev["code"], 0, "[0, 5] H1 at 15716 games")
        self.assertEqual(gate.decide(ev["games"], dict(RULES, min=3000, max=20000), ev["code"],
                                     ev["code_guard"], ev["elo"]), ("[0, 5] H1", True))


class EndToEnd(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.mm = self._tmp.name
        make_mm(self.mm, {"cand": "Engine V2-rc", "base": "Engine V1", "old": "Engine V0"})
        os.environ["FAKE_P"] = "0.62"

    def tearDown(self):
        self._tmp.cleanup()

    def start(self, *extra):
        gate.main(["--mm", self.mm, "start", "t", "--cand", "cand", "--base", "base",
                   "--no-census", "--foreground", "--ltc-games", "40", *extra])

    def test_full_run_with_gauntlet_sprt_and_control(self):
        self.start("--opponents", "old", "--min", "200", "--max", "4000")
        st = gate.state_of(self.mm, "t")
        self.assertEqual(st["phase"], "finished", open(gate.path(self.mm, "t", ".gate.status")).read())
        self.assertIn("gauntlet passed", st["done"]["gauntlet"])
        self.assertIn("H1", st["done"]["sprt"])
        self.assertIn("passed", st["done"]["ltc"])
        lines = "\n".join(gate.status_lines(self.mm, "t"))
        self.assertIn("Engine V2-rc against Engine V1", lines)

    def test_a_weaker_candidate_halts_at_the_sprt(self):
        os.environ["FAKE_P"] = "0.30"
        self.start("--no-gauntlet", "--min", "200", "--max", "4000")
        st = gate.state_of(self.mm, "t")
        self.assertEqual(st["phase"], "halted:sprt")
        self.assertNotIn("ltc", st["done"])

    def test_rules_are_re_read_while_the_sprt_runs(self):
        os.environ["FAKE_DELAY"] = "0.004"
        try:
            # min 20000 would never stop on its own; `set --min 200` must end the SPRT.
            def later():
                time.sleep(1.0)
                gate.main(["--mm", self.mm, "set", "t", "--min", "200"])
            threading.Thread(target=later).start()
            self.start("--no-gauntlet", "--no-ltc", "--min", "20000", "--max", "20000")
        finally:
            os.environ["FAKE_DELAY"] = "0.002"
        st = gate.state_of(self.mm, "t")
        self.assertEqual(st["phase"], "finished")
        log = open(gate.path(self.mm, "t", ".gate.log")).read()
        self.assertIn("min 20000", log)
        self.assertIn("min 200 ", log)
        games = int(st["done"]["sprt"].split(" after ")[1].split()[0])
        self.assertLess(games, 20000)

    def test_resume_skips_a_failed_gauntlet(self):
        os.environ["FAKE_P"] = "0.2"
        self.start("--no-sprt", "--no-ltc")
        self.assertEqual(gate.state_of(self.mm, "t")["phase"], "halted:gauntlet")
        gate.main(["--mm", self.mm, "resume", "t", "--skip", "gauntlet", "--foreground"])
        st = gate.state_of(self.mm, "t")
        self.assertEqual(st["phase"], "finished")
        self.assertIn("skipped by the owner", st["done"]["gauntlet"])

    def test_start_refuses_equal_id_names_and_too_many_workers(self):
        twin = os.path.join(self.mm, "engines", "twin")
        with open(twin, "w") as f:
            f.write("#!/bin/sh\necho 'id name Engine V1'\necho uciok\n")
        os.chmod(twin, 0o755)
        with self.assertRaises(SystemExit):
            gate.main(["--mm", self.mm, "start", "u", "--cand", "twin", "--base", "base", "--foreground"])
        with self.assertRaises(SystemExit):
            gate.main(["--mm", self.mm, "start", "u", "--cand", "cand", "--base", "cand", "--foreground"])
        with self.assertRaises(SystemExit):
            gate.main(["--mm", self.mm, "start", "v", "--cand", "cand", "--base", "base",
                       "--workers", "13", "--foreground"])

    def test_archive_packs_checks_and_deletes(self):
        self.start("--no-gauntlet", "--no-sprt", "--no-ltc")
        gate.main(["--mm", self.mm, "archive", "t"])
        self.assertFalse([f for f in os.listdir(self.mm) if f.startswith("t.") or f.startswith("t_")])
        self.assertEqual(len(os.listdir(os.path.join(self.mm, "archive"))), 1)


if __name__ == "__main__":
    unittest.main()
