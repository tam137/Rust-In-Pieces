---
name: elo_regression_analysis_procedure
description: Guidelines and procedures for investigating, debugging, and resolving playing strength (Elo) regressions between two engine versions. Contains structured test-phases, diagnostic metrics (node counts, NPS, TT statistics), and LMR/Zobrist interaction heuristics.
---

# Elo Regression Analysis Procedure

This skill defines the standard procedure for investigating, diagnosing, and resolving playing strength (Elo) regressions when a newer engine version performs worse than a previous baseline.

---

## 1. Code-Level Comparison (Suspect Identification)

When a regression is reported between Version X and Version Y:
1. **Analyze Diff Statistics:** Use `git diff --stat Version_X Version_Y` to identify all modified files.
2. **Review Detailed Code Changes:** Run `git diff Version_X Version_Y -- src/` to inspect changes in search, evaluation, and transposition table logic.
3. **Categorize Suspects:**
   - **TT Logic Changes:** Modifying write policies, collision detection, or Zobrist key generation.
   - **Pruning & Reductions:** Changes in LMR divisor/thresholds, Null Move Pruning (NMP), Futility Pruning, or Aspiration Windows.
   - **Search Parameter Propagation:** Changes in flags like `is_pv`, `in_check`, or cutoffs.
   - **Evaluation Features:** Performance regressions (NPS drop) or positional soft-clamping adjustments.

---

## 2. Isolating Binaries for Testing

To run clean diagnostics, compile separate release binaries for each candidate version:
1. **Stash Active Changes:** Run `git stash` to preserve the current workspace state.
2. **Compile Version X:**
   - Checkout version: `git checkout tags/Version_X`
   - Compile: `cargo build --release`
   - Copy binary: `cp target/release/suprah target/release/suprah_version_x`
3. **Compile Version Y:**
   - Checkout version: `git checkout tags/Version_Y`
   - Compile: `cargo build --release`
   - Copy binary: `cp target/release/suprah target/release/suprah_version_y`
4. **Restore Workspace:** Return to your active branch (e.g. `git checkout master`) and apply your stashed changes (`git stash pop`).

---

## 3. Diagnostic Benchmarking (Metrics)

Write a test script (or use a scratch Python script) to run depth-limited searches (e.g., depth 9 or 10) on a diverse suite of FEN positions. 
* **Important:** Set the UCI option `OwnBook` to `false` to ensure the engines perform actual searches on all positions instead of playing book moves instantly.

Compare these core diagnostic metrics:

* **Node Count:** The total number of nodes searched to reach the target depth.
  - *Heuristic:* A significant node explosion (e.g., >20% increase) points to a Transposition Table failure, disabled pruning, or search tree instability.
* **NPS (Nodes Per Second):** The raw search speed.
  - *Heuristic:* A drop in NPS (e.g., >10% slower) points to performance bottlenecks, unoptimized evaluation terms, or redundant search/TT writes.
* **Search Node Distribution:** Compare node counts per position.
  - *Heuristic:* If some positions see massive savings while others explode, LMR may be too aggressive, causing tactical blindness (horizon effect) that forces late-stage research explosions.

---

## 4. Key Troubleshooting Heuristics

### A. The Transposition Table (Zobrist) Collision Bug
If the TT replacement policy blocks updates on hash collisions, the table will get polluted and frozen.
- **Bug Pattern:** Using `if existing.depth == -1 || entry.depth >= existing.depth` prevents overwriting on collisions when the new entry has a lower depth.
- **Fix:** Ensure collision writes are always allowed by checking if the keys differ:
  ```rust
  if existing.depth == -1 || existing.key != hash || entry.depth >= existing.depth
  ```

### B. LMR Divisor and `is_pv` Interaction
A correct code change (like propagating `is_pv = false` to recursive searches) can remove a dampening effect that was previously masking an overly aggressive `lmr_divisor`.
- **Bug Pattern:** In older versions, a hardcoded `is_pv = true` bug dampened reductions by 1, making an `lmr_divisor` of `180` stable. Once `is_pv` is fixed, the undampened `180` divisor makes LMR too aggressive, causing tactical blindness.
- **Fix:** Adjust the `lmr_divisor` upward (e.g. `225` or `250`) in `src/config.rs` to compensate for the correct `is_pv` propagation.

---

## 5. Verification and Bugfix Release

1. **Apply the Fix:** Make the targeted logic or parameter correction in your working files.
2. **Verify Node Stability:** Run the benchmark script again. Confirm that the node counts are either reduced or stable compared to the old baseline, and NPS is restored.
3. **Execute Release Pipeline:** Run `./build_and_release.sh` to bump the patch version, update `CHANGELOG.md`, compile the final release, and deploy it to the matchups folder.
4. **Git Commit & Tag:** Stage, commit, and tag the release manually, then push to origin master with tags.
