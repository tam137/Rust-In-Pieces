---
name: Engine Release Procedure
description: Automated Release, Build, and Testing Procedure for Suprah Chess Engine. Use when instructed to release a new version.
---

# Suprah Chess Engine Release Procedure

This document outlines the mandatory procedure for building, testing, and releasing a new version of the Suprah chess engine.

## 1. Automated Release & Build Policy
- **Build Directive:** Standard manual compilation commands like `cargo build` or `cargo build --release` are strictly forbidden for releasing the engine.
- **Mandatory Script:** You MUST compile, version-bump, and deploy the engine solely using the automated pipeline script: `./build_and_release.sh`.
- **Git Branch Policy:** Do NOT create, checkout, or work on any new Git branches unless explicitly requested or commanded by the user. Git tags (e.g. `v0.21.0`) are excluded from this rule and must be created during release finalization. All work, commits, and releases must happen directly on the active branch by default.


## 2. Mandatory Release Sequence & Procedure
Whenever a release is explicitly requested by the USER (applicable for both **Patch** and **Minor** releases), the AI MUST execute the following steps in this exact chronological order:
1. **Run Unit Tests & Check Warnings:** Execute active unit tests first: `cargo test`. Whenever running asynchronous test commands during a release procedure, the AI MUST explicitly wait for background test execution to finish completely and verify 100% clean success (`test result: ok`) BEFORE proceeding to execute `./build_and_release.sh`. In addition to all tests passing (being green), the entire codebase MUST be completely free of compiler warnings. Crucially, it is strictly forbidden to use attributes or annotations that silence warnings (such as `#[allow(dead_code)]`, `#[allow(unused_variables)]`, etc.) to bypass these clean compilation requirements.
2. **Mandatory Cross-Version Gauntlet for Search and Evaluation Changes:** If the release touches
   `src/search_service.rs`, `src/eval_service.rs`, `src/move_gen_service.rs` or any search
   parameter default, the candidate MUST play a gauntlet against **at least the two preceding
   releases** before the pipeline is run:
   ```bash
   # ../matt-magie/<name>.trn, mode = gauntlet, challenger listed first
   engines = <candidate>, <previous release>, <release before that>
   time_control = 1000          # ALWAYS 1s + 100ms. Never longer.
   increment = 100
   rounds = 50                  # 100 games per pairing. Never more.
   concurrency = 9
   openings = openings_mixed.txt
   ```

   > [!IMPORTANT]
   > **This gauntlet is a smoke test, not a measurement.** Its only job is to catch a candidate
   > that is grossly broken — the class of defect a self-A/B cannot see. It is fixed at
   > **1s + 100ms and 100 games per pairing**, and must not be enlarged or lengthened to make it
   > "more accurate". At 100 games a pairing resolves to roughly +/-50 Elo, so it can see a
   > catastrophe and nothing finer. That is the intent: a release must not wait hours on a run
   > that was never going to price the change anyway.
   >
   > Pricing a change is a separate, deliberate run against the specific configuration in
   > question, at 500 to 1000 games per pairing. Do that *before* deciding to release, not as
   > part of releasing.

   Evaluate **per pairing**, never by the scoreboard rating: the Matt-Magie scoreboard is an
   iterative Bradley-Terry model normalised to a pool average of 2000, so a rating depends on
   which engines happen to be in the PGN and two ratings from different pools are not comparable.
   A candidate that scores below roughly 45% against a predecessor MUST NOT be released.

   > [!WARNING]
   > An A/B of a build against *itself* with one feature toggled is the right tool for pricing a
   > feature, but it is structurally blind to any defect both sides share. v0.30.0 shipped a
   > **-207 Elo** regression that four separate 1000-game self-A/B runs could not see, because
   > every one of them pitted a v0.30.x build against another v0.30.x build. The cross-version
   > gauntlet is the control that catches this class of defect, and the unit test suite is not:
   > all 134 tests passed on the broken release.

3. **Run Build & Release Pipeline:** Proceed to run the release script: `./build_and_release.sh`.

> [!NOTE]
> **Performance / Perft Tests (`cargo test -- --ignored`)**: By default, long-running perft and ignored performance tests are **OMITTED** during the standard release procedure to save time. Do NOT run `cargo test -- --ignored` or document `perft.md` unless the USER explicitly requests or demands perft benchmarking beforehand. If the USER explicitly requests perft benchmarking, execute `cargo test -- --ignored` and follow Section 6.

> [!NOTE]
> **LCT II Tactical Tests**: By default, Louguet Chess Test II (LCT II) tests are **NOT** executed during the standard release procedure. Do not run or document LCT II tests unless the USER explicitly requests or mentions LCT II testing beforehand. If the USER explicitly requests LCT II testing, refer to the dedicated [LCT2 Evaluation Procedure](skills/lct2_evaluation_procedure.md) skill to run and document LCT II results.

## 3. Pipeline Workflow
The `./build_and_release.sh` script automates version bumping (`Cargo.toml`), updating `CHANGELOG.md`, compiling optimized production binaries (both HCE and NNUE), deploying them to `../matt-magie/engines/`, and handling rollback on test/build failures.

## 4. Release Versioning Classification
- **Mandatory Engine Naming Scheme (`id name` in UCI)**:
  - For NNUE variants/releases (`use_nnue == true`), the engine MUST report its UCI name as `RIP V<version>-NNUE` (e.g., `RIP V0.23.12-NNUE`).
  - For standard/HCE variants/releases (`use_nnue == false`), the engine MUST report its UCI name as `Rust-In-Pieces V<version>` (e.g., `Rust-In-Pieces V0.23.12`).
- **Patch Release (x.y.z -> x.y.z+1):** Used for bug fixes, performance micro-optimizations, configuration adjustments, or minor refactorings. Run standard script: `./build_and_release.sh`.
- **Minor Release (x.y.z -> x.y+1.0):** Used for major feature implementations (e.g. History Heuristics, Transposition Tables), significant architectural migrations (e.g. Bitboard architecture, Heap-Free stack search), or any changes expected to dramatically shift engine playing strength. Run with environment override: `OVERRIDE_VERSION="x.y+1.0" ./build_and_release.sh`.

## 5. Post-Deployment
- **Failure Safety:** If compilation or testing fails, the script will automatically rollback all changes in `Cargo.toml` and `CHANGELOG.md` to prevent corrupting the workspace. Do not bypass this script!
- **Mandatory Post-Deployment Sequence:** Immediately after the automated pipeline script `./build_and_release.sh` runs successfully, the AI MUST execute the following steps in order:
  1. **Enrich Changelog:** Manually open `CHANGELOG.md` and enrich the newly created release entry with premium, comprehensive, and highly detailed technical descriptions of all added features, optimizations, fixed bugs, and performance gains. Never leave the autogenerated brief logs or arguments as-is. **Do NOT include absolute file path hyperlinks (e.g. `[config.rs](file:///...)`) in the changelog**; reference files and configurations only as plain text (e.g. `config.rs` or `src/config.rs`).
  2. **Changelog Review & Proposed Git Commands:** Present the enriched `CHANGELOG.md` to the USER. Do NOT automatically execute `git commit` or `git push` commands afterwards unless the USER explicitly asks/instructs the AI to perform the release commit/push. If explicitly requested by the USER, execute:
     ```bash
     git add Cargo.toml CHANGELOG.md skills/engine_release_procedure.md
     git commit -m "Release vX.Y.Z: <Detailed description of release changes>"
     git tag -a "vX.Y.Z" -m "Release version vX.Y.Z"
     git push origin master --tags
     ```

## 6. Perft Release Documentation Rules (Optional / Upon User Request Only)
- **Optional Documentation:** Perft performance benchmarks are **OMITTED** by default. Only run and document `perft.md` if the USER explicitly requests perft benchmarking.
- **Perft Benchmarking Procedure (Bypassing Opening Book):** To prevent triggering predefined opening book moves during the search benchmark, the AI MUST load a slightly modified FEN of the starting position where the move counter is set to **5 or higher** (e.g., `position fen rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 5`), and execute `go depth 9` or `go depth 10` to trigger a genuine search tree traversal for `perft.md`.
- **Perft Content Restriction:** `perft.md` must contain ONLY the version header (e.g., `# v0.6.0`) and the markdown table showing the latest performance benchmark results for that release. Do not include any other text, comments, or explanations.
- **Comparison History:** In `perft.md`, prepend the new version section to allow easy historical comparison.
- **Language Policy:** The table headers and all text inside `perft.md` must be written in English.
- **Perft Table Columns:** The table in `perft.md` must have exactly four columns: `Depth`, `Time`, `Nodes`, and `NPS`. The "Comment" or "Bewertung" column must be strictly excluded.

## 7. After release

- remember the user to commit and push the changes.
- verifikation: tag X.X.X exists with use_nnue: 'false' and tag X.X.X-NNUE exists with 'use_nnue: true'