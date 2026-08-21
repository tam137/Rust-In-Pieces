---
name: engine_version_comparison_and_stagnation_analysis
description: Standard operating procedure (SOP) for diagnosing playing strength (Elo) stagnation, benchmarking multi-version differentials, and analyzing why newer engine versions fail to gain rating points.
---

# Engine Version Comparison & Stagnation Analysis Procedure

This skill defines the standard operating procedure (SOP) for investigating playing strength plateaus and diagnosing why consecutive engine versions show no measurable Elo progression.

---

## 1. Multi-Version Change Classification

When comparing multiple engine releases (e.g., $V_A$, $V_B$, $V_C$):

1. **Inspect Commit Logs & Diffs:**
   ```bash
   git log Version_A..Version_C --oneline
   git diff --stat Version_A Version_C
   ```
2. **Classify Each Version's Delta:**
   - **Functional Chess Logic**:
     - *Search modifications*: Pruning rules (LMR, NMP, Futility), extensions, move ordering, Transposition Table (TT) policies.
     - *Evaluation modifications*: New positional features, piece-square table updates, king safety, pawn structure, endgame scale.
     - *Move generation*: Move generator speed, staged move picking, pseudo-legal ordering.
   - **Non-Functional / Maintenance**:
     - *Logging & UCI output*: Information string formatting, debug prints, logger cleanups (e.g. `v0.27.4`).
     - *Refactoring*: Code restructuring without mathematical/algorithmic changes.
     - *Documentation & Tooling*: SPSA scripts, build scripts, markdown files.

> [!NOTE]
> Non-functional updates cannot produce playing strength gains. Always verify whether a release introduced actual search or evaluation deltas.

---

## 2. Isolated Binary Build & Diagnostic Benchmarking

To ensure reproducible metrics without active branch pollution:

1. **Build Independent Release Binaries:**
   ```bash
   git checkout tags/Version_A && cargo build --release && cp target/release/suprah target/release/suprah_version_a
   git checkout tags/Version_B && cargo build --release && cp target/release/suprah target/release/suprah_version_b
   git checkout tags/Version_C && cargo build --release && cp target/release/suprah target/release/suprah_version_c
   git checkout master
   ```

2. **Benchmark Search Metrics (NPS & Node Counts):**
   Compare fixed-depth searches (`go depth N` with `setoption name OwnBook value false`) across standard positions (Startpos, Kiwipete, Middlegame, Endgames):
   - **Node Count**: Total nodes searched to reach the target depth.
   - **NPS (Nodes Per Second)**: Leaf throughput and search speed.
   - **Score & Best Move Agreement**: Track whether evaluations diverge or identical move choices are selected.

---

## 3. LCT II Test Suite Comparative Evaluation

Execute the Louguet Chess Test II (LCT II) suite on each candidate binary to measure tactical, positional, and endgame performance:

```bash
python3 scripts/lct2_evaluator.py -b target/release/suprah_version_a -t 5
python3 scripts/lct2_evaluator.py -b target/release/suprah_version_c -t 5
```

### Evaluation Comparison Matrix
Compile a comparison table summarizing performance:

| Version | LCT II Elo | Solved / 35 | Positional (14) | Tactical (12) | Endgame (9) | Points / 1050 |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `Version_A` | Estimated Elo | X / 35 | X / 14 | X / 12 | X / 9 | Total Pts |
| `Version_B` | Estimated Elo | X / 35 | X / 14 | X / 12 | X / 9 | Total Pts |
| `Version_C` | Estimated Elo | X / 35 | X / 14 | X / 12 | X / 9 | Total Pts |

---

## 4. Root Causes of Elo Stagnation (Diagnosis Checklist)

When a version with new features fails to increase playing strength, check these five primary failure modes:

### A. Untuned Heuristic Parameters (Missing SPSA)
- **Problem**: Adding new handcrafted evaluation terms with arbitrary constant weights (e.g. 10 cp, 15 cp, 25 cp) without running SPSA parameter tuning.
- **Consequence**: Arbitrary weights distort piece-square values, create collinearity with existing features, and misalign alpha-beta aspiration windows.
- **Remedy**: Always register new parameters in `tuning/parameters.json` and `tuning/groups.json`, and run SPSA tuning before concluding feature efficacy.

### B. Leaf Evaluation Compute Overhead (NPS Tax)
- **Problem**: Adding complex evaluation terms (e.g. empty-board ray attacks, multi-bitboard scans) that increase CPU cycles per leaf node.
- **Consequence**: If an evaluation term provides marginal positional knowledge (+2 Elo) but reduces NPS by 8% (-12 Elo), the net result is an Elo plateau or regression.
- **Remedy**: Optimize evaluation logic with branchless bitboards and verify NPS stability with `scripts/benchmark_nps.py`.

### C. Low-Frequency Edge Cases
- **Problem**: Implementing theoretical rules that are 100% correct in endgame theory (e.g. $KN$ vs $KN$, $KB$ vs $KN$, wrong-colored bishop + rook pawn) but occur in $<0.5\%$ of practical tournament games.
- **Consequence**: These features will not register measurable Elo increases in standard test suites.

### D. Collinearity & Feature Overlap
- **Problem**: Two or more evaluation terms rewarding the same underlying chess concept (e.g. rewarding open files via `rook_open_file` + `rook_open_file_attacks_king` + `rook_open_file_attacks_queen`).
- **Consequence**: Over-weighting a single factor causes the engine to compromise king safety or piece activity to chase redundant bonuses.

### E. Search & Movegen Architectural Ceilings
- **Problem**: The engine's strength is constrained by core search or move generation bottlenecks (e.g., pre-validating all pseudo-legal moves, lack of `MovePicker`, lack of Singular Extensions or Late Move Pruning).
- **Consequence**: Positional evaluation tweaks cannot overcome deep search horizon limitations. Consult `task.md` to prioritize architectural search refactoring.

---

## 5. Corrective Action & Verification Workflow

1. **Parameter Export**: Add all candidate evaluation constants to `src/config.rs`, `src/game_handler.rs` (UCI parser), and `tuning/parameters.json`.
2. **SPSA Optimization**: Execute automated SPSA tuning iterations to find mathematically balanced weights.
3. **SPRT Matchplay**: Verify the tuned engine against the baseline in a multi-game tournament (e.g., Cutechess-cli / Fastchess with SPRT `[0.0, 5.0]`).
4. **Documentation**: Record test results and updated Elo ratings in `LCT.md` and `CHANGELOG.md`.
