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
1. **Run Unit Tests & Code Quality Audit:** Execute active unit tests first: `cargo test`. Whenever running asynchronous test commands during a release procedure, the AI MUST explicitly wait for background test execution to finish completely and verify 100% clean success (`test result: ok`) BEFORE proceeding to execute `./build_and_release.sh`. In addition to all tests passing (being green), the entire codebase MUST be completely free of compiler warnings. Crucially, all modified or newly introduced code MUST strictly comply with the standards defined in the [Code Quality and Performance Audit](code_quality_and_performance_audit.md) skill (including zero-warning enforcement without suppression attributes, complete unit test coverage, zero heap allocations on search/eval hot paths, and elimination of redundant code).
2. **Mandatory Cross-Version Smoke Gauntlet for Search and Evaluation Changes:** If the release touches
   `src/search_service.rs`, `src/eval_service.rs`, `src/move_gen_service.rs` or any search
   parameter default, the candidate MUST play a gauntlet against **at least the two preceding
   releases** before the pipeline is run:
   ```bash
   # <mm>/<name>.trn, mode = gauntlet, challenger listed first
   engines = <candidate>, <previous release>, <release before that>
   time_control = 1000          # ALWAYS 1s + 100ms. Never longer.
   increment = 100
   rounds = 50                  # 100 games per pairing. Never more.
   concurrency = <floor(nproc * 0.75) - 1>   # per host. 5 on 8 cores, 14 on 20.
   openings = openings_wide.txt
   ```

   > [!IMPORTANT]
   > **This gauntlet is a smoke test, not a measurement.** Its only job is to catch a candidate
   > that is grossly broken — the class of defect a self-A/B cannot see.

   Evaluate **per pairing**, never by the scoreboard rating: the Matt-Magie scoreboard is an
   iterative Bradley-Terry model normalised to a pool average of 2000, so a rating depends on
   which engines happen to be in the PGN and two ratings from different pools are not comparable.
   A candidate that scores below roughly 45% against a predecessor MUST NOT be released.

3. **Mandatory A/B Matchplay & Strength Measurement for Critikal Changes:** For complex modifications intended or expected to shift engine playing strength (e.g. new search pruning/extension heuristics, evaluation features, SPSA parameter tunings, or major architectural migrations), an **A/B tournament / match** against the baseline engine version MUST be executed and analyzed prior to release.
   - **Procedure & Execution:** All execution details (match setup, stopped SPRT gating vs. fixed-N effect size estimation, opening pool setup with openng book, hardware concurrency caps, health auditing via `scripts/match_health.py`, and paired Elo calculation via `scripts/pairing_elo.py`) MUST strictly follow the standardized [Matchplay Measurement Procedure](matchplay_measurement_procedure.md) skill.

4. **Run Build & Release Pipeline:** Proceed to run the release script: `./build_and_release.sh`.

> [!NOTE]
> **Performance / Perft Tests (`cargo test -- --ignored`)**: By default, long-running perft and ignored performance tests are **OMITTED** during the standard release procedure to save time. Do NOT run `cargo test -- --ignored` or document `perft.md` unless the USER explicitly requests or demands perft benchmarking beforehand.

> [!NOTE]
> **LCT II Tactical Tests**: By default, Louguet Chess Test II (LCT II) tests are **NOT** executed during the standard release procedure. Do not run or document LCT II tests unless the USER explicitly requests or mentions LCT II testing beforehand. If the USER explicitly requests LCT II testing, refer to the dedicated [LCT2 Evaluation Procedure](lct2_evaluation_procedure.md) skill to run and document LCT II results.

## 3. Pipeline Workflow
The `./build_and_release.sh` script automates version bumping (`Cargo.toml`), updating `CHANGELOG.md`, compiling optimized production binaries (both HCE and NNUE), deploying them to `<mm>/engines/`, and handling rollback on test/build failures.

## 4. Release Versioning Classification
- **Mandatory Engine Naming Scheme (`id name` in UCI)**:
  - For NNUE variants/releases (`use_nnue == true`), the engine MUST report its UCI name as `RIP V<version>-NNUE` (e.g., `RIP V0.23.12-NNUE`).
  - For standard/HCE variants/releases (`use_nnue == false`), the engine MUST report its UCI name as `Rust-In-Pieces V<version>` (e.g., `Rust-In-Pieces V0.23.12`).
- **Patch Release (x.y.z -> x.y.z+1):** Used for bug fixes, performance micro-optimizations, configuration adjustments, or minor refactorings. Run standard script: `./build_and_release.sh`.
- **Minor Release (x.y.z -> x.y+1.0):** Used for major feature implementations (e.g. History Heuristics, Transposition Tables), significant architectural migrations (e.g. Bitboard architecture, Heap-Free stack search), or any changes expected to dramatically shift engine playing strength. Run with environment override: `OVERRIDE_VERSION="x.y+1.0" ./build_and_release.sh`.

## 5. Post-Deployment
- **Mandatory Post-Deployment Sequence:** Immediately after the automated pipeline script `./build_and_release.sh` runs successfully, the AI MUST execute the following steps in order:
  1. **Enrich Changelog:** Manually open `CHANGELOG.md` and replace the autogenerated stub with a **short bullet list for the release** — a handful of concise, factual bullets covering what changed, the key measured number if there is one (with a pointer to `task.md` for the full write-up), and any new default or known limitation. **Do NOT write a detailed technical analysis, a multi-section write-up, or restate the full measurement methodology** — that belongs in `task.md`, not the changelog. `CHANGELOG.md` is a scannable release list, not a report. **Do NOT include absolute file path hyperlinks (e.g. `[config.rs](file:///...)`) in the changelog**; reference files and configurations only as plain text (e.g. `config.rs` or `src/config.rs`).
  2. **Changelog Review & Proposed Git Commands:** Present the enriched `CHANGELOG.md` to the USER. Do NOT automatically execute `git commit` or `git push` commands afterwards unless the USER explicitly asks/instructs the AI to perform the release commit/push. If explicitly requested by the USER, execute:
     ```bash
     git add Cargo.toml CHANGELOG.md skills/engine_release_procedure.md
     git commit -m "Release vX.Y.Z: <Detailed description of release changes>"
     git tag -a "vX.Y.Z" -m "Release version vX.Y.Z"
     git push origin master --tags
     ```
