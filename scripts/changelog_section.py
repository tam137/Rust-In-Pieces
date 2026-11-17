#!/usr/bin/env python3
"""Prints the `CHANGELOG.md` section of a release tag, or fails if it is not ready to publish.

Used by `.github/workflows/release-binaries.yml` for the release notes: a pushed release tag
publishes immediately, so a missing, empty or not yet enriched section (still the stub that
`build_and_release.sh` writes) must stop the run before anything is built.

Usage: python3 scripts/changelog_section.py <tag> [CHANGELOG.md]
Exit status 0 prints the section body (without its header) to stdout; 1 prints the reason.
Pure standard library; kept identical on `master` and `feature/nnue-evaluation`.
"""

import sys

# The placeholder line `build_and_release.sh` writes into a new section.
PIPELINE_STUB = "- Release updates and improvements"


class ChangelogError(Exception):
    """The section of the tag cannot be published as release notes."""


def section(changelog: str, tag: str) -> str:
    """Returns the body of the `## [V<version>]` section of `tag` (e.g. `v0.53.0-NNUE`)."""
    header = f"## [V{tag.removeprefix('v')}]"
    body, inside = [], False
    for line in changelog.splitlines():
        if inside and line.startswith("## ["):
            break
        if inside:
            body.append(line)
        elif line.startswith(header):
            inside = True
    if not inside:
        raise ChangelogError(f"no CHANGELOG.md section '{header}'")
    text = "\n".join(body).strip()
    if not text:
        raise ChangelogError(f"CHANGELOG.md section '{header}' is empty")
    if any(line.strip() == PIPELINE_STUB for line in body):
        raise ChangelogError(
            f"CHANGELOG.md section '{header}' is still the pipeline stub; enrich it before tagging"
        )
    return text + "\n"


def main(argv: list[str]) -> int:
    if len(argv) not in (2, 3):
        print(__doc__.strip().splitlines()[0], file=sys.stderr)
        return 2
    path = argv[2] if len(argv) == 3 else "CHANGELOG.md"
    try:
        with open(path, encoding="utf-8") as f:
            sys.stdout.write(section(f.read(), argv[1]))
    except ChangelogError as e:
        print(f"::error::{e}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
