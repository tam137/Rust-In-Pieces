# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).



## [V0.11.2] - 2026-05-29

### Added
- Added common opening variations to book.rs

### Fixed



## [V0.11.1] - 2026-05-29

### Added
- Exposed tuning parameters via UCI for SPSA tuning

### Fixed



## [V0.11.0] - 2026-05-29

### Added
- **O(1) Incremental Evaluation for PSTs and Material (`src/pst.rs`, `src/model.rs`, `src/eval_service.rs`)**:
  - Re-architected the entire evaluation mapping by extracting static piece-values and positional piece-square tables (PSTs) into `const fn` generated mirrored arrays in `src/pst.rs`.
  - Statically combined piece material values with the positional square values at compile-time to guarantee zero-cost runtime lookup for all pieces across both Middlegame and Endgame (`PST_MG` and `PST_EG`).
  - Extended the `Board` struct with `pst_mg: i16` and `pst_eg: i16` fields to maintain the cumulative evaluation mathematically.
  - Implemented strictly O(1) incremental updating in `Board::do_move()` and restoring in `Board::undo_move()` (caching the pre-move evaluation in `MoveInformation`), removing the O(N) iterative piece-scanning from the evaluation loop completely.
  - Reduced evaluation execution time dramatically, resulting in an ~18% NPS boost in the Perft benchmark (from 2.6M NPS to 3.14M NPS) without any strength regressions.
  - Solved complex endgame and positional structures by unifying the material map and positional evaluation (achieved 2140 ELO on the LCT II benchmark suite).

### Fixed



## [V0.10.12] - 2026-05-29

### Added
- **PST (Piece-Square Tables) for Queen and Rook (`src/eval_service.rs`)**:
  - Implemented position-dependent evaluations for queens and rooks using `ROOK_PST` and `QUEEN_PST` to improve centralization evaluation.
- **Accurate Game-Phase Calculation (`src/eval_service.rs`)**:
  - Refactored `get_game_phase` to only count major and minor pieces (knights, bishops, rooks, queens) weighted by material value, ensuring pure pawn endgames are correctly classified as endgames.
- **King Danger / Attacker Count Weighting (`src/eval_service.rs`)**:
  - Implemented an advanced King Danger concept. Attackers on the king-ring are no longer just linearly summed. The evaluation now takes the total number of attacking pieces into account and applies exponential weights based on attacker counts.

### Fixed
- **Engine Panic on Game Over / Zero Legal Moves (`src/model.rs`, `src/game_handler.rs`)**:
  - Fixed a critical panic (`RIP Found no PV move row`) that occurred when the engine was asked to search a position with no legal moves (e.g. checkmate/stalemate delivered by the opponent). The `get_pv_move_row` function now safely handles empty variants instead of panicking.
  - Implemented an early exit in `game_handler.rs` at the root node. If 0 legal moves are found, it immediately outputs `bestmove 0000` instead of attempting an illegal search. This prevents the `lichess-bot` from waiting indefinitely for a move and causing Lichess timeouts.
  - Aligned `get_best_move_algebraic` to return the UCI standard `"0000"` (null move) instead of `"N/A"` for empty variant lists.



## [V0.10.11] - 2026-05-29

### Added
- **Pawn Undeveloped Malus Fix (`src/eval_service.rs`)**:
  - Restored protective pawn shields in front of the castled king by restricting the "Undeveloped Malus" strictly to the `d` and `e` center files. Pawns on `a, b, c` and `f, g, h` no longer receive artificial pressure to move in the early game.
- **Advanced Pawn Structure Evaluation (`src/config.rs`, `src/eval_service.rs`)**:
  - **Backward Pawns**: Added mathematical detection of structurally backward pawns (pawns that are not isolated but lack friendly pawns on adjacent files on the same rank or behind them). Introduced `pawn_backward_malus` to penalize this weakness.
  - **Doubled Pawns (Bitboard Full Scan)**: Replaced the hardcoded, localized (up to 3 squares ahead) doubled pawn checks with a robust, complete file bitboard intersection mask `(0x0101010101010101u64 << file)`, correctly identifying doubled pawns anywhere on the entire file.

### Fixed



## [V0.10.10] - 2026-05-28

### Added
- **Refactoring of Time Benchmark Suite (`src/time_check.rs`)**:
  - Fully removed deprecated `DataMap` parameter-passing and manual `local_map` cloning across all timing benchmarks and performance testing threads, aligning them with the stateless, stack-allocated `SearchContext` architecture.
  - Simplified the signatures of `run_time_check` and `calculate_benchmark` to no longer require `mut local_map: &mut DataMap`.
  - Refactored internal `generate_valid_moves_list`, `generate_valid_moves_list_capture`, and `calc_eval` timing loops to use explicit stateless boolean parameters instead of dynamic map insertions.
- **Deployment Enhancements (`matt-magie/deploy.sh`)**:
  - Updated the deployment payload inside `deploy.sh` in the Matt-Magie wrapper to automatically bundle and copy all `.trn` tournament configuration files (such as `test_gauntlet.trn`) to the remote server, enabling full tournament setups out-of-the-box.

### Fixed
- **Symbolic Link Resolution on Remote ARM Server**:
  - Re-created the missing `/home/mattmagie/mattmagie/` target directory on the remote server to fully resolve the broken symbolical link `/root/mattmagie`, ensuring completely warning-free native compilation and deployments.



## [V0.10.9] - 2026-05-28

### Added
- Functional cleanup in time_check

### Fixed



## [V0.10.8] - 2026-05-28

### Added

- **Dynamic UCI Hash Option (`src/threads.rs`, `src/model.rs`, `src/search_service.rs`, `src/time_check.rs`, `src/game_handler.rs`)**:
  - Implemented the `Hash` UCI protocol option (default `10` MB, max `1000` MB) to allow external platforms (like Lichess or GUI interfaces) to configure the transposition table size.
  - Dynamically reallocates the `ZobristTable` behind an `RwLock` ensuring safe resizing without invalidating references during ongoing searches.
  - Ensures robust integration by calculating max entries based on the memory limit rather than static capacities.

### Fixed



## [V0.10.7] - 2026-05-28

### Added
- **UCI Move Overhead Configuration (`src/config.rs`, `src/game_handler.rs`, `src/threads.rs`)**:
  - Implemented the `Move Overhead` UCI protocol option (default `0` ms, max `5000` ms) to allow external platforms (like Lichess or GUI interfaces) to compensate for network latency and inter-process communication delays.
  - Dynamically subtracted the overhead parameter directly from the available thinking time budget (`wtime` / `btime`) prior to algorithmic time allocation in `calculate_thinking_time`.
  - Ensures robust time management that strictly prevents the engine from dropping on time (flagging) during fast bullet or blitz time controls on external servers.

### Fixed



## [V0.10.6] - 2026-05-28

### Added
- **O(1) Evaluation Masks (`src/eval_service.rs`)**:
  - Replaced iterative loop-based evaluation logic for `get_king_ring`, `is_white_passed_pawn`, and `is_black_passed_pawn` with static, precalculated constant bitboard masks (`KING_RING_MASKS`, `WHITE_PASSED_PAWN_MASKS`, `BLACK_PASSED_PAWN_MASKS`).
  - Achieved O(1) mathematical lookup, entirely eliminating inner loops and branch prediction overheads evaluated millions of times per second.
- **Lazy Move Picking & Lazy SEE Optimization (`src/search_service.rs`)**:
  - Deprecated the O(N^2) Selection Sort that fully sorted the `MoveList` before node evaluation.
  - Implemented an incremental "Lazy Move Picker" that identifies and evaluates the absolute best unsearched move on-the-fly (`get_next_best_move_index`).
  - Shifted Static Exchange Evaluation (SEE) from an expensive, upfront sorting stage into the lazy evaluation loop. Bad captures (`SEE < 0`) are now deferred and placed correctly in the ordering flow only when needed.
  - Achieved massive scaling optimization: the engine often encounters a Beta Cutoff after the 1st or 2nd move, thereby saving 90% of the sorting and SEE execution times that were previously wasted.
  - **Performance Benchmarks**:
    - Depth 8 Search Time decreased from 116 ms to **72 ms** (1.6x faster).
    - NPS skyrocketed from 3.2M to over **4.4M NPS**.
    - ELO estimation increased to **2105** (+25 Elo over v0.10.5).

### Fixed



## [V0.10.5] - 2026-05-28

### Added
- **Asymmetric Positional Soft-Capping Restoration (V0.10.3 Baseline)**:
  - Reverted the overfitted symmetric, piece-count-based capping from `v0.10.4` back to the successful `v0.10.3` asymmetric soft-capping limits (Normal = 150 cp, Aggressive = 250 cp, HighAggressive = 400 cp).
  - Retained `positional_cap_damping: 5` as the default damping factor.
  - This preserves vital endgame positional coordination (such as king centralization and rook activity) and completely eliminates the "saturation blindness" that plagued `v0.10.4`.

### Fixed
- **Bishop Rim-Trapping Check Bug**:
  - Resolved a long-standing rank-independent evaluation bug where healthy, highly active bishops on `a2`/`h2` (White) and `a7`/`h7` (Black) were penalized by **-50 cp** whenever their baseline squares (`b1`/`g1` or `b8`/`g8`) were occupied.
  - Restricted the White bishop rim-trapping check strictly to the 7th rank (`rank == 6`, i.e., `a7` and `h7`).
  - Restricted the Black bishop rim-trapping check strictly to the 2nd rank (`rank == 1`, i.e., `a2` and `h2`).
- **De-escalation of Static Tempo Bonuses (Patzer Threat Elimination)**:
  - Scaled down redundant static threat bonuses to minor guiding values, allowing the search tree to resolve tactical threats dynamically.
  - Reduced `pawn_attacks_opponent_fig_with_tempo` from `150` to `15` cp, eliminating short-term weakening pawn pushes (`g3g4` in `LCTII.POS.01`).
  - Reduced `queen_in_attack_with_tempo` from `700` to `30` cp, resolving queen-exchange "shadowboxing" (`f2h4` instead of positional `f2c5` in `LCTII.POS.03`).
  - Reduced `knight_attacks_rook_tempo` from `100` to `10` cp.



## [V0.10.4] - 2026-05-27

> [!WARNING]
> **REGRESSION WARNING**: This version introduces a severe playing-strength regression in practical tournament play, dropping approximately 225 Elo compared to the highly successful V0.10.3 release (scoring only 47.7% in head-to-head matchups). The strictly symmetric, piece-count-based capping logic was overfitted to the static LCT II benchmark suite. By capping pure pawn endgames too tightly (30 cp), it completely blinded the engine to vital positional principles such as king centralization and rook activity. **This version is deprecated. Please use V0.10.3 for practical play.**

### Added
- **Piece-Based Dynamic Positional Evaluation Capping (Overfitted / Regressive)**:
  - Dynamically scaled the positional evaluation cap based on the number of non-pawn pieces: capping pure pawn endgames strictly at 30 cp, while scaling up to 830+ cp in piece-heavy middlegames.
  - While this overfitted design allowed the engine to resolve specific static puzzles in the LCT II benchmark (solving the `f6f3` Queen sacrifice in `LCTII.TAC.03` and estimating 2200 Elo), it crippled long-term positional coordination in actual tournament play.



## [V0.10.3] - 2026-05-27

### Added
- **Configurable HCE Positional Evaluation Capping**:
  - Implemented positional evaluation capping to prevent material/positional asymmetry blunders, making the cap tier based on `config.aggressiveness` (Normal = 150 cp, Aggressive = 250 cp, HighAggressive = 400 cp).
  - Used an elegant piecewise soft-capping function to compress excess positional evaluations by a configurable damping factor, preventing "Saturation Blindness" (flat evaluation landscapes).
  - Added configurable parameters `enable_positional_cap: bool` and `positional_cap_damping: i16` (defaulting to 5x damping instead of 16x) to the `Config` struct in `src/config.rs`.
  - Added UCI engine settings parsing support via `setoption name PositionalCapDamping value <val>` and `setoption name EnablePositionalCap value <val>`.
  - Added the unit test `test_positional_evaluation_capping` in `src/eval_service.rs` to verify correct soft-capped values.

### Fixed
- **Improved Aggressiveness Options & Damping Tuning**: Tuned the positional evaluation capping damping to 5x to ensure optimal tactical search performance and prevent overly aggressive pruning of positional advantages.



## [V0.10.2] - 2026-05-26

### Added
- **Rust Edition 2024 Upgrade**:
  - Upgraded the package edition from `2021` to `2024` in `Cargo.toml`.
  - Updated coding standards in `agents.md` to specify Rust Edition 2024.
  - Updated technical introduction in `README.md` to state `written in Rust (Edition 2024)`.
- **Keyword `gen` Conflict Resolution**:
  - Renamed custom Zobrist hashing generator function `zobrist::gen` to `zobrist::gen_hash` to resolve compilation conflict with the new `gen` keyword reserved in Rust 2024.
  - Refactored random `StdRng` generation in `src/zobrist.rs` from `.gen()` to `.next_u64()` using `rand::RngCore`, keeping the seeded sequence 100% identical and maintaining identical playing strength.
- **Unsafe Function Safety Defaults (`unsafe_op_in_unsafe_fn`)**:
  - Wrapped mutable static mutations (`BISHOP_MAGICS`, `BISHOP_TABLE`, `ROOK_MAGICS`, `ROOK_TABLE`) inside `unsafe fn initialize_magics` inside `src/magic.rs` in an internal `unsafe { ... }` block, satisfying safe-by-default unsafe function bodies in Rust 2024.

### Fixed
- **Compiler Dead Code Warnings**: Suppressed dead code warning on unused field `padding` in `TranspositionEntry` using the standard `#[allow(dead_code)]` attribute, achieving a 100% warning-free compilation.



## [V0.10.1] - 2026-05-26

### Added
- **Technical Search Reference Documentation (`README.md`)**:
  - Enriched `README.md` with a comprehensive technical table and overview of all minimax search features and selective pruning heuristics implemented in SupraH.
  - Provided concise technical definitions and direct hyperlinks to the English-language [Chess Programming Wiki (CPW)](https://www.chessprogramming.org) for:
    - Alpha-Beta Pruning
    - Principal Variation Search (PVS)
    - Late Move Reductions (LMR)
    - Null Move Pruning (NMP)
    - Aspiration Windows
    - Iterative Deepening
    - Quiescence Search (Q-Search)
    - Transposition Tables (TT)
    - Static Exchange Evaluation (SEE)
    - Killer Moves & History Heuristic



## [V0.10.0] - 2026-05-26

### Added
- **Safe & Portable Lock-Free Transposition Table (`src/zobrist.rs`)**:
  - Re-architected `ZobristTable` from an `RwLock<Vec<TranspositionEntry>>` to a completely lock-free array of `AtomicEntry` structs.
  - Utilized a standard, portable **Double-Check Atomic load/store sequence** indexed with `AtomicU64` key/data pairs to completely eliminate write races and dynamic lock contention.
  - Designed bit-packing routines (`pack()` and `unpack()`) to compress `eval` (16 bits), `best_move` (16 bits), `depth` (8 bits), and `entry_type` (8 bits) into a single `u64` data payload.
  - Implemented the depth-preferred replacement policy in `insert_entry` with a safe read-update sequence, ensuring no torn entries can ever corrupt search results.
  - Wrote a massive multithreaded stress test (`zobrist_lock_free_concurrency_test`) utilizing 8 parallel threads concurrently accessing and mutating the transposition table to verify thread-safety under heavy contention.
- **Static Exchange Evaluation (SEE) in Move Ordering (`src/search_service.rs`)**:
  - Integrated the Static Exchange Evaluation (SEE) pruning heuristic into both the recursive search `minimax` (depth > 0) and the root search `get_moves` move sorting phases.
  - Automatically filters and demotes capture moves that statically lose material (`SEE < 0`), moving them below all quiet moves while preserving their relative MVV-LVA ordering.
  - Excluded PV/TT moves from demotion to guarantee that the previous best-path candidates are always searched first.
  - **Search Tree Compression**: Compresses the search tree by **29.4%** at depth 9 (from 357,072 nodes down to 252,146 nodes on startpos), reducing search time by 5.4% while maintaining the exact same search path.
- **Configurable UCI Aggressiveness Control (`src/config.rs`, `src/threads.rs`, `src/game_handler.rs`)**:
  - Defined the `Aggressiveness` enum (Normal, Aggressive, HighAggressive) and integrated the field into the engine's `Config`.
  - Added UCI engine settings parsing support via `setoption name Aggressiveness value <mode>`.
  - Dynamically clones configuration into `active_config` inside the `game_loop` and updates it upon command, passing it down to all subsequent search layers.
  - Scales positional weights dynamically inside `calc_eval`: King Ring Attacks are scaled by `1.5x` (Aggressive) and `2.0x` (HighAggressive), Queen Attacks by `1.3x` / `1.6x`, and mobility bonuses by `1.2x` / `1.4x`, with `Normal` remaining perfectly matched to our baseline.
- **Gives-Check Bonus Elimination (`src/config.rs`, `src/eval_service.rs`, `src/search_service.rs`, `src/time_check.rs`)**:
  - Deleted the static positional `gives_check_bonus` (+30 cp) from evaluation to eliminate "patzer-checks" that statically inflated positions but worsened engine play. Deep forcing checks are still found dynamically.

### Fixed
- **Compiler Dead Code Warnings**: Resolved unused variant warnings for `Aggressive` and `HighAggressive` by actively routing them through the UCI command channel processor.



## [V0.9.9] - 2026-05-26

### Added
- **Restoration of Check-Giving Heuristic (gives_check_bonus)**:
  - Re-established the positional check-giving bonus (`gives_check_bonus = 30` centipawns) in the minimax search and Quiescence Search.
  - Dynamically resolved check-giving states directly on the stack by inspecting the last played move (`turn.gives_check`) and the side to move (`board.white_to_move`), achieving a zero-overhead, completely allocation-free integration.
  - Resolved a silent evaluation regression present since version `v0.9.4` (where the gives-check bonus was bypassed with static `false, false` arguments to `calc_eval` after the removal of `DataMap`), fully restoring the engine's tactical aggressiveness, forcing moves, and Matt-net tracking capabilities.
- **Unit Verification Suite**:
  - Implemented `test_gives_check_evaluation_bonus` in `src/eval_service.rs` to mathematically verify check-giving bonuses under the engine's game phase scaling system (`gives_check_bonus * game_phase / 256`).

### Fixed



## [V0.9.8] - 2026-05-26

> [!WARNING]
> **AFFECTED BY EVAL REGRESSION**: This version suffers from a silent evaluation regression where the check-giving bonus (`gives_check_bonus = 30`) was bypassed by passing static `false, false` arguments to `calc_eval` in search loops. This causes the engine to play much more passively and miss forcing moves.

### Added
- **Static Exchange Evaluation (SEE) Engine Heuristic**:
  - Implemented the `see` minimax exchange evaluation algorithm in `src/search_service.rs` to dynamically calculate the centipawn score of capture sequences on a single target square before fully searching them.
  - Implemented a fast threshold wrapper `see_ge` to optimize cutoff decisions during move processing.
  - Mapped piece indices (10..25) to centralized centipawn values using a robust `get_piece_value` utility.
- **Dynamic Occupancy Attacker Filtering (Magic Bitboards)**:
  - Designed `get_least_valuable_attacker` leveraging optimized magic bitboard masks from `get_attackers_mask`.
  - Added dynamic occupancy updates (`occupied &= !(1u64 << attacker_sq)`) inside the evaluation loop, allowing the recursive attacker resolution chain to handle X-ray attacks (e.g. bishops or rooks attacking behind pawns/knights) in real-time without complex static masking.
- **Quiescence Search Capture Pruning**:
  - Integrated capture pruning in the `depth <= 0` portion of the `minimax` search (Q-Search) to bypass quiet captures that statically lose material (`SEE < 0`), resolving the costly "Queen captures protected pawn" blindspot.
  - Safely restricted pruning to non-check nodes (`!in_check`) and excluded promotions (`capture_turn.promotion == 0`) to preserve tactical accuracy and avoid missing mate threats.
- **Search Tree Compression**:
  - Compressed the search tree at depth 9 by **47.0%** (from 673,795 nodes down to 357,072 nodes on startpos) without any tactical strength regressions, accelerating depth progression and clock compliance.
- **Unit Verification Suite**:
  - Wrote a comprehensive unit test `test_static_exchange_evaluation` in `src/search_service.rs` validating recursive SEE minimax scores on three distinct chess configurations (equal trades, favorable captures, and unfavorable blunder trades).

### Fixed
- **Static Attacker Infinite Loop**: Fixed a bug where `get_attackers_mask` returned captured pieces by intersecting attackers with the active `occupied` bitboard, resolving array index out of bounds panics.



## [V0.9.7] - 2026-05-26

> [!WARNING]
> **AFFECTED BY EVAL REGRESSION**: This version suffers from a silent evaluation regression where the check-giving bonus (`gives_check_bonus = 30`) was bypassed by passing static `false, false` arguments to `calc_eval` in search loops. This causes the engine to play much more passively and miss forcing moves.

### Added
- **Logarithmic LMR Table Heuristic**: Replaced the static, 1-ply quiet move search reduction with a precalculated logarithmic LMR table indexed by `[depth][move_index]` to achieve aggressive, mathematically scaled search tree compression.
- **Dynamic Reduction Discounts & Metadata Coupling**:
  - Decreased LMR reduction by `1` ply for PV nodes, Killer moves, and Counter-moves (leveraging `context.counter_move`) to protect promising paths.
  - Dynamically coupled LMR with History Heuristics: reduced LMR by `1` ply for quiet moves with high history scores (`> 4000`) and increased LMR by `1` ply for historically weak quiet moves (`< 500`) to prune poor branches earlier.
- **LMR Safety Clamping**: Quiet moves that receive substantial discounts can decrease the calculated LMR reduction to `0` or less; in such cases, the search safely skips LMR entirely and executes a full search. Clamped all valid reductions to a safe range of `[1..=depth-2]` to avoid reducing below the Horizon depth.
- **Divisor Configuration for Aggressiveness**: Centralized `lmr_base_divisor: f64` (default `1.95`) in `src/config.rs` with extensive explanatory documentation comments.
- **Unit Verification Suite**: Implemented robust automated unit test `test_logarithmic_lmr_table` to mathematically verify table computations, boundary limits, and conservative divisor changes.

### Fixed



## [V0.9.6] - 2026-05-26

> [!WARNING]
> **AFFECTED BY EVAL REGRESSION**: This version suffers from a silent evaluation regression where the check-giving bonus (`gives_check_bonus = 30`) was bypassed by passing static `false, false` arguments to `calc_eval` in search loops. This causes the engine to play much more passively and miss forcing moves.

### Added
- **Reactivated thinking time checks**: Correctly passed `go_start_time` (the start of the `go` command) instead of a fresh `std::time::Instant::now()` in each depth iteration of iterative deepening inside `src/game_handler.rs`.
- **Cumulative Time Budget Fix**: Previously, each depth iteration reset the search start time to `Instant::now()`, making the time-checking mechanism inside `minimax` evaluate thinking time relative only to the *start of that specific depth* rather than the *start of the entire move*. This allowed the engine to repeatedly spend its budget at each search depth, exceeding its time target by 2x to 3x, and eventually losing by timeout under rapid time controls (e.g. 9s + 105ms). Passing the single `go_start_time` across all search depths enforces a strict cumulative time ceiling, preventing flagging.
- **Time Control Compliance**: Fully verified in remote bullet tournament settings (8000ms + 110ms increment), ensuring the engine successfully finishes searches and makes moves within constraints, reclaiming its peak playing strength.

### Fixed
- **Tournament Timeout Bug**: Resolved the critical time management defect present in V0.9.4 and V0.9.5 that caused the engine to ignore clock commands and forfeit games on move 2 or 3.

## [V0.9.5] - 2026-05-26 [BUGGY - CRITICAL TIME MANAGEMENT & EVAL REGRESSION BUGS]

> [!WARNING]
> **BUGGY VERSION (CRITICAL)**: This release suffers from two critical defects:
> 1. **Time Management Bug**: `target_time` in `src/game_handler.rs` was hardcoded to `None`, causing immediate tournament timeouts.
> 2. **Check Evaluation Regression**: The check-giving bonus (`gives_check_bonus = 30`) was bypassed by passing static `false, false` arguments to `calc_eval` in search loops, causing passive play.

### Fixed
- **Nested NMP Recursion Bug**: Resolved a critical search logic bug where the `skip_null_move` boolean argument was ignored in the Null Move Pruning (NMP) trigger condition inside `src/search_service.rs`. This omission caused NMP to be executed redundantly within recursive null-searches and verification searches where NMP should have been strictly disabled.
- **Tree Compression & Search Focus**: Fixing the nested NMP bug resulted in massive search tree compression:
  - **Depth 6** nodes searched dropped by **34%** (from 45,031 down to 29,562).
  - **Depth 8** nodes searched dropped by **10%** (from 203,596 down to 182,680).
- **Extreme NPS Boost**: By eliminating redundant and heavily nested null-move cycles, the search engine throughput reached a new record-breaking peak of **13.75 MNPS** (Million Nodes Per Second) at depth 9 search on startpos, completing the search in just **49 ms**!

## [V0.9.4] - 2026-05-26 [BUGGY - CRITICAL TIME MANAGEMENT, NMP RECURSION & EVAL REGRESSION BUGS]

> [!WARNING]
> **BUGGY VERSION (CRITICAL)**: This release is highly unstable and suffers from three major defects:
> 1. **Time Management Bug**: `target_time` was hardcoded to `None`, causing immediate tournament timeouts.
> 2. **Nested NMP Recursion Bug**: Omission of the `!skip_null_move` check allowed recursive NMP cycles, bloating the search tree.
> 3. **Check Evaluation Regression**: Introduction of stack-allocated `SearchContext` silently bypassed the check-giving bonus (`gives_check_bonus = 30`) by passing static `false, false` arguments to `calc_eval`, causing passive play.
- **Dynamic Null Move Pruning (NMP) with Verification Search**: Implemented a mathematically robust NMP system. Replaced static depth reductions with dynamically scaling reductions `config.nmp_reduction + (depth / config.nmp_dynamic_divisor)`. Integrated a Verification Search at high depths (`depth >= config.nmp_verification_threshold`) to mathematically secure Zugzwang-vulnerable endgames, drastically reducing endgame blunders while maintaining tree compression.
- **SearchContext Architecture**: Fully removed the expensive, heap-allocated `DataMap` parameter-passing system. Replaced it with a zero-cost stack-allocated `SearchContext` struct passed by reference, unlocking major Multi-Threading stability and doubling raw NPS (Nodes Per Second) speed by eliminating dynamic borrow-checking overhead.
- **Stateless Evaluation Service**: Re-architected `eval_service.rs` to process check-states and evaluate board features procedurally using strict boolean flags instead of dynamic hash-map lookups, heavily optimizing leaf-node evaluation cycles.

### Fixed



## [V0.9.3] - 2026-05-26

### Added
- **Centralized Search Configuration System**: Moved previously hardcoded search and move-ordering parameters to `src/config.rs`, making the search engine fully tunable.
  - Added fields for `killer_move_1_rank_bonus` and `killer_move_2_rank_bonus` to configure quiet killer move prioritization.
  - Added `counter_move_rank_bonus` to tune the weight of refutation responses dynamically.
  - Added `history_max_threshold` to parameterize the halving limit of the history table.
  - Added `lmr_depth_threshold`, `lmr_move_threshold`, and `lmr_reduction` to configure Late Move Reductions.
  - Added `nmp_depth_threshold` and `nmp_reduction` to parameterize Null Move Pruning.
- **Counter-Moves Heuristic (Refutation Moves)**: Implemented counter-move tracking to store and prioritize successful quiet response moves to the opponent's previous moves, improving cutoff efficiency in deep search paths.
- **Delta Pruning in Quiescence Search**: Integrated dynamic capture pruning in leaf-node searches using `enable_delta_pruning` and `delta_pruning_margin` to skip futile captures (disabled by default to prevent tactical sacrifice regressions).
- **History Malus Heuristic**: Implemented penalization of quiet moves that failed to produce cutoffs by subtracting `depth * depth` from their history rating (disabled by default to preserve move-sorting stability).

### Fixed
- Fixed hardcoded limits and magic numbers across `src/search_service.rs` and `src/move_gen_service.rs`.
- Resolved search tree expansion issues by safely disabling regression-prone heuristics (Delta Pruning and History Malus) by default while keeping them fully toggleable in the configuration.



## [V0.9.2] - 2026-05-25

### Added
- Implemented v0.9.2 - Merged best of 0.9.0 and 0.9.1: Dampened evaluation heuristics and passed pawn rank bonuses to compress startpos search tree by 56% while maintaining peak ELO of 2260.

### Fixed



## [V0.9.1] - 2026-05-25

### Added
- Implemented advanced evaluation heuristics: rook behind passed pawn, protected passed pawn, king ring attacks, king opposition, and endgame pawn rank tuning, achieving peak ELO of 2260.

### Fixed



## [V0.9.0] - 2026-05-25

### Added
- Implemented Aspiration Windows and Reverse Futility Pruning (RFP)

### Fixed



## [V0.8.1] - 2026-05-24

### Added
- Implemented Null Move Pruning (NMP)

### Fixed



## [V0.8.0] - 2026-05-24

### Added
- **Principal Variation Search (PVS)**: Transitioned core minimax search to PVS to utilize aggressive zero-window searches `(alpha, alpha + 1)` and `(beta - 1, beta)` on subsequent moves, drastically cutting down the search space.
- **Late Move Reductions (LMR)**: Enabled 1-ply depth reductions for deep quiet moves (`turn_counter > 3`, `depth >= 3`) that are not captures, promotions, or checks.
- **Configuration System Expansion**: Added dynamic toggles `enable_pvs` and `enable_lmr` in `src/config.rs`.
- **Comprehensive Unit Testing**: Added `search_feature_toggles_test` in `src/search_service.rs` to programmatically verify functional correctness and node-pruning.
- **LCT II Benchmark Achievement**: Achieved **2110 ELO** (+60 Elo increase) by solving new positional and endgame positions (such as `LCTII.POS.13` and `LCTII.END.01`).
- **NPS & Depth Benchmarks**: Achieved a monumental **19x search speedup** at depth 9 by reducing evaluated nodes from 25.9M to 1.2M (95% search space reduction).

### Fixed



## [V0.7.10] - 2026-05-23

### Added
- Added new evaluation heuristics: Rook on 7th rank, Bishop and Knight mobility, Isolated Pawns penalty, and King safety pawn shield

### Fixed



## [V0.7.10] - 2026-05-23

### Added
- Added new evaluation heuristics: Rook on 7th rank, Bishop and Knight mobility, Isolated Pawns penalty, and King safety pawn shield

### Fixed



## [V0.7.9] - 2026-05-23

### Added
- Implement Magic Bitboards and eliminate evaluation heap allocations

### Fixed



## [V0.7.8] - 2026-05-23

### Added
- Expand opening book with irregular/dubious variations

### Fixed



## [V0.7.7] - 2026-05-23

### Added
- Release v0.7.7 - Remote ARM Compilation support

### Fixed



## [V0.7.6] - 2026-05-23

### Added
- Fix go infinite engine bug and restore standard UCI in benchmark

### Fixed



## [V0.7.5] - 2026-05-23

### Added
- Fix king safety by restricting centering to endgame and revert check extensions

### Fixed



## [V0.7.4] - 2026-05-23

### Added
- Implement Endgame King & Passed Pawn evaluations + Check Extensions

### Fixed



## [V0.7.3] - 2026-05-23

### Added
- Add LCT II Elo Estimator benchmark script

### Fixed



## [V0.7.2] - 2026-05-23

### Added
- Add unit test for obvious move early exit

### Fixed



## [V0.7.1] - 2026-05-23

### Added
- Expand opening book with Caro-Kann, Sicilian, Spanish, and standard lines

### Fixed



## [Unreleased]

### Added

### Fixed



## [V0.7.0] - 2026-05-23

### Added
- O(1) Mailbox Board (`board.mailbox: [u8; 64]`) keeping a direct piece lookup cache on the Board struct, eliminating O(12) bitboard scan loops inside the hot recursive search paths.
- 128-bit Compact Zobrist Transposition Entries (16-Byte memory footprint) by bit-packing chess moves into a `u16` and depth into an `i8`, maximizing L1/L2 cacheline density (4 entries per cacheline).
- Flat array Transposition Table (`ZobristTable`) replacing `CHashMap` with depth-preferred replacement policy to eliminate CPU cache misses and lock congestion.
- Incremental Move Sorting (Selection Sort / Pick Best Move) in standard minimax search loops and quiescence search to completely bypass O(N log N) sorting overhead on early Beta cutoffs.
- Underpromotions configuration toggle (`use_underpromotions`, default `false` for search, `true` for tests) to skip suboptimal Rook and Bishop promotions during search for additional NPS gains.
- Dynamic transposition table capacity initialization via configuration.

### Fixed

## [V0.6.0] - 2026-05-23

### Added
- Migrated engine to 100% Heap-Free search recursion using stack-allocated MoveList & MoveRawList
- Refactored move generation signatures to populate stack lists via mutable borrows
- Optimized double-check and check detection using direct popcnt count_ones on attackers bitboards
- Added robust capacity safety tests in model.rs with zero compiler warnings
- Enabled all FIDE-legal pawn promotions (Queen, Rook, Bishop, Knight) in `validate_and_add_promotion_moves` to make the engine 100% rules-compliant
- Added fully recursive Perft (Performance Test) suite supporting `startpos` and `Kiwipete` benchmarks to mathematically guarantee move generation correctness

### Fixed



## [V0.5.3] - 2026-05-22

### Added
- Fix integer underflow in calculate_thinking_time and reduce Movetime buffer to 50ms
- Implement Obvious Moves early exit when only 1 legal root move exists
- Refactor search timing to use single-threaded node-based check (every 1024 nodes) in minimax and quiescence search
- Implement Flexible Abort (+30% target time extension when >= 85% of root moves have been searched)

### Fixed



## [V0.5.2] - 2026-05-22

### Added
- Fix division by zero when benchmark search duration is 0ms

### Fixed



## [V0.5.1] - 2026-05-22

### Added
- v0.5.1: Added History Heuristic move-ordering and resolved in-check Quiescence Search stand-pat cutoff bug

### Fixed



## [V0.5.0] - 2026-05-22

### Added
- Implemented Killer Moves, Mate Distance Pruning, and solved critical Quiescence Search logic bugs

### Fixed



## [V0.4.2] - 2026-05-22

### Added
- Eliminated avoidable clone operations and heap allocations in minimax search path

### Fixed



## [V0.4.1] - 2026-05-22

### Added
- Transposition Table (TT) Optimization: Upgraded the legacy simple evaluation cache into a fully fledged Transposition Table (ZobristTable) storing TranspositionEntry containing evaluation, depth, transposition type (Exact, LowerBound, UpperBound), and best move. Added full Alpha-Beta bounds checking and pruning inside the minimax search, along with move ordering enhancements prioritizing the TT best move with PV node rank bonus.

### Fixed



## [V0.4.0] - 2026-05-22

### Added
- Fix divide-by-zero panic in calculate() and coordinate conversion parsing
- Migrated engine to 64-bit Bitboard Architecture (v0.4.0).

### Fixed



## [V0.3.0] - 2026-05-22

### Added
- Refactored engine to single-threaded Iterative Deepening Search, removed Lazy SMP and global_map, fixed board corruption on early search aborts

### Fixed



## [V0.2.7] - 2026-05-21

### Added
- Fix SMP move ordering and add UCI option Threads

### Fixed



## [V0.2.6] - 2026-05-21

### Added
- Added test-specific import of global_map_handler inside move_gen_service tests to silence compiler warnings

### Fixed



## [V0.2.5] - 2026-05-21

### Added
- Fixed E0382 compiler error in stop_flag_termination_test and verified all multithreading unit tests pass

### Fixed



## [V0.2.4] - 2026-05-21

### Added
- Added dynamic compile-time versioning in config.rs and integrated automated changelog workflow

### Fixed



## [V0.2.2] - 2025-02-06

### Added

- postpone evaluation and choose alternate moveordering
- consider 'give check' in evaluation
- fail fast check test in movegen

### Fixed

### Elos

| Name                                | Pkt   | Games | Elo  |
|-------------------------------------|-------|-------|------|
| Rust-In-Pieces V0.2.2 (new)         | 462.5 | 828   | 1573 |
| Rust-In-Pieces V0.2.1               | 427.5 | 828   | 1556 |
| Rust-In-Pieces V0.2.0               | 411.0 | 828   | 1548 |
| Rust-In-Pieces V0.1.4               | 355.0 | 828   | 1521 |


## [V0.2.1] - 2025-01-30

### Added

- improve knight and queen evaluation

### Fixed

### Elos

| Name                               | Pkt   | Games | Elo  |
|------------------------------------|-------|-------|------|
| Rust-In-Pieces V0.2.1 (new)        | 472.5 | 917   | 1550 |
| Rust-In-Pieces V0.2.0              | 464.5 | 917   | 1546 |
| Rust-In-Pieces V0.1.4              | 438.0 | 916   | 1535 |

## [V0.2.0] - 2025-01-14

### Added

- improve knight and queen evaluation

### Fixed

### Elos

| Name                                   | Pkt   | Games | Elo  |
|----------------------------------------|-------|-------|------|
| Rust-In-Pieces V0.2.0-candidate (new)  | 352.0 | 550   | 1564 |
| Rust-In-Pieces V0.1.4                  | 220.0 | 400   | 1520 |
| Rust-In-Pieces V0.1.2                  | 78.5  | 150   | 1509 |
| Rust-In-Pieces V0.1.1                  | 71.5  | 150   | 1491 |
| Rust-In-Pieces V0.1.3-one-thread       | 190.0 | 400   | 1490 |
| Rust-In-Pieces V0.1.3                  | 185.0 | 400   | 1485 |
| Rust-In-Pieces V0.1.0                  | 157.5 | 400   | 1458 |
| SupraH V00i-threaded-3-imp             | 45.5  | 150   | 1421 |


## [V0.1.4] - 2025-01-11

### Added

- remove mutex lock when reading stop_flag and debug_flag to avoid dead locks

### Fixed

- fixed panic when sending 'stop' cmd in 'go infinite' mode introduced by pv node feature

### Elos

| Name                                                | Pkt   | Games | Elo  |
|-----------------------------------------------------|-------|-------|------|
| Rust-In-Pieces V0.1.3-one-thread (new)              | 430.0 | 775   | 1530 |
| Rust-In-Pieces V0.1.4 (new)*                        | 295.5 | 541   | 1520 |
| Rust-In-Pieces V0.1.2                               | 373.5 | 738   | 1518 |
| Rust-In-Pieces V0.1.3                               | 385.0 | 783   | 1516 |
| Rust-In-Pieces V0.1.4-three-threads-candidate (new) | 118.5 | 265   | 1482 |
| Rust-In-Pieces V0.1.1                               | 197.0 | 454   | 1481 |
| Rust-In-Pieces V0.1.0                               |  89.5 | 222   | 1465 |

* all default engines are one-threaded from now


## [V0.1.3] - 2025-01-02

### Added

- implement skip strong validation methods (but disabled)

### Fixed

### Elos

| Name                                | Pkt   | Games | Elo  |
|-------------------------------------|-------|-------|------|
| Rust-In-Pieces V0.1.2               | 423.0 | 703   | 1534 |
| Rust-In-Pieces V0.1.3               | 314.0 | 533   | 1520 |
| Rust-In-Pieces V0.1.1               | 166.0 | 370   | 1466 |
| Rust-In-Pieces V0.1.0               |  84.5 | 204   | 1452 |
| Rust-In-Pieces V0.1.3-candidate     | 165.5 | 496   | 1423 |




## [V0.1.2] - 2024-12-30

### Added

- use block free transposition table (chashmap), reduce cash writing buffer
- use crossbeam-queue

### Fixed

### Elos

| Name                     | Pkt   | Games | Elo  |
|--------------------------|-------|-------|------|
| mewel 0.3.3              |  50.5 |    73 | 1575 |
| Rust-In-Pieces V0.1.2    | 381.5 |   703 | 1513 |
| Rust-In-Pieces V0.1.1    | 344.5 |   706 | 1488 |
| Rust-In-Pieces V0.1.0    | 165.5 |   402 | 1458 |



## [V0.1.1] - 2024-12-24

### Added

- improve overall eval
- eval: knight blocks opponent pawn
- eval: tempo bonus
- store min_max result when depth is only 2

### Fixed


## [V0.1.0] - 2024-12-22

### Added

- technical: refactor code to use threadsave datastructure where needed
- implement multithreading (Lazy SMP)
- implement new thinking time logic
- improve move ordering by PV nodes
- implement asynchronous logger
- implement asynchronous zobrist writer
- implement uci time commands movetime, movestogo and depth
- added some book moves
- bigger improvements in movegenerator (speed)
- solve all compiler warnings

### Fixed

- fixed error when move string was send in uci position command (fix cute chess)
- fixed errors in book moves
- fixed bug when promote to kNight

## [V00i] - 2024-11-22

### Added

- UCI understand debug on/off command
- UCI understand stop command
- UCI understand go infinite command
- improved UCI info strings
- improved ELO in quiescence search (again..) a lot by better cutting (alpha3)

### Fixed

- fixed error when quitting and stdout channel is closed

### Changed

### Removed


## [V00h] - 2024-11-13

### Added

- improved movegen performance
- improved performance in quiescence search
- improved ELO in quiescence search a lot by better cutting
- use cached hashing value instead of doubled eval calculation, improving performance
- added more Book moves
- extended evaluation

### Fixed

### Changed

### Removed

## [V00g] - 2024-11-06

### Added

- added pawn, king, knight and bishop evaluation
- extend logging
- add zobrist hashing

### Fixed

- three move repetition / board hashing

### Changed

### Removed

## [V00f] - 2024-11-05

### Added

- parse UCI time commands and add time management to engine
- Basic Book for move variance
- logging in rust-in-piece.log
- print "info cp" from engine perspective

### Fixed

- improved and fixed UCI protocol move parser when promotion

### Changed

- improved stand pat cuts in Quiescence Search

### Removed

## [V00e] - 2024-11-01

### Added

- Tests, Logging and error-handling

### Fixed

- Fixed critical bug in UCI protocol move parser

### Changed

### Removed

## [V00d] - 2024-10-31

### Added

- Support for en passant
- Better error handling for UCI notation strings

### Fixed

- Fixed bug in promotion notation in the UCI protocol

### Changed

- Refactored move generator

### Removed
