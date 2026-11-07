"""Tests for spsa_tuner.py (Matt-Magie step 94). No engine needed: games come from fake runners.

Run from the repository root:  python3 -m unittest tuning/test_spsa_tuner.py
"""
import csv
import json
import math
import os
import random
import stat
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import spsa_tuner as st  # noqa: E402


def defs_fixture():
    return {
        "zero": {"value": 0, "min": -64, "max": 64, "c_end": 8},
        "big": {"value": 300, "min": 50, "max": 800, "c_end": 20},
        "idle": {"value": 7, "min": 0, "max": 20},
    }


WIN_PLUS = [("1-0", "WhiteWin"), ("0-1", "BlackWin")]      # theta+ wins both
LOSS_PLUS = [("0-1", "BlackWin"), ("1-0", "WhiteWin")]     # theta+ loses both
DRAWS = [("1/2-1/2", "Draw"), ("1/2-1/2", "Draw")]


class FakeRunner:
    """Decides each game from a callable on (white_values, black_values)."""

    def __init__(self, decide):
        self.decide = decide
        self.calls = []

    def play_pair(self, job):
        self.calls.append(job)
        return [self.decide(job.plus, job.minus), self.decide(job.minus, job.plus)]

    def stop(self):
        pass


class Base(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = Path(self._tmp.name)

    def tearDown(self):
        self._tmp.cleanup()

    def make(self, defs=None, active=("zero", "big"), n=100, r_end=0.002, seed=1, group="g", openings=10):
        defs = defs or defs_fixture()
        return st.SPSATuner(defs, list(active), n, r_end, seed, group,
                            str(self.tmp / "state.json"), str(self.tmp / "hist.csv"), "suprah-x", openings)


class Schedules(unittest.TestCase):
    def test_schedule_matches_hand_computed_values(self):
        s = st.Schedule(c_end=8, r_end=0.002, n_iters=1000)
        c = 8 * 1000 ** 0.101
        a = 0.002 * 64 * (100.0 + 1000) ** 0.602
        for k in (1, 500, 1000):
            self.assertAlmostEqual(s.c_k(k), c / k ** 0.101)
            self.assertAlmostEqual(s.a_k(k), a / (100.0 + k) ** 0.602)
        self.assertAlmostEqual(s.c_k(1000), 8)               # the perturbation ends at c_end
        self.assertAlmostEqual(s.step(1000), 0.002 * 8)      # the step ends at r_end * c_end
        self.assertTrue(s.step(1) > s.step(500) > s.step(1000))
        self.assertTrue(s.c_k(1) > s.c_k(500) > s.c_k(1000))


class Updates(Base):
    def test_a_parameter_at_zero_is_perturbed_by_its_absolute_step(self):
        t = self.make()
        job = t.next_job()
        ck = t.sched["zero"].c_k(1)
        self.assertEqual(job.plus["zero"], round(ck * job.delta["zero"]))
        self.assertEqual(job.minus["zero"], round(-ck * job.delta["zero"]))
        self.assertGreaterEqual(abs(job.plus["zero"]), 8)   # far from the old step of 1

    def test_a_won_pair_moves_theta_by_step_times_result_along_delta(self):
        t = self.make()
        job = t.next_job()
        self.assertTrue(t.apply(job, WIN_PLUS))
        for p in ("zero", "big"):
            expected = defs_fixture()[p]["value"] + t.sched[p].step(1) * 2 * job.delta[p]
            self.assertAlmostEqual(t.theta[p], expected)
        self.assertNotEqual(t.theta["zero"], 0)

    def test_a_lost_pair_moves_the_other_way_and_draws_do_not_move(self):
        t = self.make()
        job = t.next_job()
        t.apply(job, LOSS_PLUS)
        self.assertAlmostEqual(t.theta["zero"], -t.sched["zero"].step(1) * 2 * job.delta["zero"])
        before = dict(t.theta)
        t.apply(t.next_job(), DRAWS)
        self.assertEqual(t.theta, before)

    def test_a_split_pair_is_a_zero_result(self):
        t = self.make()
        before = dict(t.theta)
        t.apply(t.next_job(), [("1-0", ""), ("1-0", "")])   # plus wins as White, loses as Black
        self.assertEqual(t.theta, before)
        self.assertEqual((t.counters["wins"], t.counters["losses"]), (1, 1))

    def test_inactive_parameters_stay_at_default_and_are_sent_at_default(self):
        t = self.make()
        for _ in range(20):
            job = t.next_job()
            self.assertEqual((job.plus["idle"], job.minus["idle"]), (7, 7))
            t.apply(job, WIN_PLUS)
        self.assertEqual(t.theta["idle"], 7)

    def test_perturbation_and_theta_are_clamped(self):
        defs = defs_fixture()
        defs["zero"]["value"] = 64
        t = self.make(defs=defs)
        for _ in range(50):
            job = t.next_job()
            self.assertTrue(-64 <= job.plus["zero"] <= 64 and -64 <= job.minus["zero"] <= 64)
            t.apply(job, WIN_PLUS if job.delta["zero"] > 0 else LOSS_PLUS)
        self.assertLessEqual(t.theta["zero"], 64)

    def test_identical_rounded_pairs_are_redrawn(self):
        # Two active parameters, one pinned so that its plus and minus collide: the pair
        # differs in the other. With both pinned no draw can help and it is an error.
        defs = {"a": {"value": 5, "min": 5, "max": 9, "c_end": 1},
                "b": {"value": 5, "min": 5, "max": 9, "c_end": 1}}
        t = self.make(defs=defs, active=("a", "b"))
        job = t.next_job()
        self.assertTrue(any(job.plus[p] != job.minus[p] for p in ("a", "b")))

    def test_validation_refuses_missing_or_small_c_end(self):
        with self.assertRaisesRegex(st.ConfigError, "idle: no c_end"):
            self.make(active=("idle",))
        defs = defs_fixture()
        defs["zero"]["c_end"] = 0.5
        with self.assertRaisesRegex(st.ConfigError, r"c_end 0.5 < 1"):
            self.make(defs=defs, active=("zero",))
        defs = defs_fixture()
        defs["zero"]["c_end"] = 40
        with self.assertRaisesRegex(st.ConfigError, "narrower than 4"):
            self.make(defs=defs, active=("zero",))
        with self.assertRaisesRegex(st.ConfigError, "not in parameters.json"):
            self.make(active=("nope",))

    def test_a_value_on_a_bound_is_a_warning(self):
        defs = defs_fixture()
        defs["big"]["value"] = 800
        t = self.make(defs=defs)
        self.assertTrue(any("big" in w and "bound" in w for w in t.warnings))


class Pairs(Base):
    def test_pair_plays_one_opening_with_colours_swapped(self):
        calls = []

        class Rec(st.MattMagieRunner):
            def _game(self, tag, white_opts, black_opts, opening):
                calls.append((white_opts, black_opts, opening))
                return ("1-0", "")

        r = Rec.__new__(Rec)
        r.openings, r.hash_mb, r.logpath = ["e2e4 e7e5", "d2d4"], 16, ""
        r.play_pair(st.Job(1, {"x": 1}, {"x": 3}, {"x": 1}, 1))
        self.assertEqual(calls[0], ("x=3,Hash=16", "x=1,Hash=16", "d2d4"))
        self.assertEqual(calls[1], ("x=1,Hash=16", "x=3,Hash=16", "d2d4"))

    def test_result_is_counted_from_theta_plus_for_both_colours(self):
        t = self.make()
        t.apply(t.next_job(), [("1-0", ""), ("1/2-1/2", "")])   # plus wins as White, draws as Black
        t.apply(t.next_job(), [("1/2-1/2", ""), ("1-0", "")])   # plus loses as Black
        self.assertEqual((t.counters["wins"], t.counters["draws"], t.counters["losses"]), (1, 2, 1))

    def test_missing_result_discards_the_pair(self):
        t = self.make()
        before = dict(t.theta)
        self.assertFalse(t.apply(t.next_job(), [("1-0", ""), (None, None)]))
        self.assertEqual(t.theta, before)
        self.assertEqual(t.counters["discarded"], 1)

    def test_time_losses_and_illegal_moves_are_counted(self):
        t = self.make()
        t.apply(t.next_job(), [("1-0", "WhiteWinByTime"), ("1-0", "WhiteWinByIllegalMove")])
        self.assertEqual((t.counters["time_losses"], t.counters["illegal"]), (1, 1))

    def test_parse_pgn(self):
        text = '[Event "x"]\n[Result "0-1"]\n[Termination "BlackWinByTime"]\n\n1. e4 0-1\n'
        self.assertEqual(st.parse_pgn(text), ("0-1", "BlackWinByTime"))
        self.assertEqual(st.parse_pgn('[Result "*"]'), (None, None))
        self.assertEqual(st.parse_pgn(""), (None, None))


class Runs(Base):
    def test_run_aborts_after_three_failed_pairs_in_a_row(self):
        t = self.make(n=50)
        with self.assertRaisesRegex(RuntimeError, "in a row"):
            st.run(t, FakeRunner(lambda w, b: (None, None)), workers=1, progress=False)

    def test_run_aborts_above_one_percent_discarded(self):
        t = self.make(n=400)
        n = {"i": 0}

        def decide(w, b):
            n["i"] += 1
            return (None, None) if n["i"] % 40 == 0 else ("1/2-1/2", "")   # every 20th pair

        with self.assertRaisesRegex(RuntimeError, "discarded"):
            st.run(t, FakeRunner(decide), workers=1, progress=False)

    def test_run_aborts_above_two_percent_time_losses(self):
        t = self.make(n=200)
        n = {"i": 0}

        def decide(w, b):
            n["i"] += 1
            return ("1-0", "WhiteWinByTime") if n["i"] % 20 == 0 else ("1/2-1/2", "")   # 5% of games

        with self.assertRaisesRegex(RuntimeError, "time losses"):
            st.run(t, FakeRunner(decide), workers=1, progress=False)

    def test_run_plays_exactly_n_pairs_with_parallel_workers(self):
        t = self.make(n=37)
        runner = FakeRunner(lambda w, b: ("1/2-1/2", ""))
        st.run(t, runner, workers=4, progress=False)
        self.assertEqual((len(runner.calls), t.k_done), (37, 37))
        self.assertEqual(sorted(j.k for j in runner.calls), list(range(1, 38)))


class State(Base):
    def test_resume_continues_with_the_same_theta_and_counters(self):
        t = self.make()
        for _ in range(10):
            t.apply(t.next_job(), WIN_PLUS)
        t2 = self.make()
        for p in t.theta:
            self.assertAlmostEqual(t2.theta[p], t.theta[p])
        self.assertEqual((t2.k_done, t2.k_next), (10, 11))
        self.assertEqual(t2.counters, t.counters)
        with open(self.tmp / "hist.csv") as f:
            rows = list(csv.reader(f))
        self.assertEqual(rows[0][:6], ["k", "opening", "result", "w", "d", "l"])
        self.assertEqual(len(rows), 11)

    def test_in_flight_pairs_are_not_counted_as_done(self):
        t = self.make()
        jobs = [t.next_job() for _ in range(4)]
        t.apply(jobs[2], DRAWS)
        t2 = self.make()
        self.assertEqual((t2.k_done, t2.k_next), (1, 2))

    def test_resume_refuses_a_changed_run(self):
        self.make()
        defs = defs_fixture()
        defs["zero"]["c_end"] = 4
        with self.assertRaisesRegex(st.ConfigError, "c_end"):
            self.make(defs=defs)
        with self.assertRaisesRegex(st.ConfigError, r"\bn\b"):
            self.make(n=200)

    def test_a_fresh_run_keeps_the_history_of_an_earlier_one(self):
        with open(self.tmp / "hist.csv", "w") as f:
            f.write("old\n")
        self.make()
        backups = list(self.tmp.glob("hist.csv.*.bak"))
        self.assertEqual(len(backups), 1)
        self.assertEqual(backups[0].read_text(), "old\n")

    def test_resume_refuses_another_start(self):
        st.SPSATuner(defs_fixture(), ["zero"], 100, 0.002, 1, "g", str(self.tmp / "s.json"),
                     str(self.tmp / "h.csv"), "e", 1, "zero=10")
        with self.assertRaisesRegex(st.ConfigError, "start"):
            st.SPSATuner(defs_fixture(), ["zero"], 100, 0.002, 1, "g", str(self.tmp / "s.json"),
                         str(self.tmp / "h.csv"), "e", 1, "")

    def test_a_version_1_state_is_moved_aside(self):
        with open(self.tmp / "state.json", "w") as f:
            json.dump({"k": 2, "theta": {"zero": 3}}, f)
        t = self.make()
        self.assertTrue((self.tmp / "state.v1.json").exists())
        self.assertEqual((t.theta["zero"], t.k_next), (0, 1))


class Convergence(Base):
    def test_synthetic_convergence_from_six_c_end(self):
        """Each game is won by the side closer to x* with a probability rising with the distance
        gap. From 6 * c_end away, 2000 pairs must end within 1.5 * c_end of x*, five seeds."""
        x_star, c_end = 0.0, 8
        for seed in range(1, 6):
            with self.subTest(seed=seed):
                for f in ("state.json", "hist.csv"):
                    if (self.tmp / f).exists():
                        os.remove(self.tmp / f)
                defs = {"p": {"value": 6 * c_end, "min": -200, "max": 200, "c_end": c_end}}
                rng = random.Random(seed)

                def decide(white, black):
                    gap = abs(black["p"] - x_star) - abs(white["p"] - x_star)
                    p_white = min(0.95, max(0.05, 0.5 + 0.004 * gap))
                    u = rng.random()
                    if u < 0.3:
                        return ("1/2-1/2", "")
                    return ("1-0", "") if rng.random() < p_white else ("0-1", "")

                t = self.make(defs=defs, active=("p",), n=2000, r_end=0.02, seed=seed)
                st.run(t, FakeRunner(decide), workers=1, progress=False)
                self.assertLessEqual(abs(t.theta["p"] - x_star), 1.5 * c_end, t.theta["p"])


class Preflight(Base):
    def fake_engine(self, unknown=()):
        script = self.tmp / "engine.py"
        script.write_text(f"""#!{sys.executable}
import os, sys
log = None
unknown = {list(unknown)!r}
for line in sys.stdin:
    parts = line.split()
    if not parts: continue
    if parts[0] == "uci": print("uciok", flush=True)
    elif parts[0] == "isready": print("readyok", flush=True)
    elif parts[0] == "quit": break
    elif parts[0] == "setoption":
        name, value = parts[2], parts[4]
        if name == "logpath":
            log = open(os.path.join(value, "engine_1.log"), "a")
        msg = ("Unknown option ignored: " if name in unknown else "Received option: ") + name + " = " + value
        if name == "Hash": msg = "Hash size dynamically set to " + value + " MB (1 entries)"
        if log: log.write(msg + "\\n"); log.flush()
""")
        script.chmod(script.stat().st_mode | stat.S_IEXEC)
        return str(script)

    def test_preflight_confirms_every_option(self):
        self.assertEqual(st.preflight(self.fake_engine(), defs_fixture(), ["zero"], 16), 4)

    def test_preflight_times_out_on_a_silent_engine(self):
        script = self.tmp / "silent.py"
        script.write_text(f"#!{sys.executable}\nimport time\ntime.sleep(60)\n")
        script.chmod(script.stat().st_mode | stat.S_IEXEC)
        with self.assertRaisesRegex(st.ConfigError, "no readyok"):
            st.preflight(str(script), defs_fixture(), ["zero"], 16, timeout=1)

    def test_preflight_refuses_an_unknown_option(self):
        with self.assertRaisesRegex(st.ConfigError, "big: unknown"):
            st.preflight(self.fake_engine(unknown=("big",)), defs_fixture(), ["zero"], 16)


if __name__ == "__main__":
    unittest.main()
