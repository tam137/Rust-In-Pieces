#!/usr/bin/env python3
"""Tests for the node accounting of `scripts/uci_driver.py`.

Run with `python3 scripts/test_uci_driver.py`. Pure standard library, no engine needed.
"""

import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import uci_driver  # noqa: E402


def result(engine_id, nodes):
    """A search whose iterations reported `nodes`, one value per completed depth."""
    info = [f"info depth {d} score cp 10 time {d} nodes {n} nps 1 hashfull 0 pv e2e4"
            for d, n in enumerate(nodes, start=2)]
    reached, score, signature = uci_driver.parse_info(info)
    return uci_driver.SearchResult(reached, score, "e2e4", 1, "", signature, engine_id)


class NodesFromGo(unittest.TestCase):
    def test_the_versions_that_count_from_go(self):
        self.assertFalse(uci_driver.counts_nodes_from_go("RIP V0.54.4-NNUE"))
        self.assertTrue(uci_driver.counts_nodes_from_go("RIP V0.54.5-NNUE"))
        self.assertTrue(uci_driver.counts_nodes_from_go("RIP V0.54.5-rc-NNUE"))
        self.assertFalse(uci_driver.counts_nodes_from_go("RIP V0.53.0-NNUE"))
        self.assertFalse(uci_driver.counts_nodes_from_go("Rust-In-Pieces V0.52.2"))
        self.assertTrue(uci_driver.counts_nodes_from_go("Rust-In-Pieces V0.52.3"))
        self.assertTrue(uci_driver.counts_nodes_from_go("Stockfish 17"))

    def test_the_versions_that_count_visited_positions(self):
        self.assertEqual(uci_driver.node_unit("RIP V0.54.6-NNUE"), "generated")
        self.assertEqual(uci_driver.node_unit("RIP V0.54.7-NNUE"), "visited")
        self.assertEqual(uci_driver.node_unit("RIP V0.54.7-rc-NNUE"), "visited")
        self.assertEqual(uci_driver.node_unit("Rust-In-Pieces V0.52.4"), "generated")
        self.assertEqual(uci_driver.node_unit("Rust-In-Pieces V0.52.5"), "visited")
        self.assertEqual(uci_driver.node_unit("Stockfish 17"), "visited")

    def test_an_old_build_and_a_new_one_report_the_same_total(self):
        # The same three iterations of 100, 300 and 900 nodes, printed both ways.
        old = result("RIP V0.54.4-NNUE", [100, 300, 900])
        new = result("RIP V0.54.5-NNUE", [100, 400, 1300])
        self.assertEqual(uci_driver.nodes_from_go(old), 1300)
        self.assertEqual(uci_driver.nodes_from_go(new), 1300)

    def test_strip_nodes_makes_the_two_countings_identical(self):
        old = result("RIP V0.54.4-NNUE", [100, 300, 900])
        new = result("RIP V0.54.5-NNUE", [100, 400, 1300])
        self.assertNotEqual(old.info_signature, new.info_signature)
        self.assertEqual(uci_driver.strip_nodes(old.info_signature), uci_driver.strip_nodes(new.info_signature))


if __name__ == "__main__":
    unittest.main(verbosity=2)
