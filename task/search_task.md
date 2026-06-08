# Search Tree Pruning & Reduction Tasks

This document outlines the proposed tasks for integrating advanced search tree pruning and reduction techniques into the **Suprah** engine.

---

## ⚠️ Configuration Principle

Every new pruning or reduction feature **must** be fully configurable via the `Config` struct. No hardcoded search heuristics should be introduced.
* Each feature must have a corresponding enable/disable toggle or a mode selector.
* Parameters (margins, depth thresholds, divisors) must be exposed in `Config` to allow for SPSA tuning.

---

## Active Search Tasks

### 1. Late Move Pruning (LMP) / Move Count Pruning
*   **Description**: Completely discards (prunes) quiet moves after a certain number of quiet moves have already been searched at low depths.
*   **Metadata**: `[Impact: High]` `[Complexity: Low]`
*   **Tasks**:
    - `[ ]` Define an option in `Config` for LMP (`enable_lmp: bool` or `lmp_mode: LmrMode`).
    - `[ ]` Add a parameter for LMP quiet move threshold formula coefficients (e.g., `lmp_base_moves: i32` and `lmp_depth_multiplier: i32`).
    - `[ ]` In `search_service.rs` inside the move loop of `minimax`, check if the node is not in check, the depth is low, and the number of searched quiet moves exceeds the threshold. If so, prune the remaining quiet moves (`break`).

### 2. Futility Pruning (FP) in the Move Loop
*   **Description**: Prunes individual quiet moves in the move loop at very shallow depths ($depth \le 2$) if the static evaluation plus a safety margin cannot possibly raise alpha.
*   **Metadata**: `[Impact: High]` `[Complexity: Medium]`
*   **Tasks**:
    - `[ ]` Add `enable_futility_pruning: bool` and `futility_margin_per_depth: i16` to `Config`.
    - `[ ]` In the `minimax` move loop, for quiet moves at depth 1 or 2, calculate if `static_eval + margin * depth < alpha`. If true, skip/prune the move (`continue`).
    - `[ ]` Ensure this is skipped if the node is in check or is a PV node.

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
