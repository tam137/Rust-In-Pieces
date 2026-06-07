---
name: Engine Release Procedure
description: Automated Release, Build, and Testing Procedure for Suprah Chess Engine. Use when instructed to release a new version.
---

# Suprah Chess Engine Release Procedure

This document outlines the mandatory procedure for building, testing, and releasing a new version of the Suprah chess engine.

## 1. Automated Release & Build Policy
- **Build Directive:** Standard manual compilation commands like `cargo build` or `cargo build --release` are strictly forbidden for releasing the engine.
- **Mandatory Script:** You MUST compile, version-bump, and deploy the engine solely using the automated pipeline script: `./build_and_release.sh`.
- **Changelog Message:** You can pass an optional description of the functional changes as the first argument, e.g., `./build_and_release.sh "Added new search features"`. If no argument is provided, the script will automatically harvest recent git commit logs as changes.

## 2. Mandatory Release Sequence & Procedure
Whenever a release is explicitly requested by the USER (applicable for both **Patch** and **Minor** releases), the AI MUST execute the following steps in this exact chronological order:
1. **Run Unit Tests & Check Warnings:** Execute the active unit tests first: `cargo test`. In addition to all tests passing (being green), the entire codebase MUST be completely free of compiler warnings. Crucially, it is strictly forbidden to use attributes or annotations that silence warnings (such as `#[allow(dead_code)]`, `#[allow(unused_variables)]`, etc.) to bypass these clean compilation requirements.
2. **Run Performance Tests (Perft):** ONLY if all unit tests are 100% successful (green) and the code compiles without warnings, you MUST execute the performance/ignored tests: `cargo test -- --ignored` (or specifically `cargo test -- --ignored perft`).
3. **Compare Performance & Document in perft.md:** Compare the search results (Nodes, Time, NPS) with the previous version. To gather these search benchmark results correctly, you MUST bypass the opening book by loading a slightly modified FEN of the starting position with the move counter set to 5 or higher (e.g., `position fen rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 5`), run `go depth <N>` (up to depth 9 or 10), and document the actual search tree results in `perft.md`.
4. **Compile Optimized Release Binary:** Compiling the optimized release binary FIRST is mandatory before running the evaluation. Run `cargo build --release` to ensure `target/release/suprah` is compiled with your latest changes.
   > [!WARNING]
   > CRITICAL ORDER WARNING: You MUST run `cargo build --release` BEFORE running the LCT II evaluator script. If you run the evaluator on a stale binary, it will evaluate old, unmodified behavior, completely invalidating the results and potentially missing game-breaking bugs.
5. **Run LCT (Louguet Chess Test II) Tests:** Execute the LCT chess tests on the newly compiled optimized release binary by running: `python3 scripts/lct2_evaluator.py`.
6. **Document LCT Results in LCT.md:** You MUST document the LCT test results (Estimated ELO, solved positions, and scoreboard) in `LCT.md` by updating/prepending the results for the new version.
7. **Communicate and Interpret Results:** The agent MUST communicate the results of both the Perft and LCT tests to the user, including a clear interpretation of those results (e.g., whether the engine became faster/slower or gained/lost tactical strength).
8. **Run Build & Release Pipeline:** ONLY if both the Perft tests and LCT tests have been successfully executed, compared, documented, and communicated, you may proceed to execute the release script: `./build_and_release.sh "Changelog entry"`.

## 3. Pipeline Workflow
1. Executes all cargo unit tests first (`cargo test`).
2. Bumps the patch/minor version in `Cargo.toml` automatically only if all tests are green.
3. Automatically updates `CHANGELOG.md` with the new version, date, and functional changes.
4. Compiles the optimized production release binary.
5. Automatically deploys the resulting artifact directly to `../matt-magie/engines/suprah-<new_version>`.
6. Automatically stages all modified tracked files (via `git add -u`), commits them, and tags the commit (e.g. `v0.13.14`) for the release. Push to the remote remains a manual action for the user.

## 4. Release Versioning Classification
- **Patch Release (x.y.z -> x.y.z+1):** Used for bug fixes, performance micro-optimizations, configuration adjustments, or minor refactorings. Run standard script: `./build_and_release.sh "Changelog entry"`.
- **Minor Release (x.y.z -> x.y+1.0):** Used for major feature implementations (e.g. History Heuristics, Transposition Tables), significant architectural migrations (e.g. Bitboard architecture, Heap-Free stack search), or any changes expected to dramatically shift engine playing strength. Run with environment override: `OVERRIDE_VERSION="x.y+1.0" ./build_and_release.sh "Changelog entry"`.

## 5. Post-Deployment
- **Failure Safety:** If compilation or testing fails, the script will automatically rollback all changes in `Cargo.toml` and `CHANGELOG.md` to prevent corrupting the workspace. Do not bypass this script!
- **Mandatory Post-Deployment Changelog Enrichment:** Immediately after the automated pipeline script `./build_and_release.sh` runs successfully, the AI MUST manually open `CHANGELOG.md` and enrich the newly created release entry with premium, comprehensive, and highly detailed technical descriptions of all added features, optimizations, fixed bugs, and performance gains. Never leave the autogenerated brief logs or arguments as-is.
- **Execution Restriction:** Only execute the build and release script (`./build_and_release.sh`) or compile a new release binary when the USER explicitly requests or triggers a release. Do NOT automatically trigger or run a build/release after implementing changes unless explicitly asked.

## 6. Perft & LCT Release Documentation Rules
- **Mandatory Documentation:** For every release (both Patch and Minor), the AI MUST run and document the performance benchmark results in `perft.md` and the LCT chess test results in `LCT.md`.
- **Perft Benchmarking Procedure (Bypassing Opening Book):** To prevent triggering predefined opening book moves during the search benchmark, the AI MUST load a slightly modified FEN of the starting position where the move counter is set to **5 or higher** (e.g., `position fen rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 5`), and execute `go depth 9` or `go depth 10` to trigger a genuine search tree traversal for `perft.md`.
- **Perft Content Restriction:** `perft.md` must contain ONLY the version header (e.g., `# v0.6.0`) and the markdown table showing the latest performance benchmark results for that release. Do not include any other text, comments, or explanations.
- **LCT Content Restriction:** `LCT.md` must contain the updated version results (estimated ELO, solved positions, detailed results per category) and prepended or updated in the Historical Comparison table.
- **Comparison History:** In both `perft.md` and `LCT.md`, prepend or append the new version section to allow easy historical comparison.
- **Language Policy:** The table headers and all text inside both `perft.md` and `LCT.md` must be written in English.
- **Perft Table Columns:** The table in `perft.md` must have exactly four columns: `Depth`, `Time`, `Nodes`, and `NPS`. The "Comment" or "Bewertung" column must be strictly excluded.
