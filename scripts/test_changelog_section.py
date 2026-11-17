#!/usr/bin/env python3
"""Tests for `scripts/changelog_section.py`.

Run with `python3 scripts/test_changelog_section.py`. Pure standard library.
"""

import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import changelog_section as cs  # noqa: E402

CHANGELOG = """# Changelog

## [V0.54.0-NNUE] - 2026-11-20

### Added
- Release updates and improvements

### Fixed

## [V0.54.0] - 2026-11-19

- **Faster move ordering**: captures sorted by MVV-LVA.
- SPRT [0, 5] H1 after 9,000 games, +6.1 Elo.

## [V0.53.0-NNUE] - 2026-11-17

- **New embedded net `rnd416w01`**.

## [V0.53.0] - 2026-11-16

- Last entry, no section follows.
"""


class SectionTests(unittest.TestCase):
    def test_enriched_section_is_returned_without_its_header(self):
        body = cs.section(CHANGELOG, "v0.54.0")
        self.assertIn("Faster move ordering", body)
        self.assertIn("+6.1 Elo", body)
        self.assertNotIn("## [V0.54.0]", body)
        self.assertNotIn("rnd416w01", body)

    def test_hce_tag_does_not_match_the_nnue_section_and_vice_versa(self):
        self.assertIn("rnd416w01", cs.section(CHANGELOG, "v0.53.0-NNUE"))
        self.assertIn("Last entry", cs.section(CHANGELOG, "v0.53.0"))
        self.assertNotIn("rnd416w01", cs.section(CHANGELOG, "v0.53.0"))

    def test_last_section_runs_to_the_end_of_the_file(self):
        self.assertEqual(cs.section(CHANGELOG, "v0.53.0").strip(), "- Last entry, no section follows.")

    def test_pipeline_stub_is_rejected(self):
        with self.assertRaisesRegex(cs.ChangelogError, "still the pipeline stub"):
            cs.section(CHANGELOG, "v0.54.0-NNUE")

    def test_missing_section_is_rejected(self):
        with self.assertRaisesRegex(cs.ChangelogError, r"no CHANGELOG\.md section '## \[V0\.99\.0\]'"):
            cs.section(CHANGELOG, "v0.99.0")

    def test_empty_section_is_rejected(self):
        text = "## [V1.0.0] - 2026-01-01\n\n\n## [V0.9.0] - 2025-12-01\n\n- x\n"
        with self.assertRaisesRegex(cs.ChangelogError, "is empty"):
            cs.section(text, "v1.0.0")

    def test_version_prefix_does_not_match_a_longer_version(self):
        text = "## [V0.5.10] - 2026-01-01\n\n- ten\n"
        with self.assertRaises(cs.ChangelogError):
            cs.section(text, "v0.5.1")


if __name__ == "__main__":
    unittest.main()
