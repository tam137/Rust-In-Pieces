# Search Tree Pruning & Reduction Tasks

This document outlines the proposed tasks for integrating advanced search tree pruning and reduction techniques into the **Suprah** engine.

> Priority order, measured specifications and the record of what has already been tried and failed
> live in [`task.md`](../task.md). LMP (#1) and SEE pruning (#3) are the current next action there
> and are meant to be built and measured **together**.

---

## ⚠️ Configuration Principle

Every new pruning or reduction feature **must** be fully configurable via the `Config` struct. No hardcoded search heuristics should be introduced.
* Each feature must have a corresponding enable/disable toggle or a mode selector.
* Parameters (margins, depth thresholds, divisors) must be exposed in `Config` to allow for SPSA tuning.

---

## Active Search Tasks

### 1. Late Move Pruning (LMP) — ✅ shipped enabled in v0.35.0
*   **Description**: At shallow depths, quiet moves appearing late in the move list are statistically irrelevant and can be skipped entirely instead of being searched at a reduced depth. Complements the existing LMR and Futility Pruning stages.
*   **Metadata**: `[Impact: High]` `[Complexity: Low]`
*   **Tasks**:
    - `[x]` Add `enable_lmp: bool`, `lmp_max_depth: i32` and `lmp_base_moves: i32` to `Config`.
    - `[x]` In the `minimax` move loop, when not in check and `depth <= lmp_max_depth`, prune all further quiet moves once the quiet move counter exceeds `lmp_base_moves + 2 * depth^2`.
    - `[x]` Expose both parameters via UCI `setoption` for SPSA tuning.

### 3. SEE-Pruning in the Main Search (Bad Capture Pruning) — ⛔ measured neutral, shipped enabled in v0.35.1
*   **Description**: Currently, captures with $SEE < 0$ are sorted to the end of the move list. This task introduces hard pruning for extremely bad captures at low depths.
*   **Metadata**: `[Impact: High]` `[Complexity: Medium]`
*   **Tasks**:
    - `[x]` Add `enable_bad_capture_pruning: bool` and `bad_capture_see_threshold: i16` to `Config`.
    - `[x]` In the `minimax` move loop, if a move is a capture, check its SEE score.
    - `[x]` If the SEE score is lower than a depth-dependent threshold (e.g., $SEE < -50 \cdot depth$), prune the capture entirely (`continue`).

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
