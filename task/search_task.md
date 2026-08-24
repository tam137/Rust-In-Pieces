# Search Tree Pruning & Reduction Tasks

This document outlines the proposed tasks for integrating advanced search tree pruning and reduction techniques into the **Suprah** engine.

---

## ⚠️ Configuration Principle

Every new pruning or reduction feature **must** be fully configurable via the `Config` struct. No hardcoded search heuristics should be introduced.
* Each feature must have a corresponding enable/disable toggle or a mode selector.
* Parameters (margins, depth thresholds, divisors) must be exposed in `Config` to allow for SPSA tuning.

---

## Active Search Tasks

### 1. Late Move Pruning (LMP)
*   **Description**: At shallow depths, quiet moves appearing late in the move list are statistically irrelevant and can be skipped entirely instead of being searched at a reduced depth. Complements the existing LMR and Futility Pruning stages.
*   **Metadata**: `[Impact: High]` `[Complexity: Low]`
*   **Tasks**:
    - `[ ]` Add `enable_lmp: bool`, `lmp_max_depth: i32` and `lmp_base_moves: i32` to `Config`.
    - `[ ]` In the `minimax` move loop, when not in check and `depth <= lmp_max_depth`, prune all further quiet moves once the quiet move counter exceeds `lmp_base_moves + 2 * depth^2`.
    - `[ ]` Expose both parameters via UCI `setoption` for SPSA tuning.

### 2. Fail-Soft Alpha-Beta Bounds
*   **Description**: `minimax` currently initialises its running score to the window bound (`eval = if white { alpha } else { beta }`), which clamps every returned score to the search window (fail-hard). Consequently, Transposition Table entries only ever carry the window bound instead of the actually observed score, which weakens both move ordering and subsequent cutoffs.
*   **Metadata**: `[Impact: High]` `[Complexity: Medium]`
*   **Evidence**: Measured while activating the aspiration window in v0.29.1. Across eight benchmark positions at depth 8 the working window reduced node counts by 15.1% overall, but two positions *regressed* by +39.9% and +55.6%. The cause is fail-hard: when a root re-search is required, every root move returns exactly the clamped window bound, so the widening logic learns nothing about how far off the window was, and the Transposition Table entries written during the failed pass carry only that bound. Fail-soft would make aspiration re-searches converge in fewer passes.
*   **Tasks**:
    - `[ ]` Initialise the running score to `i16::MIN` / `i16::MAX` and track the best child return value independently of `alpha`/`beta`.
    - `[ ]` Verify that the Transposition Table bound classification against `orig_alpha` / `orig_beta` stays sound for the widened score range.
    - `[ ]` Confirm via a node-count regression test that the sharper bounds do not increase the tree size.

### 3. SEE-Pruning in the Main Search (Bad Capture Pruning)
*   **Description**: Currently, captures with $SEE < 0$ are sorted to the end of the move list. This task introduces hard pruning for extremely bad captures at low depths.
*   **Metadata**: `[Impact: High]` `[Complexity: Medium]`
*   **Tasks**:
    - `[ ]` Add `enable_bad_capture_pruning: bool` and `bad_capture_see_threshold: i16` to `Config`.
    - `[ ]` In the `minimax` move loop, if a move is a capture, check its SEE score.
    - `[ ]` If the SEE score is lower than a depth-dependent threshold (e.g., $SEE < -50 \cdot depth$), prune the capture entirely (`continue`).

### 4. Late Move Reductions (LMR) for Bad Captures
*   **Description**: Instead of only reducing quiet moves, apply depth reductions (LMR) to captures that lose material ($SEE < 0$).
*   **Metadata**: `[Impact: Medium]` `[Complexity: Medium]`
*   **Tasks**:
    - `[ ]` Add `enable_bad_capture_lmr: bool` and `bad_capture_lmr_reduction: i32` to `Config`.
    - `[ ]` Integrate with the existing `enable_lmr` logic in `search_service.rs` to allow reducing captures with $SEE < 0$.

### 5. Razoring
*   **Description**: An aggressive pruning technique at depth 1 when the static evaluation is extremely far below alpha. Instead of searching, it directly tries a quiescence search to see if it can recover.
*   **Metadata**: `[Impact: Medium]` `[Complexity: Medium]`
*   **Tasks**:
    - `[ ]` Add `enable_razoring: bool` and `razoring_margin: i16` to `Config`.
    - `[ ]` At depth 1, if `static_eval + razoring_margin < alpha`, perform a quick Quiescence Search. If the result is still below alpha, return that score immediately.

### 6. ProbCut (Probability Cut)
*   **Description**: Searches highly promising/forced lines at a reduced depth with a very high beta threshold to detect if a beta cutoff is statistically guaranteed.
*   **Metadata**: `[Impact: High]` `[Complexity: Medium-High]`
*   **Tasks**:
    - `[ ]` Add `enable_probcut: bool`, `probcut_margin: i16`, and `probcut_depth_reduction: i32` to `Config`.
    - `[ ]` At depths $\ge 5$, perform a shallow search with a window of $[beta + margin, beta + margin + 1]$. If it fails high, prune the node and return beta.

### 7. Singular Extensions
*   **Description**: Highly sophisticated technique that detects if a transposition table move is significantly superior to all other legal moves. If so, it extends the search by 1 ply.
*   **Metadata**: `[Impact: High]` `[Complexity: High]`
*   **Tasks**:
    - `[ ]` Add `enable_singular_extensions: bool`, `singular_margin: i16`, and `singular_depth_reduction: i32` to `Config`.
    - `[ ]` If a valid TT entry exists with sufficient depth, search all other moves with a reduced depth and a small window below the TT score.
    - `[ ]` If no other move can meet this score, extend the current search depth by 1.
