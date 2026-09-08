# Search Tree Pruning & Reduction Tasks

This document outlines the proposed tasks for integrating advanced search tree pruning and reduction techniques into the **Suprah** engine.

> Priority order, measured specifications and the record of what has already been tried and failed
> live in `task.md` **on `master`**, which is the authority for the shared roadmap. This file lists
> only what is still open.
>
> Deleted on 2026-09-08 because they shipped, per `skills/task_management_procedure.md`: Late Move
> Pruning, SEE pruning for bad captures, Razoring and Singular Extensions. All four are `true` in
> `config.rs`; the Singular parameters were measured and retuned in v0.41.1.

---

## ⚠️ Configuration Principle

Every new pruning or reduction feature **must** be fully configurable via the `Config` struct. No hardcoded search heuristics should be introduced.
* Each feature must have a corresponding enable/disable toggle or a mode selector.
* Parameters (margins, depth thresholds, divisors) must be exposed in `Config` to allow for SPSA tuning.

---

## Active Search Tasks

### 1. Late Move Reductions (LMR) for Bad Captures
*   **Description**: Instead of only reducing quiet moves, apply depth reductions (LMR) to captures that lose material ($SEE < 0$).
*   **Metadata**: `[Impact: Medium]` `[Complexity: Medium]`
*   **Tasks**:
    - `[ ]` Add `enable_bad_capture_lmr: bool` and `bad_capture_lmr_reduction: i32` to `Config`.
    - `[ ]` Integrate with the existing `enable_lmr` logic in `search_service.rs` to allow reducing captures with $SEE < 0$.

### 2. ProbCut (Probability Cut)
*   **Description**: Searches highly promising/forced lines at a reduced depth with a very high beta threshold to detect if a beta cutoff is statistically guaranteed.
*   **Metadata**: `[Impact: High]` `[Complexity: Medium-High]`
*   **Tasks**:
    - `[ ]` Add `enable_probcut: bool`, `probcut_margin: i16`, and `probcut_depth_reduction: i32` to `Config`.
    - `[ ]` At depths $\ge 5$, perform a shallow search with a window of $[beta + margin, beta + margin + 1]$. If it fails high, prune the node and return beta.
