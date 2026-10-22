#!/usr/bin/env python3
"""Tests for the parsing in `measure_order_tt.py`."""
import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import measure_order_tt as m  # noqa: E402

ORDER_LINE = ("SEARCHDIAGORDER cutoffs=100 first=80 index=80,10,4,2,2,1,1 class=50,30,0,10,10 "
              "by_depth=0,60,40 first_by_depth=0,50,30 late_after_tt=5")
TT_LINE = ("SEARCHDIAGTT probe_main=10 hit_main=4 probe_qs=20 hit_qs=5 eval_main=8 eval_main_hit=2 "
           "eval_qs=30 eval_qs_hit=6 store=1,2,3,4,5 evict_deeper=1 evict_age=1,1,1,1 "
           "evict_deeper_age=1,0,0,0 sample_empty=2 sample_age=1,1,0,0")


class Parsing(unittest.TestCase):
    def test_lists_and_scalars(self):
        f = m.fields(ORDER_LINE.split(" ", 1)[1])
        self.assertEqual(f["cutoffs"], 100)
        self.assertEqual(f["index"], [80, 10, 4, 2, 2, 1, 1])

    def test_the_last_line_carries_the_totals_and_processes_add_up(self):
        text = ORDER_LINE.replace("cutoffs=100", "cutoffs=1") + "\n" + ORDER_LINE + "\n" + TT_LINE + "\n"
        order, tt, occ = m.summarize([text, text])
        self.assertEqual(order["cutoffs"], 200)
        self.assertEqual(order["class"], [100, 60, 0, 20, 20])
        self.assertEqual(tt["store"], [2, 4, 6, 8, 10])
        self.assertNotIn("sample_empty", tt)
        self.assertAlmostEqual(occ[0], 0.5)

    def test_occupancy_is_the_mean_share_over_searches(self):
        second = TT_LINE.replace("sample_empty=2", "sample_empty=0").replace("sample_age=1,1,0,0", "sample_age=4,0,0,0")
        occ = m.occupancy(TT_LINE + "\n" + second)
        self.assertAlmostEqual(occ[0], 0.25)
        self.assertAlmostEqual(occ[1], 0.625)


if __name__ == "__main__":
    unittest.main()
