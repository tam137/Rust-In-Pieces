"""Tests for spsa_trajectory.py (Matt-Magie step 99). Run from the repository root:

    python3 -m unittest scripts/test_spsa_trajectory.py
"""
import contextlib
import io
import json
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import spsa_trajectory as st  # noqa: E402

PARAMS = {
    "a": {"value": 10, "min": 0, "max": 20, "c_end": 2},
    "b": {"value": 0, "min": -100, "max": 100, "c_end": 8},
}


def write_run(directory, n_rows, n_total=1000, alert=None, log_rate=None, theta_end=None):
    """A run named `run` in `directory`: state, history with `n_rows` pairs, parameter file."""
    prefix = os.path.join(directory, "run")
    rows = []
    for i in range(1, n_rows + 1):
        # `a` rises by 0.01 per pair, `b` falls by 0.1; k is shuffled like a parallel run.
        rows.append(f"{(i * 7) % n_rows + 1},{i},{1 if i % 3 == 0 else 0},0,1,0,"
                    f"{10 + 0.01 * i:.3f},{-0.1 * i:.3f}")
    with open(prefix + "_history.csv", "w") as f:
        f.write("k,opening,result,w,d,l,a,b\n" + "".join(r + "\n" for r in rows))
    theta = theta_end or {"a": 10 + 0.01 * n_rows, "b": -0.1 * n_rows}
    state = {"version": 2, "k_done": n_rows, "seed": 1, "theta": theta,
             "fingerprint": {"group": "g", "active": ["a", "b"], "n": n_total, "r_end": 0.01,
                             "c_end": {"a": 2, "b": 8}, "engine": "e"},
             "counters": {"pairs": n_rows, "discarded": 0, "wins": 5, "draws": 7, "losses": 3,
                          "time_losses": 0, "illegal": 0, "redraws": 0}}
    with open(prefix + "_state.json", "w") as f:
        json.dump(state, f)
    with open(prefix + "_parameters.json", "w") as f:
        json.dump(PARAMS, f)
    if alert:
        with open(prefix + "_alert.txt", "w") as f:
            f.write(alert + "\n")
    if log_rate:
        with open(prefix + "_spsa.log", "w") as f:
            f.write(f"k 1/{n_total} +0 =1 -0 disc 0 time 0 ill 0 50 pairs/h | a=10\r"
                    f"k 2/{n_total} +0 =2 -0 disc 0 time 0 ill 0 {log_rate} pairs/h | a=10\r")
    return prefix + "_state.json", prefix + "_parameters.json"


def run_main(args):
    out = io.StringIO()
    with contextlib.redirect_stdout(out):
        st.main(args)
    return out.getvalue()


class Checkpoints(unittest.TestCase):
    def test_evenly_spaced_with_the_last_pair_always_included(self):
        self.assertEqual(st.checkpoints(1000, 10), [100, 200, 300, 400, 500, 600, 700, 800, 900, 1000])
        self.assertEqual(st.checkpoints(1005, 10)[-1], 1005)
        self.assertLessEqual(len(st.checkpoints(1005, 10)), 11)

    def test_fewer_pairs_than_rows_gives_one_row_per_pair(self):
        self.assertEqual(st.checkpoints(3, 10), [1, 2, 3])
        self.assertEqual(st.checkpoints(0, 10), [])


class Trajectory(unittest.TestCase):
    def test_each_row_is_the_mean_over_the_window_before_it(self):
        series = [float(i) for i in range(1, 101)]  # value after pair i is i
        rows = st.trajectory(series, st.checkpoints(100, 10))
        self.assertEqual(rows[0], (10, 5.5))       # mean of pairs 1..10
        self.assertEqual(rows[-1], (100, 95.5))    # mean of pairs 91..100

    def test_drift_in_c_end_units(self):
        self.assertAlmostEqual(st.drift(14.0, 10, 2), 2.0)
        self.assertAlmostEqual(st.drift(-12.0, 0, 8), -1.5)


class Report(unittest.TestCase):
    def test_full_report_on_a_synthetic_run(self):
        with tempfile.TemporaryDirectory() as d:
            state, params = write_run(d, 200, alert="b an der Grenze", log_rate=900)
            text = run_main([state, "--param-file", params])
        self.assertIn("200/1000", text)
        self.assertIn("900 pairs/h", text)
        self.assertIn("ALERT", text)
        self.assertIn("b an der Grenze", text)
        self.assertIn("start", text)
        # final line: a moved 2.0 (= 1.00 c_end), b moved -20 (= -2.50 c_end)
        self.assertRegex(text, r"a\s+10\s+12\.00\s+\+2\.00\s+\+1\.00")
        self.assertRegex(text, r"b\s+0\s+-20\.00\s+-20\.00\s+-2\.50")

    def test_a_parameter_on_its_bound_is_flagged(self):
        with tempfile.TemporaryDirectory() as d:
            state, params = write_run(d, 50, theta_end={"a": 20.0, "b": -5.0})
            text = run_main([state, "--param-file", params])
        self.assertIn("on bound", text)
        self.assertIn("no alert", text)

    def test_missing_or_empty_history_does_not_crash(self):
        with tempfile.TemporaryDirectory() as d:
            state, params = write_run(d, 0)
            text = run_main([state, "--param-file", params])
            self.assertIn("no pairs", text)
            os.remove(os.path.join(d, "run_history.csv"))
            text = run_main([state, "--param-file", params])
            self.assertIn("no history", text)


if __name__ == "__main__":
    unittest.main()
