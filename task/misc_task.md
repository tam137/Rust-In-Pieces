# Miscellaneous Tasks & Improvements

This document outlines the proposed tasks for book extensions, UCI configurations, convenience functions, and other non-search/non-eval upgrades.

---

## Active Miscellaneous Tasks

### 1. Opening Book Expansion (Caro-Kann & Petrov's Defense)
*   **Description**: Add deeper lines for solid defensive openings in `src/book.rs` to steer the engine into drawing and solid lines.
*   **Metadata**: `[Impact: Medium]` `[Complexity: Low-Medium]`
*   **Tasks**:
    - `[ ]` Document the target FEN states and corresponding move sequences.
    - `[ ]` Add the moves to the opening book dictionary in `src/book.rs`.
    - `[ ]` Add unit tests confirming the validity of the book moves.

### 2. Expose Remaining Evaluation Parameters to UCI Options
*   **Description**: Expose all parameters from `Config` (such as isolated pawn malus, rook open file, etc.) that are currently not exposed to allow full SPSA Symmetrical Parameter Tuning.
*   **Metadata**: `[Impact: High]` `[Complexity: Medium]`
*   **Tasks**:
    - `[ ]` Register the options inside `src/threads.rs`.
    - `[ ]` Update the option parser loop in `src/game_handler.rs`.
    - `[ ]` Log parameters inside `log_all_parameters` function.

### 3. SMP Thread Synchronization Latency Optimization
*   **Description**: Reduce the latency of worker threads starting/stopping during SMP (Shared Multi-Processing) search.
*   **Metadata**: `[Impact: High]` `[Complexity: High]`
*   **Tasks**:
    - `[ ]` Profile thread wakeup overhead using cargo bench.
    - `[ ]` Optimize lock-free job queues or atomic polling parameters.
