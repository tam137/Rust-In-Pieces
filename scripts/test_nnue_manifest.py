#!/usr/bin/env python3
"""Tests for `scripts/nnue_manifest.py`.

Run with `python3 scripts/test_nnue_manifest.py`. Pure standard library.
"""

import hashlib
import json
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import nnue_manifest as nm  # noqa: E402

CONFIG_LINE = (
    'datagen: DatagenConfig { out: "/some/where/part_1.txt", games: 480000, seed: 2000001, '
    'nodes: 5000, random_plies_min: 8, random_plies_max: 12, random_prob: 50, opening_limit: 400, '
    'win_cp: 2000, win_plies: 4, draw_cp: 10, draw_from_move: 40, draw_plies: 8, max_moves: 400, '
    'hash_entries: 1048576, report_every: 5000, sample: 0.33 }'
)
DONE_OLD = (
    "done: games 109000 | dropped 61766 | positions 8275035 | filtered 26.8 % | "
    "W/D/L 41.7/19.6/38.7 % | plies/game 76 | 422 pos/s"
)
REPORT_NEW = (
    "games 415000 | dropped 236791 | positions 10406128 | filtered 26.8 % | sampled out 49.0 % | "
    "W/D/L 41.8/19.4/38.8 % | plies/game 75 | 138 pos/s"
)


def write(path, text):
    with open(path, "w") as f:
        f.write(text)


def read(path):
    with open(path) as f:
        return f.read()


class ParseLines(unittest.TestCase):
    def test_the_rust_debug_config_line_becomes_a_dict(self):
        cfg = nm.parse_config_line(CONFIG_LINE)
        self.assertEqual(cfg["seed"], 2000001)
        self.assertEqual(cfg["sample"], 0.33)
        self.assertEqual(cfg["out"], "part_1.txt")  # file name only, no host path
        self.assertEqual(cfg["nodes"], 5000)

    def test_a_config_line_without_sample_is_the_old_generator(self):
        line = CONFIG_LINE.replace(", sample: 0.33", "")
        self.assertNotIn("sample", nm.parse_config_line(line))

    def test_counter_lines_old_and_new(self):
        old = nm.parse_counter_line(DONE_OLD)
        self.assertTrue(old["done"])
        self.assertEqual((old["games"], old["dropped"], old["positions"]), (109000, 61766, 8275035))
        self.assertEqual(old["wdl_percent"], [41.7, 19.6, 38.7])
        self.assertNotIn("sampled_out_percent", old)
        new = nm.parse_counter_line(REPORT_NEW)
        self.assertFalse(new["done"])
        self.assertEqual(new["sampled_out_percent"], 49.0)
        self.assertEqual(new["plies_per_game"], 75)

    def test_other_lines_are_not_counters(self):
        self.assertIsNone(nm.parse_counter_line("started 12 processes"))


class DataManifest(unittest.TestCase):
    def make_batch(self, root):
        d = os.path.join(root, "batch")
        os.mkdir(d)
        write(os.path.join(d, "part_1.txt"), "a | 1 | 0.5\nb | 2 | 1.0\n")
        write(os.path.join(d, "part_1.log"), CONFIG_LINE + "\n" + DONE_OLD.replace("109000", "2", 1) + "\n")
        write(os.path.join(d, "part_2.txt"), "c | 3 | 0.0\n")
        cfg2 = CONFIG_LINE.replace("2000001", "2000002").replace("part_1", "part_2")
        write(os.path.join(d, "part_2.log"), cfg2 + "\n" + REPORT_NEW + "\n")
        return d

    def test_parts_totals_and_completeness(self):
        with tempfile.TemporaryDirectory() as root:
            d = self.make_batch(root)
            m = nm.data_manifest(d, note="test")
            self.assertEqual(m["kind"], "data")
            self.assertEqual(m["positions"], 3)  # counted lines, not log counters
            self.assertEqual([p["seed"] for p in m["parts"]], [2000001, 2000002])
            self.assertEqual(m["parts"][0]["lines"], 2)
            want = hashlib.sha256(b"c | 3 | 0.0\n").hexdigest()
            self.assertEqual(m["parts"][1]["sha256"], want)
            self.assertFalse(m["complete"])  # part 2 ended without a `done:` line
            self.assertEqual(m["games"], 2 + 415000)
            self.assertEqual(m["settings"]["nodes"], 5000)
            self.assertEqual(m["note"], "test")

    def test_differing_settings_are_reported_per_part(self):
        with tempfile.TemporaryDirectory() as root:
            d = self.make_batch(root)
            log = os.path.join(d, "part_2.log")
            write(log, read(log).replace("nodes: 5000", "nodes: 8000"))
            m = nm.data_manifest(d)
            self.assertIn("nodes", m["settings_differ"])

    def test_a_derived_batch_without_logs_records_its_source(self):
        with tempfile.TemporaryDirectory() as root:
            d = os.path.join(root, "thin")
            os.mkdir(d)
            write(os.path.join(d, "n1_part_1.txt"), "x | 0 | 0.5\n")
            m = nm.data_manifest(d, derived_from=["n1"], rule="keep each line with p = 1/3")
            self.assertEqual(m["positions"], 1)
            self.assertEqual(m["derived_from"], ["n1"])
            self.assertIsNone(m["games"])

    def test_a_directory_without_parts_is_an_error(self):
        with tempfile.TemporaryDirectory() as root:
            with self.assertRaises(ValueError):
                nm.data_manifest(root)


class SetManifest(unittest.TestCase):
    def test_positions_from_the_32_byte_records(self):
        with tempfile.TemporaryDirectory() as root:
            data = os.path.join(root, "x.data")
            with open(data, "wb") as f:
                f.write(b"\0" * 96)
            m = nm.set_manifest(data, sources=[{"name": "s33", "positions": 3}])
            self.assertEqual(m["positions"], 3)
            self.assertEqual(m["sha256"], hashlib.sha256(b"\0" * 96).hexdigest())
            self.assertEqual(m["sources"][0]["name"], "s33")

    def test_a_size_that_is_no_multiple_of_32_is_an_error(self):
        with tempfile.TemporaryDirectory() as root:
            data = os.path.join(root, "x.data")
            with open(data, "wb") as f:
                f.write(b"\0" * 33)
            with self.assertRaises(ValueError):
                nm.set_manifest(data, sources=[])


class NetManifest(unittest.TestCase):
    LOG = (
        "=== Suprah NNUE trainer ===\n"
        "Data            : ../nnue_data/n1_100m.data\n"
        "Superbatches    : 1 to 10 (6104 batches of 16384)\n"
        "Layout          : (768x10 mirrored -> 256)x2 -> 1x8, SCReLU\n"
        "WDL / scale     : 0 / 400\n"
        "Learning rate   : cosine 0.001 -> 2.4300002e-6\n"
        "\x1b[36mBeginning Training\x1b[0m\n"
    )

    def test_the_trainer_header_is_parsed(self):
        h = nm.parse_trainer_log(self.LOG)
        self.assertEqual(h["data"], "n1_100m.data")
        self.assertEqual((h["superbatch_start"], h["superbatch_end"], h["batches_per_superbatch"]), (1, 10, 6104))
        self.assertEqual((h["hidden"], h["wdl"], h["eval_scale"]), (256, 0.0, 400.0))

    def test_net_manifest_hashes_the_checkpoint(self):
        with tempfile.TemporaryDirectory() as root:
            ck = os.path.join(root, "w00", "w00-10")
            os.makedirs(ck)
            with open(os.path.join(ck, "quantised.bin"), "wb") as f:
                f.write(b"net")
            log = os.path.join(root, "w00_run.log")
            write(log, self.LOG)
            m = nm.net_manifest(ck, log_path=log, set_sha256="abc")
            self.assertEqual(m["name"], "w00-10")
            self.assertEqual(m["quantised_sha256"], hashlib.sha256(b"net").hexdigest())
            self.assertEqual(m["training"]["wdl"], 0.0)
            self.assertEqual(m["set_sha256"], "abc")
            self.assertEqual(m["results"], [])

    def test_results_are_appended(self):
        m = {"kind": "net", "results": []}
        nm.add_result(m, "gauntlet", "40.5 % / 30.0 %", when="2026-11-12")
        self.assertEqual(m["results"], [{"kind": "gauntlet", "value": "40.5 % / 30.0 %", "date": "2026-11-12"}])


class Register(unittest.TestCase):
    def test_one_row_per_object(self):
        objs = [
            {"kind": "data", "name": "s33", "positions": 125, "games": 5, "complete": False, "settings": {"sample": 0.33}},
            {"kind": "set", "name": "x.data", "positions": 7, "sources": [{"name": "s33"}], "sha256": "ab" * 32},
            {"kind": "net", "name": "w00-10", "set_name": "x.data", "training": {"wdl": 0.0, "superbatch_end": 10},
             "quantised_sha256": "cd" * 32, "results": [{"kind": "gauntlet", "value": "40 %"}]},
        ]
        text = nm.render_register(objs)
        self.assertIn("| s33 |", text)
        self.assertIn("| x.data |", text)
        self.assertIn("| w00-10 |", text)
        self.assertIn("gauntlet: 40 %", text)


class JsonRoundTrip(unittest.TestCase):
    def test_write_and_read(self):
        with tempfile.TemporaryDirectory() as root:
            path = os.path.join(root, "MANIFEST.json")
            nm.write_json(path, {"kind": "data", "schema": nm.SCHEMA})
            self.assertEqual(json.loads(read(path))["schema"], nm.SCHEMA)


if __name__ == "__main__":
    unittest.main()
