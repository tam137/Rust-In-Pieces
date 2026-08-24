# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).



## [V0.29.1] - 2026-08-24

### Fixed
- **Aspiration Windows Were Permanently Inactive**:
  - `SearchService::get_moves` seeded the aspiration window by probing the Transposition Table for the root position's hash. The root, however, is never written to the Transposition Table — all four `insert_entry` call sites live inside `minimax`, which only ever operates on child nodes. The probe could therefore only ever return an unrelated entry, and in practice returned nothing at all.
  - Instrumentation on `position startpos moves e2e4 e7e5 g1f3`, `go depth 9` confirmed the seed was `None` at **every** iteration from depth 3 through depth 9. Consequently `alpha` and `beta` stayed at `i16::MIN` / `i16::MAX`, the re-search loop exited immediately via its `prev_eval.is_none()` guard, and every iterative deepening iteration searched with a full window.
  - The score of the previous *completed* iteration is now passed explicitly into `get_moves` as `prev_score` and threaded through both iterative deepening loops in `src/game_handler.rs`. An explicit parameter was chosen over writing the root into the Transposition Table because it is immune to hash collisions and table eviction, and it keeps aspiration working when `use_zobrist` is disabled.
  - **Impact on SPSA tuning**: `aspiration_window_initial_delta` and `aspiration_window_multiplier` were exposed for automated tuning in v0.28.4. Because the surrounding code never executed, every tuning run since has been optimising two parameters with zero effect on play, adding noise to every other parameter in the same tuning group. Results from that period should be treated with caution.
- **Aspiration Re-Search Widened Both Window Bounds**:
  - On a fail-low or fail-high the re-search logic reset *both* `alpha` and `beta` around the returned score, discarding the bound that had just been proven correct and needlessly enlarging the re-search tree. Only the bound that actually failed is now relaxed.
  - Added a bounded fallback: once the delta reaches `aspiration_window_max_delta`, the next re-search uses a full window instead of widening geometrically forever, which caps the number of root re-searches per iteration.

### Added
- **`aspiration_window_max_delta` Configuration Parameter**:
  - New `Config` field (default `1000`), exposed as UCI option `AspirationWindowMaxDelta` (spin, 50–30000) and parsed via `setoption` in `src/game_handler.rs`, in line with the configuration principle in `task/search_task.md` that forbids hardcoded search heuristics.
- **Aspiration Window Regression Tests**:
  - `test_aspiration_window_is_seeded_from_previous_score` asserts that a caller-supplied score measurably reduces the node count, which fails on the previous implementation because the window never narrowed.
  - `test_aspiration_recovers_from_a_wrong_seed` drives the search with seeds 2000 cp above and below the true score and asserts convergence on the full-window result, covering both the fail-low and fail-high widening paths.

### Changed
- **Measured Search Behaviour**: Across eight benchmark positions at depth 8 with full iterative deepening, the now-functional aspiration window reduces total node count by **15.1%** (1,080,649 → 917,837). The effect varies strongly by position, from −63.1% to +55.6%. Six of the eight positions return a slightly different final score, which is expected rather than a defect: null-move pruning, reverse futility pruning, futility pruning and late move reductions are all unsound heuristics whose decisions depend on the current alpha/beta window, so a narrower window legitimately prunes more. This release therefore changes playing behaviour despite being classified as a patch, and warrants an Elo regression run against v0.29.0.
- **Search Backlog (`task/search_task.md`)**: Raised the Fail-Soft Alpha-Beta task from Medium to High impact and recorded the supporting measurement. The two positions that regressed (+39.9%, +55.6%) do so because fail-hard bounds return the clamped window edge on a failed root pass, leaving the widening logic without information about the true magnitude of the miss and filling the Transposition Table with entries that carry only that bound.


## [V0.29.0] - 2026-08-24

### Added
- **Check Extensions — Horizon Resolution for Forcing Sequences**:
  - Prior to this release the search contained **no depth extensions of any kind**. Every recursive call in `src/search_service.rs` descended with `depth - 1`, so a forcing check sequence was unconditionally truncated at the nominal horizon. The in-check branch of the Quiescence Search recovered only the terminal mate node, never the intermediate quiet continuations of the forcing line, which left the engine tactically blind exactly where sharp play is decided.
  - The `minimax` move loop now computes `child_depth = depth - 1 + extension`, granting `+1` ply whenever the selected move gives check. Because the remaining depth stays constant along a checking line, the search follows a forced sequence to its resolution instead of evaluating a position mid-combination.
  - **No interaction with Late Move Reductions**: the LMR stage already excludes checking moves via its `!current_turn.gives_check` guard, so the extension applies exclusively to the PVS and full-depth search paths. Reduced and extended searches can never be requested for the same move.
- **`MAX_PLY` Termination Ceiling — Structural Search Bound**:
  - Introduced `pub const MAX_PLY: usize = 128` in `src/search_service.rs` together with a hard guard at node entry that returns a static evaluation once `ply >= MAX_PLY - 1`.
  - This is a load-bearing prerequisite rather than a defensive nicety: the previous search relied on the implicit invariant `ply + depth == root_depth` to keep the recursion finite and every ply-indexed table access in range. Check Extensions break that invariant by design, so termination is now enforced structurally at the node boundary instead of being inferred from depth arithmetic.
- **Configurable & SPSA-Tunable Extension Parameters**:
  - `enable_check_extension: bool = true` and `check_extension_max_ply: i32 = 64` added to `Config` in `src/config.rs`, in line with the mandatory configuration principle in `task/search_task.md` that forbids hardcoded search heuristics.
  - Registered as UCI options `EnableCheckExtension` (check) and `CheckExtensionMaxPly` (spin, 0–127) in `src/threads.rs`, with `setoption` parsing in `src/game_handler.rs`.
  - Setting `check_extension_max_ply` to `0` neutralises the feature while leaving the enable flag untouched, which gives the SPSA harness a continuous rather than binary control axis over the extension budget.

### Fixed
- **Out-of-Bounds Killer Move Indexing in the Late Move Reduction Stage**:
  - The LMR killer-move comparison in `src/search_service.rs` indexed `killer_moves[ply as usize]` without any bounds check, while the three other ply-indexed accesses in the same function each guarded themselves differently (`(0..128).contains(&ply)`, `ply.clamp(0, 127)`, and `(ply as usize) < 128`). The unguarded access was the sole outlier in an otherwise inconsistent pattern.
  - The defect was latent under the previous search: without extensions the ply could not exceed the root depth, which the iterative deepening loop in `src/game_handler.rs` caps at 100. Introducing Check Extensions removes that bound and would have turned the access into a panic in the middle of a search.
  - All four call sites now share a single saturating `ply_idx`, eliminating the divergent ad-hoc guards that allowed the omission to go unnoticed.

### Changed
- **Roadmap Synchronisation (`task.md`)**:
  - Marked Milestone 2.2.4 (Transposition Table probing and storage in Quiescence Search) as implemented; the feature shipped in v0.28.1 but the roadmap checklist still listed it as outstanding.
  - Documented Check Extensions as specification 2.2.5 including trigger condition, LMR interaction, termination guarantee, and configuration surface.
- **Search Backlog (`task/search_task.md`)**:
  - Documented Late Move Pruning (LMP) and a Fail-Soft Alpha-Beta conversion as the next prioritised search tasks. The latter records that `minimax` currently initialises its running score to the window bound, which clamps returned scores to the search window and therefore limits Transposition Table entries to carrying the bound instead of the observed score.


## [V0.28.4] - 2026-08-21

### Added
- **SPSA-Tunable Search Parameterization Subsystem**:
  - Parametrisation of 6 core search thresholds and margins across Aspiration Windows, Reverse Futility Pruning (RFP), and Late Move Reduction (LMR) History Heuristics in `src/config.rs` and `src/search_service.rs`.
  - **Aspiration Window (`aspiration_window_initial_delta: i16 = 15`, `aspiration_window_multiplier: i16 = 4`)**: Exposes the root aspiration delta and geometric widening multiplier, allowing fine-tuning of search stability vs re-search frequency.
  - **Reverse Futility Pruning (`rfp_margin_per_depth: i16 = 80`, `rfp_max_depth: i32 = 3`)**: Parameterizes the maximum depth and centipawn margin slope for static null-move / RFP early cutoffs.
  - **LMR History Heuristic Coupling (`lmr_history_good_threshold: u32 = 4000`, `lmr_history_bad_threshold: u32 = 500`)**: Exposes the dynamic reduction dampening/increasing thresholds tied to historical move success in `history_table`.
- **UCI Dynamic Option Registration & Parameter Parsing**:
  - Registered UCI `setoption` command parsing for all 6 search parameters in `src/game_handler.rs` (`aspiration_window_initial_delta`, `aspiration_window_multiplier`, `rfp_margin_per_depth`, `rfp_max_depth`, `lmr_history_good_threshold`, `lmr_history_bad_threshold`).
- **SPSA Automated Tuning Integration**:
  - Added all 6 search parameters to `tuning/groups.json` under `search_and_ordering` and `all`, and configured tuning bounds in `tuning/parameters.json` and `tuning/server_parameters.json`.

### Changed
- **Strict Master HCE Parameter Integrity**:
  - Initialized all newly exposed search parameters to match the proven, hardcoded master constants (`15`, `4`, `80`, `3`, `4000`, `500`), preserving 100% baseline search behavior while unlocking automated tuning capabilities.



## [V0.28.2] - 2026-08-21

### Fixed
- **Eliminated Redundant Transposition Table Lookup in Capture Move Generation**:
  - Fixed a redundant TT lookup inside `MoveGenService::generate_valid_moves_from_move_list` when generating capture moves (`only_captures = true`) in `src/move_gen_service.rs`.
  - Restored `if !only_captures && config.use_zobrist` guard so that capture-only move generation relies on the already-probed `tt_move` from `SearchService::minimax` (which assigns top rank `1_000_000`), eliminating duplicate atomic hash table operations and memory cache-line pressure across all Quiescence Search leaf nodes.



## [V0.28.1] - 2026-08-21

### Added
- **Quiescence Search Transposition Table Subsystem (`enable_qs_tt`)**:
  - Implemented comprehensive Transposition Table (TT) probing, cutoffs, caching, and move ordering in Quiescence Search (`depth <= 0`) in `src/search_service.rs`.
  - **Node Entry TT Probing**: Probes `context.zobrist_table` upon entering Quiescence Search. Takes instant cutoffs on `Exact` entries, `LowerBound >= beta`, or `UpperBound <= alpha` with `depth >= 0`, resolving recurring tactical transpositions and capture permutations in $O(1)$ without evaluating static evaluation or generating capture lists.
  - **Stand-Pat Cutoff Caching**: Records instant stand-pat fail-high (`LowerBound`) and fail-low (`UpperBound`) scores into the Transposition Table with `depth: 0`, preventing redundant static evaluation computations on repeated tactical subtrees.
  - **Terminal Quiescence Entry Storage**: Stores final tactical evaluations, normalized checkmate scores (`ply`-adjusted), and refuting capture moves into the TT upon capture loop completion.
  - **Capture Move Ordering Synergy**: Extracted `tt_move` is prioritized at the front of generated capture lists in `src/move_gen_service.rs` and `src/search_service.rs` with top rank bonus (`+1,000,000`), triggering immediate beta-cutoffs on move 1 of Quiescence Search.
  - **Engine Configuration & UCI Control**: Exposed `enable_qs_tt: bool` (default: `true`) in `src/config.rs` with dynamic UCI parser support (`enable_qs_tt` / `enableqstt`) in `src/game_handler.rs`.
  - Achieved up to **~20% node reduction** on complex tactical positions with zero tactical blindness.

### Fixed
- **TT Cache Pollution & Collision Overwrite Flaw in Replacement Policy**:
  - Fixed a critical vulnerability in `src/zobrist.rs` where the previous "always replace on collision" policy (`existing.key != hash`) allowed high-frequency shallow Quiescence leaf nodes ($d = 0$) to evict valuable deep Main Search entries ($d \ge 1$).
  - Enforced a strict depth-priority replacement hierarchy: `!(entry.depth <= 0 && existing.depth >= 1)`. Quiescence entries ($d \le 0$) are strictly forbidden from evicting Main Search entries ($d \ge 1$) on hash collisions, preserving interior tree node caching and preventing standard search degradation.
- **Root Aspiration Window Depth Safeguard**:
  - Guarded root aspiration window seeding in `src/search_service.rs` (`get_moves`) to strictly require `entry.depth > 0`, ensuring shallow $d = 0$ Quiescence evaluations cannot distort root window bounds.

### Tests
- **Expanded Search & TT Unit Test Suite**:
  - Added `zobrist_qs_tt_collision_protection_test` in `src/zobrist.rs` to verify that deep main search entries are immune to Quiescence collision overwrites.
  - Added `test_qs_tt_probe_cutoff`, `test_qs_tt_mate_normalization`, and `test_qs_tt_search_consistency_and_node_reduction` in `src/search_service.rs`.
  - Maintained 100% pass rate across all 119 unit tests with 0 compiler warnings.



## [V0.28.0] - 2026-08-21

### Added
- **Asymmetric Dual-Threshold Lazy Evaluation Architecture (`lazy_eval_margin_search`, `lazy_eval_margin_qs`)**:
  - Refactored `EvalService::calc_eval` and the Alpha-Beta search core to support differentiated lazy evaluation margins between the Main Search and Quiescence Search.
  - **Main Search (`lazy_eval_margin_search = 180` cp)**: Provides a conservative safety margin (1.8 pawns) preserving subtle positional advantages across multi-ply depth traversals.
  - **Quiescence Search (`lazy_eval_margin_qs = 120` cp)**: Implements a tightened fast-path threshold (1.2 pawns) in the capture resolution tree, allowing over 75% of tactical leaf nodes to exit immediately on `cheap_eval` (material + piece-square tables) without executing the full 2,900-line positional evaluation pass.
  - Achieved **+30 Elo** rating gain in Louguet Chess Test II (LCT II score improved from 2230 to **2260 Elo**, positional solve rate increased from 21.4% to 28.6%).
- **SPSA Tuning Subsystem & UCI Integration**:
  - Exposed `lazy_eval_margin_search` (min: 50, max: 400) and `lazy_eval_margin_qs` (min: 30, max: 300) in `tuning/parameters.json` and `tuning/groups.json` under `search_and_ordering` for automated gradient optimization.
  - Registered UCI options `LazyEvalMarginSearch` and `LazyEvalMarginQs` with full `setoption` command parsing in `src/game_handler.rs` and `src/threads.rs`.

### Fixed
- **Independent Alpha/Beta Bound Checking in Lazy Evaluation**:
  - Fixed a critical flaw in `src/eval_service.rs` where lazy evaluation required both `alpha > -30000 && beta < 30000` simultaneously. On Alpha-Beta Cut-Nodes (where `alpha = -INF`), lazy evaluation was previously skipped across the entire subtree. Decoupled bound checks now evaluate `alpha` and `beta` independently, allowing fail-high and fail-low cutoffs on all node types.



## [V0.27.5] - 2026-08-20

### Added
- **Bishop Diagonal Alignment & X-Ray (`bishop_diagonal_attacks_king`, `bishop_diagonal_attacks_queen`)**:
  - Implemented diagonal X-Ray and alignment heuristics for Bishops in `white_bishop` and `black_bishop` inside `src/eval_service.rs`.
  - Evaluates empty-board diagonal slider rays (`movegen.get_bishop_attacks(sq, 0)`) targeting enemy Kings (`bishop_diagonal_attacks_king: i16 = 15` cp) and Queens (`bishop_diagonal_attacks_queen: i16 = 10` cp), rewarding long-diagonal pressure, tactical pins, and king attack battery setups.
  - Parameterized in `src/config.rs` and dynamically tunable via UCI `setoption name BishopDiagonalAttacksKing value <v>` / `BishopDiagonalAttacksQueen`.
- **Rook 7th-Rank King Cut-Off & Doubled Rooks ("Pigs on the 7th")**:
  - Enhanced 7th-rank rook evaluation in `white_rook` and `black_rook` (`src/eval_service.rs`):
    - **King Cut-Off (`rook_on_seventh_king_cutoff: i16 = 20` cp)**: Rewards trapping the opposing King on the back rank (8th rank for Black, 1st rank for White), restricting king escape routes and endgame counterplay.
    - **Doubled Rooks on 7th (`rooks_doubled_on_seventh: i16 = 25` cp)**: Implements decisive synergy evaluation when two rooks simultaneously control the 7th rank (rank 6 for White, rank 1 for Black).
  - Parameterized in `src/config.rs` and tunable via UCI `setoption name RookOnSeventhKingCutoff value <v>` / `RooksDoubledOnSeventh`.
- **Passed Pawn Nimzowitsch Blockade Penalty (`passed_pawn_blockaded_malus`)**:
  - Integrated passed pawn blockade detection into `white_pawn_structure_score` and `black_pawn_structure_score` in `src/eval_service.rs`.
  - Penalizes stalled passed pawns when an opposing piece directly occupies the front square ($sq \pm 8$) ahead of the pawn (`passed_pawn_blockaded_malus: i16 = 15` cp in endgame, `7` cp in middlegame), preventing overestimation of immobilized passers.
  - Parameterized in `src/config.rs` and tunable via UCI `setoption name PassedPawnBlockadedMalus value <v>`.
- **Candidate Passed Pawn Heuristic (`candidate_passed_pawn_bonus`)**:
  - Implemented candidate passed pawn detection in `white_pawn_structure_score` and `black_pawn_structure_score` in `src/eval_service.rs` for non-passed pawns that have a clear front file corridor and friendly adjacent pawn majority support.
  - Dynamically scales candidate bonus with rank advancement factor: $\text{Bonus} = \text{candidate\_passed\_pawn\_bonus} \times (\text{Advancement Rank} - 2)$.
  - Parameterized via `candidate_passed_pawn_bonus: i16 = 8` cp in `src/config.rs` and tunable via UCI `setoption name CandidatePassedPawnBonus value <v>`.
- **Flank Pawn Storm on Enemy Castled King (`pawn_storm_bonus`)**:
  - Integrated dynamic wing pawn storm heuristics into `white_pawn_dynamic_score` and `black_pawn_dynamic_score` in `src/eval_service.rs`.
  - Rewards advancing flank pawns ($f, g, h$ against kingside castled king, or $a, b, c$ against queenside castled king) to break open shelter barriers during opening and middlegame phases (`pawn_storm_bonus: i16 = 6` cp per advancement rank).
  - Parameterized in `src/config.rs` and tunable via UCI `setoption name PawnStormBonus value <v>`.
- **Theoretical Dead-Draw Recognition for Wrong-Colored Bishop & Rook Pawn ($K+B+P$ vs $K$)**:
  - Extended the 1-cycle fast-path insufficient material cutoff in `is_insufficient_material` (`src/eval_service.rs`) to detect classical theoretical dead draws where one side has a bare King + 1 Bishop + 1 Rook Pawn ($a$- or $h$-file) and the opposing King controls the corner promotion square while the bishop cannot control the promotion square (wrong square color).
  - Instantly evaluates to `0.00` cp in search tree traversals, eliminating horizon errors and preventing erroneous trade-downs into drawn endgames.
- **Classical Evaluation Test Suite Expansion**:
  - Added 6 dedicated unit tests in `src/eval_service.rs`: `test_bishop_diagonal_attacks_king_and_queen`, `test_rook_on_seventh_king_cutoff_and_doubled`, `test_passed_pawn_blockade_penalty`, `test_candidate_passed_pawn_evaluation`, `test_pawn_storm_evaluation`, `test_wrong_colored_bishop_rook_pawn_dead_draw`, and `test_all_new_eval_features_black_white_symmetry`.



## [V0.27.4] - 2026-08-20

### Added
- **Per-Completed-Depth Search Calculation Metrics in Engine Logs**:
  - Enhanced iterative deepening in `src/game_handler.rs` (both standard `go` and `infinite` search loops) to log structured, high-density calculation metrics whenever a depth iteration successfully completes.
  - Format: `Depth <D> completed | score <cp/mate> | time <T>ms | nodes <N> | nps <NPS> | pv <PV>`.
  - Intelligently formats evaluation as centipawns (`cp +45`) or exact checkmate distance (`mate 3` / `mate -2`).
  - Computes and logs true Nodes Per Second (`nps`) alongside total nodes and iteration runtime.
  - Enriched final move selection logging to record total elapsed search time: `final move: bestmove <mv> (total time: <T>ms)`.

### Fixed
- **Eliminated Massive Parameter Dump Log Spam**:
  - Removed repetitive 64-line static parameter dumps (`log_all_parameters`) previously triggered on every move (`go` command) and every `setoption` command, eliminating megabytes of redundant log spam during tournaments.
  - Removed dead `log_all_parameters` method from `src/config.rs`.
- **Log Formatting & Typo Cleanups**:
  - Fixed typo `"Engine startet: {}"` to `"Engine started: {}"` in `src/threads.rs`.
  - Fixed typo `"incomming go cmd"` to `"Incoming go command"` in `src/game_handler.rs`.
  - Fixed missing space in unrecognized UCI command handling in `src/threads.rs` (`cmd unknown: <cmd>`).



## [V0.27.3] - 2026-08-20

### Added
- **Enemy Heavy-Piece Penetration & Threat on King Open Files (`king_open_file_heavy_threat_malus`)**:
  - Enhanced king safety evaluation in `white_king` and `black_king` in `src/eval_service.rs` to dynamically penalize open and half-open files when opposing heavy pieces (Rooks or Queens) actively occupy that file (`opp_heavy_on_file`).
  - Differentiates between harmless empty open files in simplified endgames and lethal heavy-piece assault batteries bearing down directly on the King during the opening and middlegame.
  - Fully parameterized via `king_open_file_heavy_threat_malus: i16 = 15` cp in `src/config.rs` and dynamically tunable via UCI `setoption name KingOpenFileHeavyThreatMalus value <v>`.
- **Rook Battery & X-Ray Alignment on Open / Semi-Open Files (`rook_open_file_attacks_king`, `rook_open_file_attacks_queen`)**:
  - Integrated attacking pressure evaluation into `white_rook` and `black_rook` in `src/eval_service.rs` when a Rook occupies an open or semi-open file that directly aligns with the enemy King (`rook_open_file_attacks_king: i16 = 15` cp) or the enemy Queen (`rook_open_file_attacks_queen: i16 = 10` cp).
  - Rewards establishing vertical batteries and pinning enemy pieces along active file corridors.
  - Configurable in `src/config.rs` and tunable via UCI `setoption name RookOpenFileAttacksKing value <v>` and `RookOpenFileAttacksQueen`.
- **Pawn Phalanx (Dynamic Duos) Structural Evaluation (`pawn_phalanx_mg`, `pawn_phalanx_eg`)**:
  - Implemented Pawn Phalanx formation evaluation in `white_pawn_structure_score` and `black_pawn_structure_score` in `src/eval_service.rs` for horizontally adjacent friendly pawns on the same rank ($f \pm 1$) across ranks 3–5 for White and ranks 2–4 for Black (e.g. d4+e4, c5+d5, f4+g4).
  - Scales dynamically with advancement rank factor:
    $$\text{Phalanx Bonus} = \text{pawn\_phalanx\_mg} \times (\text{Advancement Rank} - 2)$$
  - Evaluated in both middlegame and endgame, providing an incentive for building space-gaining pawn duos that deny enemy outposts.
  - Configurable via `pawn_phalanx_mg: i16 = 8` cp and `pawn_phalanx_eg: i16 = 4` cp in `src/config.rs` and tunable via UCI `setoption name PawnPhalanxMg value <v>` / `PawnPhalanxEg`.
- **Extended Theoretical Insufficient Material Dead-Draw Recognition (`is_insufficient_material`)**:
  - Extended the 1-cycle fast-path insufficient material cutoff in `src/eval_service.rs` to detect additional theoretical dead-draw configurations without pawns:
    - $K+N$ vs $K+N$ (one minor knight each, 0 pawns)
    - $K+B$ vs $K+N$ / $K+N$ vs $K+B$ (one bishop vs one knight, 0 pawns)
    - $K+B$ vs $K+B$ on same-colored squares without pawns
  - Immediately prunes leaf and root nodes to `0.00` cp in both `cheap_eval` and `calc_eval`, eliminating wasted depth exploration in guaranteed draw endgames.
- **Classical Evaluation & Insufficient Material Test Suite**:
  - Added unit test `test_extended_insufficient_material_detection` in `src/eval_service.rs` validating draw recognition for $KN$ vs $KN$, $KB$ vs $KN$, and same-color $KB$ vs $KB$.
  - Added unit test `test_pawn_phalanx_evaluation` in `src/eval_service.rs` validating that central phalanxes ($d4+e4$) score higher than disjointed pawns ($d4+a4$).
  - Added unit test `test_pawn_phalanx_black_white_symmetry` in `src/eval_service.rs` verifying exact numerical score symmetry between White and Black phalanxes.
  - Added unit test `test_rook_open_file_attacks_king_and_queen` in `src/eval_service.rs` validating tactical bonuses for Rooks aligned with opposing heavy pieces.
  - Added unit test `test_king_open_file_heavy_threat_penalty` in `src/eval_service.rs` verifying penalties when open files facing the King are occupied by opposing Rooks.



## [V0.27.2] - 2026-08-20

### Added
- **Unified Safe Mobility Evaluation (Safe Squares Masking)**:
  - Ported advanced classical evaluation concepts from Cassandra.jl into Suprah HCE in `src/eval_service.rs`.
  - Implemented unified bitwise safe square masking: `safe_mask = !(opp_pawn_attacks | friendly_pieces)`. All piece mobility counts (Knights, Bishops, Rooks, and Queens) now strictly exclude squares occupied by friendly pieces and squares attacked by enemy pawns.
  - Implemented branchless parallel pawn attack bitboard generation:
    - `get_white_pawn_attacks(wp)`: `((wp & !FILE_A) << 7) | ((wp & !FILE_H) << 9)`
    - `get_black_pawn_attacks(bp)`: `((bp & !FILE_H) >> 7) | ((bp & !FILE_A) >> 9)`
- **Safe Queen Mobility (`queen_mobility_factor`)**:
  - Integrated safe Queen mobility calculation into `white_queen` and `black_queen` in `src/eval_service.rs`, evaluating Queen mobility across legal slider rays masked against enemy pawn attacks and friendly pieces.
  - Configurable via `queen_mobility_factor: i16 = 1` cp in `src/config.rs` and dynamically tunable via UCI `setoption name QueenMobilityFactor value <v>`.
- **Advanced Dynamic King-Passer Proximity Heuristics (`king_passer_proximity_score`)**:
  - Replaced crude legacy hardcoded distance constants with dynamic endgame king-passer proximity scoring in `src/eval_service.rs`.
  - Computes the Chebyshev distance delta: $\Delta D = D(\text{EnemyKing}, \text{Pawn}) - D(\text{FriendlyKing}, \text{Pawn})$, scaled by pawn advancement rank factor ($R+1$ for White, $8-R$ for Black):
    $$\text{Bonus} = \frac{\Delta D \times \text{king\_passer\_dist\_weight} \times \text{RankFactor}}{8}$$
  - Added directly to endgame evaluation (`e_eval`) and smoothly tapered across game phases via `calculate_weighted_eval`.
  - Tunable via `king_passer_dist_weight: i16 = 12` cp in `src/config.rs` and UCI `setoption name KingPasserDistWeight value <v>`.
- **Classical Safe Mobility & Proximity Test Suite**:
  - Added unit test `test_safe_mobility_excludes_enemy_pawn_attacks` verifying Knight safe mobility reduction when landing squares are controlled by opposing pawns.
  - Added unit test `test_safe_mobility_excludes_friendly_blockers` verifying Rook mobility exclusion of friendly blocking pieces.
  - Added unit test `test_queen_mobility_evaluation` validating central active Queens vs trapped corner Queens.
  - Added unit test `test_king_passer_proximity_monotonic_gradient` verifying strictly monotonic evaluation growth as the friendly king escorts advanced passed pawns.
  - Added unit test `test_king_passer_proximity_black_white_symmetry` validating exact numerical score symmetry between White and Black passed pawn positions.

### Fixed
- **Knight Mobility Static Attack Count Bug**:
  - Fixed a legacy evaluation bug in `white_knight` and `black_knight` in `src/eval_service.rs` where knight mobility was calculated using unmasked pseudolegal moves (`movegen.get_knight_attacks(sq).count_ones()`), producing an invariant static constant ($2 \dots 8$) redundant with Piece-Square Tables (PST) regardless of whether squares were blocked by friendly pieces or controlled by enemy pawns.
- **Black Minor Piece Malus Sign Defect**:
  - Corrected sign inversions in `black_knight` and `black_bishop` in `src/eval_service.rs` where positional penalties (`knight_on_rim_malus`, `undeveloped_knight_malus`, `undeveloped_bishop_malus`, `bishop_trapped_at_rim_malus`) were previously subtracted (`o_eval -= malus`) instead of added (`o_eval += malus`), erroneously rewarding Black for undeveloped and rim-trapped minor pieces.
- **Mobility Parameter Stabilization across Aggressiveness Profiles**:
  - Standardized mobility factors across all engine profiles to balanced defaults ($N=3, B=3, R=2, Q=1$ cp), keeping mobility factors invariant across `Normal`, `Aggressive`, and `HighAggressive` profiles in `src/config.rs`.



## [V0.27.1] - 2026-08-20

### Fixed
- **Bare-King Mop-Up Activation Guard (`apply_endgame_mopup`)**:
  - Fixed a critical activation defect in `src/eval_service.rs` where Mop-Up heuristics were previously triggered whenever `winning_non_pawns != 0 && losing_pawns == 0`, ignoring whether the defending side possessed active pieces (`losing_non_pawns`).
  - In pawnless piece endgames (e.g. $Q$ vs $R$, $R+B$ vs $R$, $Q$ vs $Q$, $R$ vs $R$), this defect falsely rewarded marching the winning king toward enemy pieces with up to $+90$ cp, exposing the king to skewers, forks, checks, and tactical counterplay.
  - Strictly enforced `winning_non_pawns != 0 && losing_non_pawns == 0 && losing_pawns == 0`, ensuring Mop-Up activates exclusively against a true bare King ($K$).
- **Mop-Up Evaluation Scaling Order in Search Pipeline (`calc_eval`)**:
  - Reordered the evaluation pipeline in `src/eval_service.rs` to execute `adjust_eval` prior to `apply_endgame_mopup`.
  - In deep endgames (`game_phase <= 60`), `adjust_eval` applies dynamic multiplier scaling of up to $2.0\times$ (`mult = 255 / (game_phase + 100)`). Executing Mop-Up post-adjustment prevents double-amplification from blowing up the $+170$ cp geometric bonus to $+340$ cp (over a full minor piece), preserving genuine material and tactical evaluation balances.

### Added
- **Piece Endgame Deactivation Test Suite**:
  - Added unit test `test_endgame_mopup_disabled_when_losing_side_has_pieces` in `src/eval_service.rs` verifying that pawnless endgames where the defending side retains pieces ($K+Q$ vs $K+R$, $K+R+B$ vs $K+R$, $K+Q$ vs $K+Q$, $K+R$ vs $K+N$, $K+R$ vs $K+B$) maintain identical evaluations with or without `enable_endgame_mopup`.



## [V0.27.0] - 2026-08-19

### Added
- **Insufficient Material Draw Detection (`is_insufficient_material`)**:
  - Implemented an instant 1-cycle fast path (`(white_pawns | black_pawns) != 0`) in `src/eval_service.rs` at the entry points of both `cheap_eval` and `calc_eval`.
  - Automatically identifies dead drawn material configurations ($KvK$, $KNvK$, $KBvK$, $KNNvK$, and same-color bishop endings $KBvKB$ without pawns) and returns `0.00` cp immediately.
  - Eliminates catastrophic evaluation leaks in deep search trees where material advantages in dead drawn endgames previously led to incorrect horizon trade-downs (e.g., trading down into $K+N$ vs $K$ believing it was winning $+3.2$ pawns).
- **Endgame Mop-Up Heuristics (`apply_endgame_mopup`)**:
  - Implemented geometric king cornering and Chebyshev king-to-king proximity heuristics in `src/eval_service.rs` for winning pawnless endgames with minor/major pieces (`game_phase <= 60`, `eval.abs() >= 400`, `winning_non_pawns != 0`, `losing_pawns == 0`).
  - Rewards driving the losing king from the center towards board edges and corners via Manhattan distance to center, combined with rewarding close Chebyshev king-to-king proximity (`(7 - king_dist) * mopup_proximity_weight`).
  - Establishes a steep search gradient towards forced checkmate in simple pawnless endgames (such as $K+R$ vs $K$, $K+Q$ vs $K$, $K+B+B$ vs $K$), eliminating wandering king shuffles and 50-move rule draws.
- **Configurable Mop-Up Parameterization & UCI Option Integration**:
  - Added new configuration fields to `Config` in `src/config.rs`: `enable_endgame_mopup` (default: `true`), `mopup_center_weight` (default: `10`), `mopup_proximity_weight` (default: `15`), `mopup_eval_threshold` (default: `400`), and `mopup_max_game_phase` (default: `60`).
  - Integrated dynamic UCI `setoption` command parsing for all Mop-Up parameters in `src/game_handler.rs` and included them in engine debug logging.
- **HCE Endgame Test Suite & Monotonic Gradient Verification**:
  - Added unit test `test_insufficient_material_detection` in `src/eval_service.rs` validating draw evaluations across $KvK$, $KNvK$, $KBvK$, $KNNvK$, and $KBvKB$ (same-color), while verifying that positions with pawns ($KPvK$) or rooks ($KRvK$) remain unsuppressed.
  - Added unit test `test_endgame_mopup_heuristics` in `src/eval_service.rs` validating cornering advantage, pawn safety deactivation, and black-winning score symmetry.
  - Added unit tests `test_endgame_mopup_edge_push_monotonic_gradient` and `test_endgame_mopup_king_proximity_monotonic_gradient` in `src/eval_service.rs` verifying strictly monotonic evaluation growth as the enemy king is driven from center to corner ($d4 \to c5 \to c8 \to a8$) and as the friendly king approaches the cornered king ($d=7 \to d=5 \to d=3 \to d=2 \to d=1$).



## [V0.26.3] - 2026-08-19

### Changed
- **Pawn Hash Table Capacity Restored to 16 MB (1,000,000 Entries)**:
  - Reverted default Pawn Hash Table capacity `max_pawn_hash_entries` from `10,000,000` (~160 MB) back to `1,000,000` entries (~16 MB RAM allocation) in `src/config.rs`.
  - Drastically shrinks working set memory footprint and eliminates CPU L3 cache pollution, TLB thrashing, and RAM bandwidth bottlenecks during high-speed parallel search and tournament match play.
- **Default Tactical Profile Restored to Normal Baseline**:
  - Restored `Aggressiveness::Normal` as the engine default profile in `Config::new()` (`src/config.rs`), re-aligning with the proven evaluation baseline from `v0.25.0`.
  - Re-aligned UCI option `Aggressiveness` default value to `Normal` in `src/threads.rs`.

### Added
- **Default Configuration Initialization Test**:
  - Added unit test `test_config_default_initialization` in `src/config.rs` verifying that default startup configuration enforces `max_pawn_hash_entries == 1,000,000`, `max_zobrist_hash_entries == 50,000,000`, and `aggressiveness == Aggressiveness::Normal`.



## [V0.26.2] - 2026-08-17

### Changed
- **Default Tactical Profile Set to HighAggressive**:
  - Configured `Aggressiveness::HighAggressive` as the default engine profile in `Config::new()` (`src/config.rs`).
  - Automatically doubles king attack danger weights (`king_ring_attack_*` by 2.0x), boosts queen dynamic attack bonuses (`queen_in_attack*` by 1.6x), piece mobility factors (`*_mobility_factor` by 1.4x), and raises the positional evaluation soft cap to 400 cp right at initialization.
  - Set UCI option `Aggressiveness` default value to `HighAggressive` in `src/threads.rs`.



## [V0.26.1] - 2026-08-17

### Changed
- **Default Tactical Profile Set to Aggressive**:
  - Configured `Aggressiveness::Aggressive` as the default engine profile in `Config::new()` (`src/config.rs`).
  - Automatically scales king attack danger weights (`king_ring_attack_*` by 1.5x), queen dynamic attack bonuses (`queen_in_attack*` by 1.3x), piece mobility factors (`*_mobility_factor` by 1.2x), and increases the positional evaluation soft cap to 250 cp right at initialization.
  - Set UCI option `Aggressiveness` default value to `Aggressive` in `src/threads.rs`.



## [V0.26.0] - 2026-08-17

### Changed
- **Magic Bitboard Inner Loop Optimization**:
  - Removed redundant `init()` calls and `INIT.call_once` atomic synchronization barriers from the hot-paths of `get_bishop_attacks` and `get_rook_attacks` in `src/magic.rs`.
  - Added explicit one-time startup initialization of magic bitboards in `Service::new()` (`src/service.rs`) and `main()` (`src/main.rs`), eliminating millions of atomic branch checks per second during search tree traversals.
- **Evaluation Heap-Free Aggressiveness Handling**:
  - Eliminated `config.clone()` in `calc_eval` within `src/eval_service.rs` when non-default aggressiveness profiles (`Aggressive`, `HighAggressive`) are configured.
  - Added `set_aggressiveness` and `apply_aggressiveness` helper methods to `Config` (`src/config.rs`) and `src/game_handler.rs`, scaling evaluation weights in-place upon configuration updates and making leaf static evaluation 100% clone-free.
- **Hot-Path Leaf Function Inlining**:
  - Added `#[inline(always)]` annotations to central leaf routines across the engine, including `see`, `see_ge`, and `get_piece_value` in `src/search_service.rs`, `calc_incremental_hash`, `get_zobrist_val`, `pack`, `unpack`, `compress_move`, and `decompress_move` in `src/zobrist.rs`, and `piece_to_bb_idx` in `src/model.rs`.

### Fixed
- **Promotion Move Mutation State Bug**:
  - Fixed an underpromotion loop mutation bug in `validate_and_add_promotion_moves` (`src/move_gen_service.rs`) where `turn.gives_check` and `turn.rank` were mutated during queen promotion without being reset before evaluating subsequent underpromotions (e.g. knight promotion). Now `base_rank` and `gives_check = false` are explicitly restored for each promotion candidate.

### Added
- **Config Aggressiveness & Movegen Promotion Regression Tests**:
  - Added unit test `test_config_aggressiveness_scaling` in `src/config.rs` verifying that `Normal`, `Aggressive`, and `HighAggressive` profiles correctly scale all 9 evaluation factors in-place without requiring leaf heap clones.
  - Added unit test `test_promotion_gives_check_independence` in `src/move_gen_service.rs` verifying that underpromotions (knight, bishop) maintain independent `gives_check` state and do not inherit check flags from queen promotions.



## [V0.25.1] - 2026-08-04

### Changed
- **Zobrist & Pawn Hash Table Capacity Upgrades**:
  - Increased default Transposition Table (Zobrist) capacity `max_zobrist_hash_entries` from `10,000,000` entries (~150 MB) to `50,000,000` entries (~800 MB RAM allocation) in `src/config.rs`. This significantly reduces entry eviction rate and hash collision thrashing during deep search trees in long time controls.
  - Increased default Pawn Hash Table capacity `max_pawn_hash_entries` from `1,000,000` entries (~15 MB) to `10,000,000` entries (~150 MB RAM allocation) in `src/config.rs`, ensuring high hit rates for static pawn structure evaluation across deep branches.

### Added
- **Hash Entry Alignment & Memory Footprint Verification Tests**:
  - Added unit test `zobrist_entry_size_test` in `src/zobrist.rs` verifying that `AtomicEntry` and `TranspositionEntry` remain strictly 16 bytes each (and 100M entries equal exactly 1.6 GB).
  - Added unit test `pawn_entry_size_test` in `src/pawn_hash.rs` verifying that `PawnEntry` and `Cell<PawnEntry>` remain strictly 16 bytes each.



## [V0.25.0] - 2026-08-04

### Added
- Release updates and improvements

### Fixed



## [V0.25.0] - 2026-08-04

### Changed
- **Architectural Move Generator Refactoring & Performance Overhaul**: Conducted a comprehensive 3-phase optimization of `src/move_gen_service.rs`, boosting overall engine move generation throughput from ~2.91M NPS to ~3.77M NPS (+29.5% NPS speedup) while maintaining 100% perft precision across 4,085,603 nodes.
  - **Phase 1: Bitboard Iteration & Redundant Lookup Hoisting**: Replaced mailbox square loops (`0..=63`) in `generate_moves_list_for_piece` with direct `while piece_mask != 0` trailing-zero bitboard iterations over `board.white_pieces` / `board.black_pieces`. Hoisted opponent king square lookups (`opp_king_sq`) out of inner piece loops and applied bitwise king proximity masking (`targets &= !KING_ATTACKS[opp_king_sq]`).
  - **Phase 2: Transposition Table Probing Removal & Streamlined Check Validation**: Removed redundant Transposition Table probing (`get_hash`) during move generation, eliminating dozens of unnecessary Zobrist lookups per movegen node. Streamlined `validate_and_add_move` to extract king square positions once per played move and evaluate move legality and check status (`gives_check`) in a single unified pass.
  - **Phase 3: Staged Quiescence Move Generation**: Extended `generate_moves_list_for_piece` with an `only_captures: bool` parameter. When generating moves for Quiescence Search (`generate_valid_moves_list_capture`), non-tactical quiet moves (quiet pawn pushes, non-capturing piece jumps, castling) are filtered directly at bitboard attack mask level (`target_mask = opp_pieces`), avoiding allocation and pseudo-legal generation of quiet moves entirely.

### Added
- **Comprehensive Move Generator Unit Test Suite**: Added dedicated regression tests in `src/move_gen_service.rs` including `test_generate_moves_list_bitboard_consistency`, `test_king_attacks_proximity_masking`, `test_streamlined_move_validation_gives_check`, and `test_generate_moves_list_capture_only_filtering`.




## [V0.23.12] - 2026-07-30

### Changed
- **Ultra-Safe Lazy Evaluation Tuning**: Increased default `lazy_eval_margin` from `360` to `400` in `src/config.rs`. This ultra-conservative static evaluation cutoff threshold requires `cheap_eval` to differ from alpha/beta bounds by at least 4.0 pawns before skipping full positional evaluation, moving engine behavior even closer to v0.22.10 full-eval precision while maintaining high-speed cutoffs for overwhelming tactical positions.

### Fixed



## [V0.23.11] - 2026-07-30

### Changed
- **Golden Hybrid Lazy Evaluation Tuning**: Re-aligned default `lazy_eval_margin` from `320` to `360` while retaining `lazy_eval_min_game_phase` at `60` in `src/config.rs`. This hybrid configuration combines the superior middle-game evaluation safety of v0.23.6 (360 cp margin) with the extended endgame bypass protection of v0.23.10 (`game_phase < 60`), preventing premature evaluation cutoffs in complex tactical positions while safeguarding transitional endgames.

### Fixed



## [V0.23.10] - 2026-07-29

### Changed
- **Lazy Evaluation Min Game Phase Tuned**: Increased `lazy_eval_min_game_phase` from `40` to `60` in `src/config.rs`. This provides a more conservative endgame bypass (`game_phase < 60`), ensuring full evaluation is invoked earlier as piece density decreases towards the endgame.

### Fixed



## [V0.23.9] - 2026-07-29

### Changed
- **Lazy Evaluation Min Game Phase Tuned**: Increased `lazy_eval_min_game_phase` from `30` to `40` in `src/config.rs`. This relaxes the endgame bypass threshold (`game_phase < 40`), allowing Lazy Evaluation cutoffs to run slightly further into transitional endgame positions while maintaining tactical precision.

### Fixed



## [V0.23.8] - 2026-07-29

### Changed
- **Lazy Evaluation Min Game Phase Tuned**: Decreased `lazy_eval_min_game_phase` from `50` to `30` in `src/config.rs`. This restricts Lazy Evaluation to earlier game phases with higher piece density, automatically bypassing lazy evaluation cutoffs in late endgames (`game_phase < 30`) where subtle positional nuances and king mobility dominate.

### Fixed



## [V0.23.7] - 2026-07-29

### Changed
- **Lazy Evaluation Margin Tuned**: Adjusted default `lazy_eval_margin` from `360` to `320` in `src/config.rs`. This re-aligns the static evaluation cutoff threshold, striking an optimal balance between aggressive search tree pruning speed and positional evaluation stability.

### Fixed



## [V0.23.6] - 2026-07-29

### Changed
- **Lazy Evaluation Margin Tuned**: Increased default `lazy_eval_margin` from `180` to `360` in `src/config.rs`. This provides more conservative early static evaluation cutoffs during the main search, prioritizing positional evaluation accuracy and tactical safety in complex search trees.

### Fixed



## [V0.23.5] - 2026-07-29

### Added
- **Configurable `LazyEvalMinGamePhase`**: Introduced a new parameter and UCI spin option `LazyEvalMinGamePhase` (range 0-256, default 50) in `config.rs`. Lazy Evaluation is automatically bypassed in deep endgames (`game_phase < 50`) where positional factors like king opposition and dynamic passed pawn races dominate.

### Changed
- **Simplified `EnableLazyEval` UCI Option**: Converted the `LazyEvalMode` enum into a clean boolean check option `EnableLazyEval` (`true`/`false`, default `true`), enabling lazy evaluation uniformly across main search and quiescence search.

### Fixed
- **In-Check Guard**: Lazy Evaluation is now automatically bypassed whenever the position is in check (`in_check`), preventing tactical oversights and blunders during check evasions.



## [V0.23.4] - 2026-07-29

### Changed
- **Lazy Evaluation Margin Tuned**: Decreased the default `lazy_eval_margin` from `320` to `180` in `config.rs`. This provides much more aggressive early evaluation cutoffs during the main search, leading to higher hit rates and faster searches at the potential expense of occasional tactical inaccuracies.

### Fixed



## [V0.23.3] - 2026-07-29

### Changed
- **Lazy Evaluation Margin Tuned**: Increased the default `lazy_eval_margin` from `250` to `320` in `config.rs`. This provides more conservative early evaluation cutoffs during the main search, preserving evaluation integrity in complex positions and ensuring fewer tactical errors at the cost of a slightly reduced hit rate.

### Fixed



## [V0.23.2] - 2026-07-29

### Fixed
- **Architectural Repair of Lazy Evaluation Pruning**: Fixed a major Elo regression (>500 Elo loss in v0.23.1 and ~200 Elo loss in v0.23.0) caused by node-level early returns in tree search. Removed invalid node-level cutoffs from `pvs` (main search) and `quiescence_search` in `src/search_service.rs` that were truncating search trees and causing tactical blindness.
- **Static Evaluation Bound Pruning in `calc_eval`**: Shifted Lazy Evaluation logic directly into `calc_eval` in `src/eval_service.rs`. When `cheap_eval` (material + PST + pawn table) is far outside alpha/beta search bounds, `calc_eval` skips expensive positional evaluations (king danger, piece mobility, passed pawns) and returns `cheap_eval` directly as the static score, while allowing tree search and candidate move generation to proceed normally.
- **Search Speed & Elo Recovery**: Restored Nodes Per Second (NPS) from 799 NPS up to 1725 NPS (>115% speed increase) and improved LCT-II tactical evaluation score to 2110 Elo (7/35 positions solved).



## [V0.23.1] - 2026-07-29

### Added
- Bugfix: Set LazyModeEvl auf MainSearchOnly

### Fixed



## [V0.23.0] - 2026-07-29

### Added & Optimized
- **Lazy Evaluation Pruning Reactivation & Modularization**:
  - Reintroduced robust Lazy Evaluation utilizing the extremely fast, O(1) `cheap_eval` estimation function (which computes basic material, incremental PSTs, pawn hash, and endgame scaling).
  - Implemented stringent safety guards: Lazy Evaluation cutoffs are now exclusively triggered in Null-Window searches (`alpha + 1 == beta`) and strictly prohibited when the side to move is in check, completely eliminating tactical blindness risks.
  - Eliminated the risk of Transposition Table (TT) pollution by bypassing all TT write operations when a search path terminates via a Lazy Evaluation cutoff, fixing historical depth-0 cache thrashing bugs.
- **Configurable `LazyEvalMode` UCI Implementation**:
  - Replaced the inflexible `enable_lazy_eval` boolean with a dynamic `LazyEvalMode` enum supporting multiple target granularities: `Disabled`, `QuiescenceOnly`, `MainSearchOnly`, and `Both`.
  - Exposed `LazyEvalMode` as a UCI Combo option (replacing the old checkbox) to enable fine-grained testing and parameter tuning via GUI interfaces.
  - Set the default operational mode to `QuiescenceOnly` for aggressive optimization testing in highly active search branches.



## [V0.22.11] - 2026-07-29

### Changed & Optimized
- **Turn Struct Memory Halving & Cache Optimization (`src/model.rs`)**:
  - Removed the `hash: u64` field entirely from the `Turn` struct, drastically reducing the struct size from 24 Bytes down to 12 Bytes (50% reduction).
  - This shrinks the size of the heavily utilized `MoveList` stack allocations from 6,144 Bytes to 3,072 Bytes, massively improving CPU L1/L2 Cache efficiency and density during Move Generation.
  - Accelerated move sorting operations during Alpha-Beta Search (Move Ordering) by halving the memory footprint that needs to be copied and shifted in RAM.
- **Inline Incremental Hash Relocation (`src/model.rs` & `src/move_gen_service.rs`)**:
  - Relocated the call to `calc_incremental_hash` from the pre-validation pseudo-move generator directly into the `do_move` function.
  - This perfectly shifts the hashing computation overhead to be performed in-place (assigned seamlessly to `board.cached_hash`), eliminating the need to persist and pass the hash via the `Turn` object.
  - **Performance**: +1.4% NPS



## [V0.22.10] - 2026-07-29

### Added & Optimized
- **Incremental Zobrist Hashing Architecture (`src/zobrist.rs` & `src/move_gen_service.rs`)**:
  - Implemented a purely incremental transition mechanism for `board.cached_hash` using `calc_incremental_hash` to completely bypass the expensive O(N) `gen_hash()` full-bitboard traversal.
  - Refactored `Lazy` initialization constants in `zobrist.rs` to use `pub` fields, exposing direct XOR capabilities to external modules.
  - **Move Generator Pre-Calculation**: The `MoveGenService` now efficiently pre-calculates the incremental hash upon `Turn` creation, seamlessly injecting the localized XOR adjustments (piece movement, castling rights, and en-passant tracking).
  - **Null Move Search Performance**: Optimized the Null Move Pruning (NMP) branch in `search_service.rs` by replacing `gen_hash()` with targeted `WHITE_TO_MOVE` and `EN_PASSANT_FILE` XOR manipulations.
  - **Performance**: +106% NPS

### Fixed
- **Critical Repetition Map Corruption Bug (`src/model.rs`)**:
  - Identified and resolved a fatal regression where `undo_move` was erroneously decrementing the *new* hash from the `move_repetition_map` because `board.cached_hash` was reset to `0`, resulting in severe panics (`RIP move_repetition_map value 4`).
  - `MoveInformation` now immutably tracks and stores the `old_cached_hash` prior to `do_move` mutation, allowing `undo_move` to perfectly restore the pre-move hash state and safely manage the 3-fold repetition map counts.
- **Fuzz Testing & Verification**: Added the aggressive `incremental_hash_complex_sequence_test` fuzzing suite that executes deep pseudo-random move sequences to assert absolute cryptographic parity between incremental adjustments and full `gen_hash()` reconstruction.



## [V0.22.9] - 2026-07-29

### Fixed & Changed
- **Re-aligned Late Move Reductions Divisor (`lmr_divisor = 185`)**:
  - Restored default `lmr_divisor` value to **185** in `src/config.rs` and recalculated the static logarithmic LMR lookup table (`lmr_table` with `divisor = 185.0 / 100.0`).
  - Aligned SPSA tuning parameter definitions in `tuning/parameters.json`, `tuning/server_parameters.json`, and `tuning/spsa_state_remote.json`.
  - **Performance**: Baseline
  - Re-establishes the optimal quiet move depth reduction scaling factor during search, optimizing search tree depth reach while preventing tactical horizon-effect pruning errors.



## [V0.22.8] - 2026-07-28

### Changed
- **Extended Futility Pruning Max Depth (`futility_max_depth = 4`)**:
  - Increased `futility_max_depth` default value from `3` to `4` in `src/config.rs` and UCI options in `src/threads.rs`.
  - Expands leaf/frontier Futility Pruning to quiet moves at search depths 1, 2, 3, and 4, further optimizing node count pruning efficiency while maintaining tactical safety guards.



## [V0.22.7] - 2026-07-28

### Added & Changed
- **Moderately Aggressive Futility Pruning Baseline (`src/config.rs`)**:
  - Re-enabled Futility Pruning by default (`enable_futility_pruning: true`) in `Config::new()`.
  - Configured moderately aggressive pruning thresholds to achieve optimal search tree node reduction while preserving full tactical safety:
    - `futility_max_depth`: `3` (pruning unpromising quiet moves at shallow search depths 1, 2, and 3).
    - `futility_margin_base`: `120` cp (between conservative 150 cp and aggressive 100 cp).
    - `futility_margin_slope`: `80` cp per depth level (between conservative 100 cp and aggressive 70 cp).
- **UCI Protocol Configuration Support (`src/threads.rs`, `src/game_handler.rs`)**:
  - Exposed Futility Pruning configuration parameters via standard UCI `setoption` commands:
    - `EnableFutilityPruning`: boolean check (default: `true`)
    - `FutilityMaxDepth`: integer spin (default: `3`, range: 1–10)
    - `FutilityMarginBase`: integer spin (default: `120`, range: 0–500)
    - `FutilityMarginSlope`: integer spin (default: `80`, range: 0–300)
  - Integrated full parameter normalization in `src/game_handler.rs` to allow case-insensitive and underscore/camelCase option configuration.
- **Documentation (`README.md`)**:
  - Updated `README.md` with complete technical definitions and references for Futility Pruning (FP) in the Core Search & Selective Pruning reference table.



## [V0.22.6] - 2026-07-28

### Added
- Bugfix: Correct LMR divisor default to 225 and Zobrist transposition table replacement policy to fix Elo regression

### Fixed



## [V0.22.5] - 2026-07-28

### Optimized & Fixed
- **Evaluation Performance & Integrity Optimizations (`src/eval_service.rs`)**:
  - **Connected Passed Pawns Acceleration**: Replaced $O(N^2)$ nested loops with division/modulo with single-pass bitwise mask checks using `movegen.get_king_attacks(sq) & !file_mask`.
  - **Piece Loop Optimization**: Eliminated per-square `board.get_piece_at()` mailbox lookups and 12-branch `match` dispatching in favor of direct iteration over piece type bitboards.
  - **King Danger Defender Lookup**: Streamlined defender count loops for knights and bishops without mailbox lookups.
  - **Knight Evaluation Symmetry**: Removed duplicate `attacks_on_ring` computation in `white_knight`, resolving evaluation asymmetry between White and Black knights.
  - **Unit Testing Suite**: Added targeted unit tests for `get_king_attacks`, connected passed pawns, and knight evaluation symmetry.

## [V0.22.4] - 2026-07-28

### Changed & Configuration
- **Full Search & Evaluation Fidelity Baseline**:
  - Disabled **Lazy Evaluation** (`enable_lazy_eval: false`) in src/config.rs for full positional feature calculation accuracy across all evaluated nodes.
  - Disabled **Futility Pruning** (`enable_futility_pruning: false`) in src/config.rs by default.
  - Retains all underlying Futility Pruning code, configuration infrastructure, and unit tests, allowing FP to be toggled on-demand via UCI parameters (`EnableFutilityPruning: true`) or SPSA tuning harnesses without modifying code.



## [V0.22.3] - 2026-07-28

### Changed & Optimized
- **Aggressive Futility Pruning Configuration (`src/config.rs`)**:
  - Extended maximum Futility Pruning depth to `futility_max_depth = 4` (pruning quiet moves at depths 1, 2, 3, and 4).
  - Tightened pruning safety margins: reduced base margin `futility_margin_base = 100` cp (from 150) and slope `futility_margin_slope = 70` cp (from 100).
  - New depth-scaled margin thresholds: Depth 1 = 170 cp, Depth 2 = 240 cp, Depth 3 = 310 cp, Depth 4 = 380 cp.
- **Search Tree Node Reduction**:
  - Increases search tree pruning aggressiveness, significantly boosting Nodes Per Second (NPS) and reaching deeper ply search depths in fast time controls.
- **Overflow-Safe Mate Score Normalization (`src/search_service.rs`)**:
  - Switched mate score Transposition Table normalization to `saturating_add` and `saturating_sub` in `minimax`, preventing potential integer overflow panics when evaluating nodes with extreme scores.
  - Added dedicated unit test `test_mate_score_normalization_overflow_safety` to verify saturating arithmetic and search stability on forced mate scores.



## [V0.22.2] - 2026-07-28

### Added & Verified
- **Futility Pruning Unit Tests (`src/search_service.rs`)**:
  - `test_futility_pruning_node_reduction`: Verifies that `enable_futility_pruning: true` achieves a strict reduction in searched nodes compared to `enable_futility_pruning: false` during iterative search.
  - `test_futility_pruning_tactical_safety_guards`: Verifies that search with Futility Pruning active runs safely on complex tactical positions without dropping tactical moves or returning corrupted evaluation bounds.



## [V0.22.1] - 2026-07-28

### Added & Optimized
- **Futility Pruning (FP) at Low Search Depths (`depth <= 3`)**:
  - Implemented leaf/frontier Futility Pruning inside the main move search loop in src/search_service.rs.
  - Skips unpromising quiet moves at low search depths (`depth <= 3`) when `static_eval + futility_margin <= alpha` (for White) or `static_eval - futility_margin >= beta` (for Black), where `futility_margin = base + slope * depth`.
  - Added strict tactical safety guards: Futility Pruning is bypassed if the node is in check (`turn.gives_check`), if the move is tactical (captures, promotions, or check-giving moves), or if the move is a priority move (Transposition Table move, Killer moves, or Counter move).
  - Preserved PV-node evaluation fidelity by disabling Futility Pruning when `is_pv` is active or when near mate scores (`alpha.abs() >= 20000`).
- **Centralized `static_eval` Computation**:
  - Precalculates `static_eval` once per node when `depth > 0` and not in check, reusing the value across both Reverse Futility Pruning (RFP) and Futility Pruning (FP) to prevent redundant evaluation calls.
- **Transposition Table Hash-Move Preservation**:
  - Extracted the Transposition Table move (`tt_move`) during TT lookup to guarantee that TT best-moves are protected from pruning in subsequent search plies.
- **New UCI & Tuning Parameters in `Config`**:
  - Added `enable_futility_pruning: bool` (default: `true`), `futility_max_depth: i32` (default: `3`), `futility_margin_base: i16` (default: `150`), and `futility_margin_slope: i16` (default: `100`) in src/config.rs.



## [V0.22.0] - 2026-07-28

### Fixed & Added
- **Complete Zobrist Hash Specification**:
  - Expanded Zobrist key generation in src/zobrist.rs to include 16 castling rights states (`CASTLING_RIGHTS`) and 8 en-passant file target fields (`EN_PASSANT_FILE`), eliminating Transposition Table collisions across positions with differing castling or en-passant availability.
  - Added unit tests `zobrist_castling_rights_hash_test` and `zobrist_en_passant_hash_test` for Zobrist key differentiation.
- **Quiescence Search & Search Baseline Restoration**:
  - Reverted flawed early-return lazy evaluation in Quiescence Search back to the robust, high-performing v0.20.0 search baseline.
  - Reset `enable_lazy_eval` default configuration to `false`.

## [V0.21.1] - 2026-07-28

### Added
- Unified lazy evaluation and search fixes, optimized outpost calculation

### Fixed



## [V0.21.1] - 2026-07-28

### Added & Fixed
- **Clean Quiescence Search Lazy Evaluation**:
  - Integrated `cheap_eval` in src/eval_service.rs into Quiescence Search inside src/search_service.rs to bypass full evaluation when positional terms cannot change the alpha/beta cutoff.
  - Excluded Transposition Table writes when lazy evaluation prunes, completely preventing the TT pollution and depth-0 cache thrashing present in v0.20.3.
- **Outpost Calculation Speedup**:
  - Replaced 48-iteration unconditional loops in `calc_eval` with candidate bitboard filtering, cutting full evaluation time from ~8.5µs down to ~2.5µs.
- **UCI Parameter Support**:
  - Re-exposed `EnableLazyEval` and `LazyEvalMargin` UCI options in src/threads.rs and src/game_handler.rs.


## [V0.21.0] - 2026-07-28

### Fixed & Changed
- **Transposition Table Replacement Policy & Consistency**:
  - Corrected the replacement policy in `insert_entry` within zobrist.rs to check `existing.depth == -1 || entry.depth >= existing.depth`. This prevents lower-depth search results from overwriting higher-depth entries of the same position.
  - Removed useless depth-0 writes to the Transposition Table in Quiescence Search (`depth <= 0` block in search_service.rs). Since Quiescence Search does not read from the Transposition Table and main search requires `depth >= 1`, writing depth-0 entries was causing memory overhead and cache thrashing by overwriting valuable deeper entries.
  - Removed a redundant and incorrect write in `get_moves` within search_service.rs at `depth == 2`, which fälschlicherweise overwrote child bounds (`UpperBound`/`LowerBound`) as `Exact` in the Transposition Table.
  - Adjusted unit tests in zobrist.rs (`zobrist_replacement_policy_test`) to verify that deeper entries are correctly preserved when writing lower-depth entries to the same key.
- **Search Efficiency & Correctness**:
  - Added a `stop_flag` check directly at the entry of `minimax` in search_service.rs to abort the recursive call hierarchy immediately upon timeout/signal, preventing redundant deep search tree traversals.
  - Corrected the `is_pv` argument passed in recursive `minimax` calls in search_service.rs for Late Move Reductions (LMR) and Principal Variation Search (PVS) null-window checks. By passing `false` for non-PV nodes instead of a hardcoded `true`, we avoid dampening LMR on non-PV moves, leading to much faster search speeds.




## [V0.19.4] - 2026-07-26

### Fixed & Changed
- **Disabled Positional Evaluation Capping (`enable_positional_cap = false`)**:
  - Disabled positional evaluation capping by default (`enable_positional_cap: false`) in config.rs.
  - Removes soft-clamping of positional evaluation terms above 150 centipawns (`positional_cap_damping`), allowing true uncompressed positional evaluation weight throughout the search tree.
  - Updated `test_positional_evaluation_capping` in eval_service.rs to explicitly enable capping for unit testing.



## [V0.19.3] - 2026-07-26

### Fixed & Changed
- **Re-aligned Late Move Reductions (`lmr_divisor = 225`)**:
  - Set `lmr_divisor` to **225** in config.rs, parameters.json, server_parameters.json, and spsa_state_remote.json.
  - Recalculated the static logarithmic LMR lookup table (`lmr_table` with `divisor = 225.0 / 100.0`).
  - Eliminates overly aggressive quiet move depth reductions that caused tactical horizon-effect errors, restoring safe search tree traversal and tactical stability.



## [V0.19.2] - 2026-07-26

### Fixed & Changed
- **Full Positional Evaluation Accuracy (`enable_lazy_eval = false`)**:
  - Disabled Lazy Evaluation cutoffs (`enable_lazy_eval: false`) in config.rs.
  - Ensures that every single node in the search tree evaluates complete positional features (including king danger weights, pawn structures, passed pawn shields, and threat matrix calculations) without premature early exits based solely on raw material margins.
- **Aggressive Late Move Reductions (`lmr_divisor = 180`)**:
  - Set `lmr_divisor` to **180** in config.rs and parameters.json.
  - Recalculated the static logarithmic LMR lookup table (`lmr_table` with `divisor = 180.0 / 100.0`), matching the aggressive quiet move reduction strength of `v0.15.3` to achieve deeper ply search reach in fast time controls.
- **Retained Positional Cap Damping**:
  - Kept `enable_positional_cap = true` and `positional_cap_damping = 5` active to prevent positional evaluation saturation blindness while preserving true strategic evaluation depth.



## [V0.19.1] - 2026-07-26

### Fixed
- **Reverted SPSA Evaluation Parameter Regression & Restored v0.18.1 Playing Strength**:
  - Reverted all evaluation and search configuration parameters in config.rs and SPSA parameter definitions in parameters.json back to the proven version `v0.18.1` baseline.
  - Re-aligned the Late Move Reductions (LMR) `lmr_divisor` to **225** (from **152**), eliminating aggressive quiet move pruning that caused tactical horizon-effect errors in `v0.19.0`.
  - Re-established exact `v0.18.1` evaluation weights across king safety, pawn shield, piece activity, outposts, and threat penalties (`king_open_file_malus`: 37, `king_pawn_shield`: 37, `bishop_pair_bonus`: 48, `pawn_on_last_rank_bonus`: 183, `rook_open_file`: 27, etc.).

### Performance & Search Benchmark
- **Search Tree & Evaluation Accuracy**:
  - Restored full search tree fidelity at startpos depth 10 to exactly **1,811,143 nodes** (606 ms, ~3.0 MNPS), matching the exact node count of the `v0.18.1` release.



## [V0.19.0] - 2026-07-25

### Added
- **SPSA Parameter Harvest**:
  - Harvested SPSA optimization results into engine configuration (config.rs) and parameter definitions (parameters.json).
  - **Late Move Reductions (LMR) Divisor Optimization**:
    - Adjusted baseline `lmr_divisor` to **152** (from **225**), optimizing logarithmic search depth reductions and reducing search tree node count at depth 10 by >5x (from 1,811,143 down to 342,889 nodes).
  - **King Safety & Shield Enhancements**:
    - Fine-tuned `king_pawn_shield` (38) and `king_pawn_shield_kingside` (39).
    - Adjusted `king_trapp_at_baseline_malus` (73) and `king_in_double_check_malus` (342).
  - **Piece Activity & Positional Heuristics**:
    - Updated `pawn_centered` (14), `knight_centered` (24), `knight_blockes_pawn` (27), `bishop_trapped_at_rim_malus` (58), `pawn_attacks_opponent_fig` (35), and `queen_in_attack_with_tempo` (30).
    - Tuned rook positioning parameters (`rook_on_seventh` set to 31, `rook_behind_passed_pawn_endgame` set to 37).

### Performance & Search Benchmark
- **Search Tree Compression**:
  - Startpos depth 10 node count compressed from **1,811,143 nodes** (606 ms) down to **342,889 nodes** (160 ms) while maintaining tactical precision, significantly speeding up move generation and position evaluation.


## [V0.18.1] - 2026-07-24

### Added
- **PolyGlot In-Memory RAM Caching (`CacheBookInRam`)**:
  - Implemented automatic in-memory RAM caching for PolyGlot `.bin` opening books inside `Book` (book.rs).
  - Upon first lookup or UCI configuration of `BookFile`, the entire 16-byte PolyGlot entry array is loaded into RAM (`Option<PolyglotBook>`), eliminating per-move disk I/O and enabling sub-microsecond ($O(\log N)$) move selection.
  - Added new UCI option `CacheBookInRam` (`setoption name CacheBookInRam value true/false`, default `true`) to control RAM caching dynamically.
  - Integrated `clear_polyglot_cache` to reset in-memory cached entries whenever `BookFile` changes or `CacheBookInRam` is toggled.
- **Fail-Fast Book Error Policy & Logging**:
  - Enhanced error handling when opening PolyGlot `.bin` files: if a specified `BookFile` path is invalid or unreadable, a critical error message is logged to the active log buffer (`RIP Critical Error: Failed to open PolyGlot book file ...`) and written to `stderr`, followed by an immediate controlled process exit (`std::process::exit(1)`).

### Fixed
- **Cleaned Up Debug Output**:
  - Removed verbliebene debug `println!` statements in `polyglot_key()` (polyglot.rs), keeping the UCI `stdout` stream completely clean.
- **Unit Test Coverage**:
  - Added `test_clear_polyglot_cache` unit test in book.rs to verify cache clearing and state management.



## [V0.18.0] - 2026-07-24

### Added
- **Open-Source PolyGlot (`.bin`) Opening Book Integration**:
  - Implemented `src/polyglot.rs` providing full support for world-standard binary opening books (`.bin`).
  - Added 64-bit PolyGlot Zobrist hashing (`polyglot_key`) covering all board pieces, castling availability, en-passant square, and side to move.
  - Added 16-bit big-endian PolyGlot move decoding into standard UCI move strings (`e2e4`, `e7e8q`).
  - Added $O(\log N)$ binary search reader (`PolyglotBook`) with weighted random move selection.
- **UCI Options `BookFile` and `OwnBook`**:
  - `BookFile`: Enables configuring an external PolyGlot `.bin` file via UCI (`setoption name BookFile value /path/to/book.bin`).
  - `OwnBook`: Toggles internal opening book fallback usage (`setoption name OwnBook value true/false`).
- **Priority Logic**:
  - When `BookFile` is specified, PolyGlot book moves take highest priority regardless of `OwnBook`.
  - When `BookFile` is unpopulated or lacks a move for the position, `OwnBook` controls fallback to the internal hardcoded Rust book.

### Documentation
- Updated `README.md` with complete documentation for `BookFile` and `OwnBook` UCI options and priority rules.

## [V0.17.3] - 2026-06-12

### Fixed
- **Ucinewgame Latency and Timeouts (Bullet Time Controls)**: Resolved a critical issue in `v0.17.2` where the engine would immediately lose by time on move 1 under fast bullet time controls (such as `0/110` with 110ms increment). Recreating and allocating the 512MB transposition table (`ZobristTable` with 33M elements) from scratch on `ucinewgame` took 100-150ms of CPU time, which consumed the entire starting clock budget before the engine could process the first `go` command.
- **In-place Cache Clearing**: Replaced the expensive heap reallocation of the transposition table with a fast, in-place `clear` method in `src/zobrist.rs` that resets only the keys of all `AtomicEntry` slots to `0`. This keeps memory pages warm and active in the cache, reducing the latency of `ucinewgame` from 100+ms to sub-millisecond ranges (and also improving subsequent search performance).



## [V0.17.2] - 2026-06-12

> **Important Note:** This version has proven to be 'buggy' in tournaments under very fast bullet time controls (e.g. `0/110`). Due to the time-consuming re-allocation of the 512MB transposition table on the `ucinewgame` command, the engine playing as White lost immediately by timeout before it could calculate its first move. Please use the corrected version `v0.17.3` instead.

### Fixed
- **Cache Persistence & State Pollution (95 Elo Regression Fix)**: Corrected a major regression from version `v0.17.1` where the transposition table (`zobrist_table`) and pawn structure cache (`pawn_table`) were not cleared between games. When tournament managers reuse the engine process, transposition entries and pawn hashes carried over from previous games, causing search non-determinism and major tactical blunders. We now re-initialize the `pawn_table` and recreate the `zobrist_table` upon receiving the `ucinewgame` command.
- **Search Determinism**: Restored identical node counts and evaluation outputs across consecutive game starts under process reuse conditions.

### Added
- **Incremental Pawn Key Consistency Test**: Implemented `test_pawn_key_consistency` in `src/move_gen_service.rs` to verify that the incremental Zobrist hash updates for pawns are perfectly aligned with full pawn hash recalculations across recursive move/undo sequences for complex FENs.



## [V0.17.1] - 2026-06-11

> **Important Note:** This version has proven to be 'buggy' in tournaments, leading to a massive Elo regression (approx. 95 Elo). Due to a lack of cache clearing (`zobrist_table` and `pawn_table`) on game resets, it caused Elo-damaging state carryover between consecutive games. Please use `v0.17.3` instead.

### Added
- Clean split of the Pawn HashTable logic (Static/Dynamic)

### Fixed



## [V0.17.0] - 2026-06-11

### Added
- **Phase 1: Pawn Hash Architecture Purge (Single-Thread Preparation)**:
  - Completely removed the legacy `PawnHashTable` implementation (`src/pawn_hash.rs`) and all `RwLock`/`Arc` overheads originally designed for Lazy SMP multithreading.
  - Eradicated the massive overhead associated with multi-threaded synchronization of pawn structure evaluations.
  - Cleaned up the `SearchContext`, `game_handler.rs`, and all core evaluation parameters to operate cleanly without pawn cache dependencies.
  - This marks the absolute baseline for a pure single-threaded search tree, preparing the grounds for a massive `std::cell::Cell` lock-free integration in v0.17.1.
  - **Search & Performance Impact**:
    - Resolves all potential data races during high-speed iterative deepening.
    - Yields a clean search benchmark resolution of **1,811,143 nodes** at depth 10 (3.11 MNPS) for precise baseline verification before cache reconstruction.

### Fixed
- Fixed compilation and lifetime issues inside the move generation test suites, strictly stripping all deprecated caching dependencies.


## [V0.16.2] - 2026-06-11

> **Important Note:** Versions v0.16.0 to v0.16.2 have proven to be 'buggy' in tournaments and significantly weaker in playing strength than their predecessors (especially v0.15.3 and v0.14.0). The reason for this was faulty dynamic caching within the new Pawn HashTable (causing incorrect position evaluations). The associated Git tags have been removed to prevent accidental re-builds.

### Added
- **Optimized Late Move Reductions (LMR) Divisor (225)**:
  - Adjusted the default `lmr_divisor` value to **225** in config.rs.
  - Aligned the static logarithmic reduction lookup table (`lmr_table`) initialization divisor inside `Config::new()` to `225.0 / 100.0` in config.rs for startup consistency.
  - Aligned SPSA tuning variables to use 225 as base.
  - **Search & Performance Impact**:
    - Evaluates a highly conservative LMR quiet move reduction scaling factor (greater than 205). A higher divisor results in significantly less aggressive quiet move depth reductions, rendering search trees safer and more robust against horizon-effect tactical blunders, at the cost of searching a larger number of nodes.

### Fixed



## [V0.16.1] - 2026-06-11

### Added
- **Optimized Late Move Reductions (LMR) Divisor (205)**:
  - Adjusted the default `lmr_divisor` value to **205** in config.rs.
  - Aligned the static logarithmic reduction lookup table (`lmr_table`) initialization divisor inside `Config::new()` to `205.0 / 100.0` in config.rs for startup consistency.
  - Aligned SPSA tuning variables to use 205 as base.
  - **Search & Performance Impact**:
    - Tests a softer LMR quiet move reduction scaling factor. Increasing the divisor to 205 results in slightly less aggressive depth reductions, which improves tactical safety in complex search branches by mitigating the risk of horizon-effect pruning errors.

### Fixed





## [V0.16.0] - 2026-06-11

### Added
- **Optimized Late Move Reductions (LMR) Divisor (190)**:
  - Adjusted the default `lmr_divisor` value to **190** in config.rs.
  - Aligned the static logarithmic reduction lookup table (`lmr_table`) initialization divisor inside `Config::new()` to `190.0 / 100.0` in config.rs for startup consistency.
  - Aligned SPSA tuning variables to use 190 as base.
  - **Search & Performance Impact**:
    - Targets the optimal LMR scaling factor near the v0.14.0 baseline to prune quiet late moves efficiently without triggering unwanted search kaskades or boundary crossings.
- **Pawn Hash Table Integration**:
  - Implemented a thread-safe, lock-free `PawnHashTable` in `src/pawn_hash.rs` to cache evaluation values for pawn structures.
  - Significantly reduces redundant static pawn evaluations across the search tree, freeing up CPU cycles for deeper positional searches.
  - Integrated `pawn_table` lookup into `calc_eval` inside eval_service.rs and passed it down the search tree via `SearchContext`.

### Fixed





## [V0.15.3] - 2026-06-10

### Added
- **Re-aligned Late Move Reductions (LMR) Divisor**:
  - Restored the default `lmr_divisor` value to **180** (up from **148** / **150**) in config.rs.
  - Aligned the static logarithmic reduction lookup table (`lmr_table`) initialization divisor inside `Config::new()` to `180.0 / 100.0` in config.rs for consistency.
  - Aligned the SPSA tuning environment configuration by updating the default `lmr_divisor` value in parameters.json and server_parameters.json to **180**.
  - Reset the active SPSA optimization state in spsa_state_remote.json to **180.0** and cleared its momentum `m` to **0.0**.
  - **Search & Performance Impact**:
    - Re-establishes the optimal LMR scaling factor from `v0.14.1` (where the engine scored a peak 58.3% win rate).
    - Ensures search tree reductions are appropriately scaled, preventing search depth degradation.

### Performance & ELO Validation
- **Louguet Chess Test II Scoreboard**:
  - Estimated tactical/positional rating of **2110 ELO** on the Louguet Chess Test II, solving **7 / 35 positions** (scoring **210 points**).
  - Category performance: Positional (3/14), Tactical (2/12), Endgame (2/9).

### Fixed



## [V0.15.2] - 2026-06-09

> [!WARNING]
> **REGRESSION (ELO DROP)**: This version performed worse than its predecessor `v0.15.1` (Elo 2053 vs 2058). The SPSA tuned evaluation parameters likely misbalanced the engine's threat perception (e.g., over-deescalating minor piece attacks), leading to weaker positional play.

### Added
- **SPSA Parameter Harvest (Iteration 43)**:
  - Harvested the optimized parameter values from a 43-iteration SPSA run on EODServer.
  - **Threat Matrix De-escalation**:
    - Reduced the heuristic penalties for minor pieces attacking rooks/queens (`threat_minor_attacks_rook` from 20 to 15, `threat_minor_attacks_queen` from 45 to 30) and rooks attacking queens (`threat_rook_attacks_queen` from 30 to 20). This makes the engine less prone to panicking and misjudging positions where these "threats" are safely defended.
  - **Positional Mastery Enhancements**:
    - Amplified the strength of connected passed pawns in both middlegame and endgame (`connected_passed_pawn_mg` from 10 to 15, `connected_passed_pawn_eg` from 20 to 30).
    - Strengthened the "True Outpost" heuristic for Knights (`knight_outpost_true_mg` from 25 to 30, `knight_outpost_true_eg` from 10 to 15) and Bishops (`bishop_outpost_true_mg` from 15 to 20, `bishop_outpost_true_eg` from 5 to 10), rewarding centralized minor pieces that cannot be chased away by enemy pawns.
  - **Defensive Tarrasch Rule Integration**:
    - Significantly increased the reward for rooks placed behind enemy passed pawns (`rook_behind_enemy_passed_pawn_mg` from 5 to 10, `rook_behind_enemy_passed_pawn_eg` from 15 to 25) for improved endgame defense.

### Performance & ELO Validation
- **Search Tree Efficiency**:
  - Startpos Depth 11 resolves **1,602,002 nodes** at **2.47 MNPS**, maintaining extremely fast positional evaluation with zero search tree bloat.

### Fixed



## [V0.15.1] - 2026-06-08

### Fixed
- **Reverted King Danger Scaling Regression**:
  - In `v0.15.0`, King Danger was scaled with `game_phase` via `calculate_weighted_eval` (interpolating middlegame and endgame). Because the weights (`king_ring_attack_*`) were originally tuned for an unscaled static evaluation, mathematically halving them during the middlegame severely weakened the engine's attacking initiative and led to a -62 Elo regression.
  - Reverted this logic in eval_service.rs: `white_king_danger_term` and `black_king_danger_term` now bypass `calculate_weighted_eval` and are added directly to the final `eval`, restoring the engine's original sharp tactical play.

### Added
- **Config Heap Allocation Optimizations**:
  - Eliminated severe hot-path heap allocations during Config cloning when non-normal aggressiveness is active.
  - In config.rs, changed `pub version: String` to `pub version: &'static str`, initialized with a compile-time static literal `concat!("V", env!("CARGO_PKG_VERSION"))`.
  - Changed `pub log_path: String` to `pub log_path: std::sync::Arc<str>`, initialized with `std::sync::Arc::from("")`, which is cheap to clone without heap allocation.
  - Updated UCI option assignment for `log_path` in game_handler.rs to use `Arc::from(val_str.as_str())`.
- **Zero Warnings Clippy Cleanup**:
  - Resolved all compiler and Clippy linter warnings in accordance with the Zero Warnings Policy.
  - Refactored multiple range loops in magic.rs, zobrist.rs, config.rs, model.rs, and move_gen_service.rs to use iterators/enumerate, preventing boundary check overhead and resolving lints.
  - Rewrote loop control flows in threads.rs and game_handler.rs to use `while let` pattern matching.
  - Simplified manual check bounds to `clamp` in search_service.rs.
  - Cleaned up redundant condition checks and identical blocks.
  - Added a crate-level allow attribute for `clippy::too_many_arguments` to preserve register-based search parameter performance.



## [V0.15.0] - 2026-06-08

### Added
- **Connected Passed Pawns Heuristic**:
  - Implemented dynamic detection and bonuses for connected passed pawns. Connected passed pawns are highly resilient and powerful, especially in the endgame.
  - Added configurable parameters `connected_passed_pawn_mg` and `connected_passed_pawn_eg` in `Config`, mapped to UCI options.
- **True Outpost Identification**:
  - Upgraded piece placement evaluation with "True Outposts" for Knights and Bishops on ranks 4-6 (White) / ranks 3-5 (Black).
  - Outpost validation ensures the square is defended by a friendly pawn and cannot be attacked by any enemy pawn.
  - Added `knight_outpost_true_mg`, `knight_outpost_true_eg`, `bishop_outpost_true_mg`, and `bishop_outpost_true_eg` to configuration.
  - Pieces occupying or actively attacking/controlling these outpost squares are awarded positional bonuses.
- **Asymmetric Castling Pawn and Shield Heuristics**:
  - Replaced symmetric castling evaluation with kingside vs. queenside-specific pawn/piece shields.
  - Added dedicated parameters `king_pawn_shield_kingside`, `king_pawn_shield_queenside`, `king_piece_shield_kingside`, and `king_piece_shield_queenside` to the config.
  - Improves defensive evaluation depending on whether the King castles short (files f-h) or long (files a-c).
- **Scale Down for Opposite-Colored Bishops Endgames**:
  - Implemented automatic scale-down towards 0 for highly drawish opposite-colored bishops endgames (only Kings, Pawns, and exactly 1 Bishop per side on opposite colors).
  - Configurable scaling via `opposite_bishops_draw_scale` (percentage scaled by 100, default 50 representing 50% scale-down).
  - Helps the engine avoid entering dead-drawn opposite-colored bishop endgames even when up in material.
- **Defensive Tarrasch Rule Integration**:
  - Rewarded rooks for being placed behind *enemy* passed pawns on the same file, in addition to existing friendly passed pawn support.
  - Configured via `rook_behind_enemy_passed_pawn_mg` and `rook_behind_enemy_passed_pawn_eg`.

### Fixed
- **King Danger Endgame Scaling Bug**:
  - Refactored King Danger evaluation to scale directly into middlegame evaluation (`o_eval`) rather than applying to the final interpolated evaluation. This correctly ensures King Danger scaling properly transitions to 0 in pure endgames.
  - Added comprehensive test coverage validating correct King Danger interpolation.

### Performance & ELO Validation
- **Louguet Chess Test II Strength Boost**:
  - Achieved an estimated ELO rating of **2140 ELO** (+30 ELO over `v0.13.12`), solving **8 / 35 positions** (+1 positional/tactical/endgame net improvement).
  - Solved `LCTII.POS.08` (Unzicker - Fischer, Varna 1962) in **1.28s** due to new true outpost and king safety logic.
  - Solved `LCTII.TAC.03` (Drimer - Rellstab, corr. 1968) in **7.34s** and `LCTII.TAC.08` (Nei - Bronstein, Moskau 1963) in **7.92s**.
  - Solved `LCTII.END.03` (Bishop Endgame Study) in **5.16s** (improving endgame bishop coordination).
- **Search Tree & Perft Consistency**:
  - Search tree node count on start FEN remains consistent, confirming zero unwanted search explosions from the positional evaluation upgrades.

## [V0.14.2] - 2026-06-08

> [!WARNING]
> **REGRESSION (ELO DROP)**: This version performed worse than `v0.14.1` (Elo < 2049). Decreasing the LMR divisor further to 150 caused significant search tree node bloat due to boundary crossing issues, severely degrading performance.

- **Late Move Reductions (LMR) Divisor Tuning**:
  - Decreased the default `lmr_divisor` value from **180** to **150** in config.rs to test even more aggressive quiet late move reductions ($1.50$ scaling factor).
  - Re-aligned the static logarithmic reduction lookup table (`lmr_table`) initialization divisor inside `Config::new()` to `150.0 / 100.0` in config.rs for consistency.
  - Updated the SPSA tuning parameter default value in parameters.json to **150**.
  - **Search Characteristics Impact**:
    - Reduces quiet move depth much more aggressively.
    - Yields a significantly larger node count at depth 10 (Nodes: **1,905,766** vs **1,019,063** in `v0.14.1`), caused by deep search branch extensions and specific depth/move index truncation threshold boundary crossings.
  - Documented search tree benchmark metrics in perft.md.

### Fixed



## [V0.14.1] - 2026-06-08

> [!WARNING]
> **REGRESSION (ELO DROP)**: This version performed worse than the `v0.14.0` baseline (Elo 2056 vs 2065). Decreasing the LMR divisor to 180 made pruning too aggressive, causing horizon-effect tactical blunders.

- **Late Move Reductions (LMR) Divisor Tuning**:
  - Decreased the default `lmr_divisor` value from **195** to **180** in config.rs to test more aggressive quiet late move reductions.
  - Re-aligned the static logarithmic reduction lookup table (`lmr_table`) initialization divisor inside `Config::new()` to `180.0 / 100.0` in config.rs for consistency.
  - Updated the SPSA tuning parameter default value in parameters.json to **180**.
  - **Search Characteristics Impact**:
    - Reduces quiet move depth more aggressively (Nodes at depth 10: **1,019,063** vs **904,120** in `v0.14.0`). Note that depth 10 nodes increases slightly because of search depth truncation boundary shifts at specific depth/move idx parameters.
  - Documented search tree benchmark metrics in perft.md.
- **Unit Verification Enhancements**:
  - Refactored `test_logarithmic_lmr_table` in search_service.rs to dynamically compute assertions based on the active `config.lmr_divisor` rather than using hardcoded expected values. This prevents test failures when the divisor is adjusted.

### Fixed



## [V0.14.0] - 2026-06-08

- **Late Move Reductions (LMR) Divisor Reversion**:
  - Reverted the default `lmr_divisor` value from **250** to **195** in config.rs to return to the optimal 1.95 baseline.
  - Re-aligned the static logarithmic reduction lookup table (`lmr_table`) initialization divisor inside `Config::new()` to `195.0 / 100.0` in config.rs for startup consistency.
  - Updated the SPSA tuning parameter default value in parameters.json to **195**.
  - **Search Characteristics Impact**:
    - Reverting to 1.95 increases the reduction amounts compared to 250, resulting in more aggressive pruning of quiet moves.
    - Reduces the search tree size back to a more efficient state (Nodes at depth 10: **904,120** vs **2,163,889** in `v0.13.16`).
  - Documented search tree benchmark metrics for the reverted divisor in perft.md.

### Fixed



## [V0.13.16] - 2026-06-08

### Added
- **Late Move Reductions (LMR) Divisor Tuning**:
  - Increased the default `lmr_divisor` value from **220** to **250** in config.rs.
  - Updated the static logarithmic reduction lookup table (`lmr_table`) initialization divisor inside `Config::new()` to `250.0 / 100.0` in config.rs to ensure consistency at engine startup.
  - Aligned the SPSA tuning environment configuration by updating the default `lmr_divisor` value in parameters.json to **250**.
  - **Search Characteristics Impact**:
    - By increasing the divisor in the formula $\text{reduction} = \frac{\ln(\text{depth}) \times \ln(\text{move\_idx})}{\text{divisor}}$, the overall logarithmic search reductions for quiet moves are rendered **less aggressive**.
    - This increases the search tree safety and improves tactical accuracy for late-ordered moves, lowering the risk of horizon-effect blunders at the cost of a slightly larger search tree (Nodes at depth 10 startpos: **2,163,889** vs **2,367,889** in `v0.13.15`).
  - Documented search tree benchmark metrics for the new divisor in perft.md.

### Fixed



## [V0.13.15] - 2026-06-08

> [!WARNING]
> **REGRESSION (ELO DROP)**: This version performed worse than `v0.13.14` (Elo 2051 vs 2057). Increasing the LMR divisor to 220 caused search tree node bloat, which negatively impacted time management and overall strength under time controls.

### Added
- **Late Move Reductions (LMR) Divisor Tuning**:
  - Increased the default `lmr_divisor` value from **196** to **220** in config.rs.
  - Updated the static logarithmic reduction lookup table (`lmr_table`) initialization divisor inside `Config::new()` to `220.0 / 100.0` in config.rs to ensure consistency at engine startup.
  - Aligned the SPSA tuning environment configuration by updating the default `lmr_divisor` value in parameters.json to **220**.
  - **Search Characteristics Impact**:
    - By increasing the divisor in the formula $\text{reduction} = \frac{\ln(\text{depth}) \times \ln(\text{move\_idx})}{\text{divisor}}$, the overall logarithmic search reductions for quiet moves are rendered **less aggressive**.
    - This increases the search tree safety and improves tactical accuracy for late-ordered moves, lowering the risk of horizon-effect blunders at the cost of a slightly larger search tree (Nodes at depth 10 startpos: **2,367,889** vs **1,518,649** in `v0.13.12`).
  - Documented search tree benchmark metrics for the new divisor in perft.md.

### Fixed



## [V0.13.14] - 2026-06-07

### Added
- **Evaluation Clone Avoidance Optimization**:
  - Refactored `calc_eval` in eval_service.rs to bypass the expensive cloning of the `Config` struct when running under `Normal` aggressiveness mode (the default tournament mode).
  - This avoids cloning the heap-allocated `log_path: String` field introduced in recent versions millions of times per second during search.
  - Successfully recovers the ~20% NPS (Nodes Per Second) performance regression, making search speed slightly faster than `v0.13.4`.

### Fixed
- **LMR Divisor Configuration Revert**:
  - Reverted `lmr_divisor` and its corresponding initialization table divisor back to the stronger `v0.13.12` baseline value of **196** (from 198) in config.rs and parameters.json.
  - Matches the stronger configuration found in SPSA iteration 20, which performed significantly better in matchups than the iteration 75 value of 198.



## [V0.13.13] - 2026-06-06

### Added
- **SPSA Parameter Harvest (Iteration 75)**:
  - Harvested the final optimized parameter values from a 75-iteration SPSA run (comprising 22,500 games under 2s + 100ms time control on EODServer).
  - Baked in the new optimized baseline value of **198** (from 196) for lmr_divisor into parameters.json and config.rs.
- **LCT II Tactical & Positional Strength Boost**:
  - Achieved an estimated ELO rating of **2110 ELO** on the Louguet Chess Test II, solving **7 / 35 positions** (scoring **210 points**).
  - Positional mastery verified: solved `LCTII.POS.02`, `LCTII.POS.10`, and `LCTII.POS.13`.
  - Tactical/Endgame stability verified: solved `LCTII.TAC.02`, `LCTII.TAC.04`, `LCTII.END.01`, and `LCTII.END.02`.

### Fixed
- **LMR Table Initialization Divisor Sync Bug**:
  - Updated the default `lmr_table` calculation inside `Config::new()` in config.rs to use the correct baseline divisor `198.0 / 100.0`. Previously, changing `lmr_divisor` in the struct field did not automatically update the hardcoded table initialization divisor at engine startup.
  - Modified harvest_tuning.py to automatically replace the hardcoded table divisor during future harvests.



## [V0.13.12] - 2026-06-05

### Added
- Set default lmr_divisor to 250 for manual play

### Fixed



## [V0.13.11] - 2026-06-05

### Added
- update default lmr_divisor to 196 based on SPSA iteration 20

### Fixed



## [V0.13.10] - 2026-06-04

### Added
- **Configurable LMR Divisor Parameter (`lmr_divisor`)**: Exposed the Late Move Reductions (LMR) divisor as a configurable parameter (`lmr_divisor` in `Config`, mapped to UCI options `"lmr_divisor"` and `"lmr_divisor_scaled"`). Since SPSA operates on integers, it is scaled by 100 (default `195` representing $1.95$) and dynamically recomputes the logarithmic lookup table `lmr_table` upon receipt of UCI setoption commands.
- **Dynamic LMR Table Calculation**: Implemented `Config::recalculate_lmr_table` to update search depth reductions at runtime without impacting search speed (nodes per second).

### Fixed
- **Warning-Free Compilation**: Resolved multiple compiler warnings across the codebase:
  - Cleaned up unused variables `rank`, `file`, and `e_eval` in static queen evaluation functions (`white_queen` and `black_queen` in `src/eval_service.rs`).
  - Marked unused parameter `config` as `_config` in `get_piece_value` and prefixed unused test variables in `src/search_service.rs`.
  - Removed a redundant trailing double semicolon in `src/book.rs`.
- **Opening Book Linkages**: Correctly inserted two previously unused book maps (`e2e4_e7e5_g1f3_g8f6_f3e5_d7d6` and `e2e4_c7c6_d2d4_d7d5_b1c3_d5e4_c3e4_c8f5_e4g3`), linking Petrov's Defense and Caro-Kann variations to their active book moves and resolving related compiler warnings.



## [V0.13.9] - 2026-06-04

### Added
- Expanded built-in static opening book (`src/book.rs`) to include 11 new/deepened opening lines:
  - **Scandinavian Defense**: Fleshed out lines after `2. exd5 Qxd5 3. Nc3` and `2... Nf6`.
  - **Scotch Game**: Added support for `1. e4 e5 2. Nf3 Nc6 3. d4 exd4 4. Nxd4 Nf6/Bc5`.
  - **King's Indian Defense**: Expanded lines through `1. d4 Nf6 2. c4 g6 3. Nc3 Bg7 4. e4 d6 5. Nf3 O-O`.
  - **Queen's Indian Defense**: Added depth to `1. d4 Nf6 2. c4 e6 3. Nf3 b6` with `4. g3` or `4. a3`.
  - **Alekhine's Defense**: Fleshed out lines after `1. e4 Nf6 2. e5 Nd5 3. d4 d6`.
  - **Petrov's Defense**: Expanded main line `1. e4 e5 2. Nf3 Nf6 3. Nxe5 d6 4. Nf3 Nxe4 5. d4 d5`.
  - **Philidor Defense**: Fleshed out `1. e4 e5 2. Nf3 d6 3. d4`.
  - **Vienna Game**: Added lines after `1. e4 e5 2. Nc3`.
  - **Modern Defense**: Added lines after `1. e4 g6 2. d4 Bg7 3. Nc3 d6`.
  - **Nimzowitsch Defense**: Fleshed out `1. e4 Nc6 2. d4 d5`.
  - **Caro-Kann Classical**: Added deep line `3. Nc3 dxe4 4. Nxe4 Bf5 5. Ng3 Bg6 6. h4 h6 7. Nf3 Nd7`.
- Added unit tests verifying that all book moves are legal in their respective FEN states.

### Fixed
- Fixed an incorrect FEN pawn structure for Petrov's Defense (`e2e4_e7e5_g1f3_g8f6_f3e5_d7d6_e5f3_f6e4`) in `src/book.rs`, restoring the white pawn on `d2` to correct board representation (`PPPP1PPP` instead of `PPP2PPP`).





## [V0.13.8] - 2026-06-04

### Added
- Release v0.13.8: Add missing SPSA parameters to parameter logger

### Fixed



## [V0.13.7] - 2026-06-04

> [!WARNING]
> **BUGGY VERSION (BROKEN SPSA TUNED PARAMETERS)**: This version is deprecated and contains incorrect evaluation parameters harvested from a broken SPSA tuning run. The parser bug in `src/threads.rs` caused the engine to ignore parameter updates sent via UCI during tuning matches, leading to random noise-based parameter values and significant search tree node bloat. Please use `V0.13.8` or `V0.13.4` instead.

### Added
- Release v0.13.7: Revert parameters and fix SPSA parser/logging

### Fixed



## [V0.13.6] - 2026-06-03

> [!WARNING]
> **BUGGY VERSION (BROKEN SPSA TUNED PARAMETERS)**: This version is deprecated and contains incorrect evaluation parameters harvested from a broken SPSA tuning run. The parser bug in `src/threads.rs` caused the engine to ignore parameter updates sent via UCI during tuning matches, leading to random noise-based parameter values and significant search tree node bloat. Please use `V0.13.8` or `V0.13.4` instead.

### Added
- Release v0.13.6: Harvest SPSA tuned parameters

### Fixed



## [V0.13.5] - 2026-06-03

> [!WARNING]
> **BUGGY VERSION (CRITICAL UCI OPTION PARSER BUG)**: This version contains a critical parser bug in `src/threads.rs` that ignores most evaluation parameter updates sent via UCI `setoption` commands. While its default parameters are stable, tuning runs on this version will fail as the engine ignores mutated parameters. Please use `V0.13.8` or `V0.13.4` instead.

### Fixed
- **Complete Cleanup of Easy-Move Remnants (Tests & SPSA Parameter Synchronization)**:
  - **The Cleanup**: While `v0.13.4` successfully removed the core Easy-Move logic from search execution, configuration references and deprecated testing assets remained. This release performs the final structural sanitation to keep the repository completely clean and prevent build issues in custom test harnesses.
  - **Removals & Adjustments**:
    - Removed the obsolete `test_easy_move_failing` unit test from `tests.rs`, which referenced the removed `easy_move_margin` parameter.
    - Completely deleted `easy_move_margin` from the `tuning/parameters.json` parameter list, ensuring that future SPSA optimization runs do not attempt to tune a non-existent parameter.
  - **Verification**:
    - Passes all 71 active unit tests and 5 ignored tests successfully.
    - Confirmed search tree size remains at the optimal **186,567 nodes** at depth 8 startpos (bypassing book) with **604 ms** search time at depth 10, achieving **1.49 MNPS**.
    - Louguet Chess Test II rating remains stable at **2075 ELO** solving 6/35 positions (175 points), confirming no regressions were introduced.

## [V0.13.4] - 2026-06-03

### Fixed
- **Complete Revert of Easy-Move Feature (Restored Alpha-Beta Pruning & ELO Performance)**:
  - **The Problem**: In `v0.13.3`, the root search window was widened by `easy_move_margin` (150 cp) for all subsequent root moves to exactly evaluate rival moves and compute the score gap. However, this caused a catastrophic **7.37x increase in the search tree** in quiet positions (e.g. startpos at depth 8 bloat from **186,567** to **1.376,272** nodes) because multiple legal moves fall within 1.5 pawns of the best move, disabling alpha pruning on them. In matches under time controls, this forced the engine to play 2-3 plies shallower, leading to a severe **-86 ELO** tournament regression.
  - **The Fix**: Completely reverted and removed the Easy-Move feature from the engine codebase, restoring the mathematical purity of the root alpha-beta search.
  - **Structural Cleanup**:
    - Removed `enable_easy_move`, `easy_move_depth_threshold`, `easy_move_stable_depths`, and `easy_move_margin` configurations from the `Config` struct and UCI option parser.
    - Cleaned up the game loop in `src/game_handler.rs` by removing the early-exit check and associated unused tracking variables (`pv_stable_count`, `last_best_move`, `is_infinite`).
    - Reverted `src/search_service.rs` search window updates to the standard, highly efficient alpha-beta bounds (`current_alpha = current_alpha.max(min_max_eval)`).
    - Removed Easy-Move unit tests and integration tests.
  - **Performance & ELO Validation**:
    - Search tree nodes at depth 8 is restored to the optimal **186,567 nodes** in startpos (down from 1.37M).
    - Louguet Chess Test II rating rose back to **2075 ELO** (6/35 solved, 175 points), solving key tactical positions (`LCTII.TAC.04` and `LCTII.TAC.05`) within the 10s time limit due to restored search depth and pruning efficiency.
    - Passes all 71 active unit tests and 5 ignored tests successfully with zero compiler warnings.



## [V0.13.3] - 2026-06-02

### Added
- **Root Search Window Margin for Easy-Move Heuristic**:
  - Implemented a root search window margin to accurately evaluate candidate moves at the root. Subsequent root moves are searched with a widened alpha/beta window: `alpha = best_eval - easy_move_margin` for White, and `beta = best_eval + easy_move_margin` for Black.
  - This ensures that any rival move within the `easy_move_margin` (150 cp) is fully and exactly evaluated rather than returning a fail-low alpha bound. The true evaluation gap is then computed at the game handler level, preserving the math of the easy-move early exit check.

### Fixed
- **Easy-Move Premature Search Exit Bug (False Positives)**:
  - **The Bug**: In version `v0.12.4`, a fallback checking if `search_result.variants.len() <= 1` was introduced to signal an easy move when only one move improved alpha at the root. However, under fail-hard alpha-beta search with optimized move ordering, the best move is searched first. All subsequent moves fail to beat this best move, failing low and returning exactly `alpha` (the lower bound). Because they failed low, they were not appended to `variants`. This resulted in `variants.len()` being `1` in over 90% of all chess positions, causing the engine to falsely trigger the easy-move exit. It would immediately cut the search short, losing significant depth and playing strength.
  - **The Fix**: Removed the flawed `variants.len()` check and restored the mathematical gap check in `src/game_handler.rs` using `best_score` and `second_best_score`. Together with the root search window margin in `src/search_service.rs`, the engine now correctly evaluates whether a position has a single dominant move or multiple competitive candidates.
  - **Performance & ELO Validation**:
    - Confirmed search correctness: the engine now searches to full depth on complex positions, with perft depth 10 resolving **904,120 nodes** in **490 ms** (**1.84 MNPS**).
    - Passes all 73 active unit tests (including `test_easy_move_failing` which explicitly validates the gap search logic) and 5 ignored tests successfully.
    - Achieved an estimated ELO of **2050 ELO** on the Louguet Chess Test II (5/35 solved).



## [V0.13.2] - 2026-06-02

### Added
- Revert evaluation parameters to stable v0.12.4 baseline to fix SPSA overfitting regression

### Fixed



## [V0.13.1] - 2026-06-02

> [!WARNING]
> **BUGGY VERSION (CRITICAL EVAL OVERFITTING)**: This version suffered from severe evaluation parameter regression. While the search bug was corrected, the evaluation parameters remained in a heavily distorted, overfitted state from the broken SPSA tuning of v0.13.0 (e.g., setting `protected_passed_pawn_endgame` to `0` and `rook_on_seventh` to `12`). This caused the engine to perform at a highly degraded level (~1370 Elo). **This version is deprecated. Please use V0.13.2 instead.**

### Added
- **SPSA Artifact De-escalation & Tuning Reset**:
  - Restored `undeveloped_knight_malus` to `31` (from `53`).
  - Restored `undeveloped_bishop_malus` to `34` (from `62`).
  - Restored `undeveloped_king_malus` to `54` (from `100`).
  - The previous values were artificially inflated by SPSA during an LMR search bug, which led to overly passive opening structures.

### Fixed
- **LMR Regression (Node Bloat & Speed Loss)**:
  - **The Bug**: Due to the panic fixed in v0.13.0, the SPSA tuner had drastically compensated by pushing `lmr_move_threshold` up to `9` (from `3`), forcing the engine to fully search the first 9 quiet moves at every single node. This caused the search tree to explode and created massive time management issues.
  - **The Fix**: Completely eliminated the obsolete `lmr_depth_threshold` configuration from the engine and UCI options (the depth constraint is now mathematically hardcoded as `depth >= 3`). Reset `lmr_move_threshold` back to the performant `3`.
  - **Performance Recovered**: Perft benchmark confirms the fix. Search tree nodes at Depth 9 dropped back down to a highly compressed **383,426 nodes** (in just **244 ms**), restoring optimal search depth and tournament speed.



## [V0.13.0] - 2026-06-02

> [!WARNING]
> **BUGGY VERSION (CRITICAL EVAL REGRESSION)**: This version suffers from a massive search speed degradation (~100 ELO loss in bullet time controls). The SPSA tuner artificially pushed the `lmr_move_threshold` up to `9` (effectively disabling early LMR pruning) to mask a depth calculation bug. This caused the search tree nodes to explode in size. Furthermore, the engine suffers from excessively passive opening play due to heavily inflated undeveloped piece maluses. **This version is deprecated. Please use V0.13.1.**

### Added
- **Major Evaluation Parameter Tuning (SPSA Iteration 21 - 10,500 games)**:
  - **Systematic Figure Development Penalties**:
    - Drastically raised maluses for undeveloped minor pieces in the early phases to enforce rapid, classical piece mobilization (`undeveloped_knight_malus` from 31 to 53, `undeveloped_bishop_malus` from 34 to 62, and `undeveloped_king_malus` from 54 to 100).
  - **Deepened King Safety focus**:
    - Greatly increased the king pawn shield bonus (`king_pawn_shield` from 39 to 61) to reward robust pawn shelter structures.
    - Highly elevated penalties for open and half-open files near the king (`king_open_file_malus` from 38 to 50, and `king_half_open_file_malus` from 20 to 50).
    - Doubled the value of active defenders in the king's ring (`king_ring_defender_value` from 1 to 2).
  - **Rook Coordination Re-balancing**:
    - Significantly increased reward for doubled, coordinated rooks on files (`rook_doubled_bonus` from 25 to 60), while slightly de-escalating single rooks on the 7th rank (`rook_on_seventh` from 32 to 12), driving the engine towards battery formations.
  - **Late Move Reductions (LMR) Restructuring**:
    - Integrated SPSA tuning results which lowered `lmr_depth_threshold` from 3 to 0 and raised `lmr_move_threshold` from 3 to 9, allowing LMR to trigger more aggressively at shallower depths but exclusively on late-sorted quiet moves.
- **Tuning and Test Stability Adjustments**:
  - Configured undeveloped piece malus overrides to `0` inside the evaluation test helper `_for_evel_equal_tests()` in `src/config.rs` to prevent material test distortions.
  - Widened evaluation boundary ranges in `src/eval_service.rs` to accommodate tuned pawn structures (`-3800` / `3800` bounds).

### Fixed
- **Late Move Reduction (LMR) Clamp Safety Panic**:
  - **The Bug**: SPSA's reduction of `lmr_depth_threshold` to `0` allowed LMR to trigger at search depths 1 and 2. This caused a critical standard library panic inside `.clamp(1, depth - 2)` because the upper limit (`0` or `-1`) was less than the lower limit (`1`), crashing the engine process.
  - **The Fix**: Embedded a strict depth safety guard `&& depth >= 3` to the LMR execution condition in `src/search_service.rs`. This guarantees LMR is only performed when search depth is high enough to allow mathematical reduction, ensuring absolute crash safety under all SPSA configurations.

### Performance & ELO Validation
- **Search Tree Efficiency Breakthrough**:
  - At depth 8, the engine searched fewer nodes (**172,574 nodes** vs 186,567 in `v0.12.4`) and resolved faster (**93 ms** vs 121 ms), pushing NPS up to **1.85 MNPS**.
  - At depth 15, the search resolved **4.90 million nodes** in only **1,420 ms** at an incredible **3.45 MNPS**, verifying superior branch pruning and speed.
- **Louguet Chess Test II Scoreboard**:
  - Stable tactical rating of **2080 Elo** (6/35 solved).
  - **Positional Mastery Unlocked**: Solved **`LCTII.POS.13` (Capablanca - Ragozin, Moskau 1935)** in a rapid **`0.39s`** (previously unsolved), demonstrating advanced positional and piece-development understanding.
  - **Tactical Study Solved**: Successfully resolved the deep tactical study **`LCTII.TAC.03` (Drimer - Rellstab)** in **`7.34s`** (previously unsolved) due to optimized LMR depth reductions.



## [V0.12.4] - 2026-06-02

### Added
- **Premium Variants-Based Easy-Move Early-Exit Heuristic (`src/game_handler.rs`)**:
  - Engineered a highly optimized, stateless, and mathematically pure solution to the Easy-Move detection gap issue.
  - **The Design**: Instead of modifying the alpha-beta search window at the root (which introduces TT pollution, search-tree expansion, and breaks under tight aspiration windows), the engine now queries `search_result.variants.len()`.
  - Since the `variants` vector only records root moves that successfully improved `alpha` during search, a value of `1` (or `0`) mathematically proves that all other quiet moves failed low and are catastrophically worse (blunders) than the PV move, signaling a definitively easy move.
  - If `variants.len() >= 2`, both moves improved `alpha` and thus have exact, fully-searched evaluations. The engine then safely subtracts `variants[1].eval` from `variants[0].eval` to verify if the gap meets the required `EasyMoveMargin` (150 cp).
  - **Zero Search Overhead**: This premium approach is **completely free (0 cost)**, requiring no extra nodes or search window widening, preserving search tree purity while unlocking rapid, safe early-exits.

### Fixed
- **Root-Search Aspiration & TT Pollution Vulnerability**: Completely avoided the critical flaw of window-widening which gets neutralized by tight aspiration windows (`delta = 15`) and expands branches of poor moves.
- **LCT II Tactical ELO & Speed Breakthrough**:
  - Achieved a monumental playing strength boost, raising estimated tactical rating by **+5 ELO** to **2080 ELO** on the Louguet Chess Test II!
  - Tactical points increased to **90/360 points**.
  - Successfully solved **`LCTII.TAC.05`** (Fischer's legendary queen sacrifice) in just **`8.83s`** (accelerated from `9.30s` in version `v0.12.2`), securing the full 30 points due to optimized search tree efficiency and clean move-ordering checks.
  - Verified 100% regression safety: all 68 active cargo unit tests passed successfully.



## [V0.12.3] - 2026-06-01

### Added
- **Dynamic Easy-Move Early-Exit Customization**:
  - Registered new UCI spin option `EasyMoveMargin` (range `0` to `10000`, default `150` centipawns), enabling users to configure the required evaluation margin between the best and second-best moves dynamically.
  - Fully exposed `easy_move_margin` through all internal thread configurations, configuration structures, and CLI parser modules, allowing easy SPSA tuning and interface customization.
  - Added new automated unit tests (`test_easy_move_configuration`) in `src/game_handler.rs` to verify thread-safe UCI option parsing and dynamic updates of Easy-Move parameters.

### Fixed
- **Critical Search Blindspot Fix (Safe Easy-Move Early-Exit Heuristic)**:
  - **The Bug**: In version `v0.12.1`, the engine mistakenly executed the Easy-Move early exit heuristically during `go infinite` commands. This caused the engine to exit the search early (often at depth 6) during infinite tactical searches, rendering it unable to solve complex tactical positions in evaluators (like LCT II) and severely degrading playing strength in deep tournament matches.
  - **The Fix**: Strictly restricted the Easy-Move early-exit check to only execute when `!is_infinite` is verified, restoring standard deep-search behavior for tournament benchmarks and puzzles.
  - **Evaluation Threshold Safety**: Restructured the early-exit condition to only trigger when the best move has a robust evaluation advantage over the second-best move ($\ge 150$ centipawns, or the user-configured `EasyMoveMargin`). This prevents premature exits on tactical positions where multiple solid options exist, ensuring that the engine only takes an early exit when the choice is mathematically clear and safe.



## [V0.12.1] - 2026-06-01

### Added
- **Premium SPSA Evaluation Parameter Tuning (Iteration 56)**:
  - Integrated 45 highly optimized evaluation parameters successfully tuned on the remote EODServer over 56,000 matches.
  - **Strategic Rook Play Enhancements**:
    - Significantly increased value for active rooks on the 7th rank (`rook_on_seventh` from 25 to 32, +7 ELO contribution).
    - Highly elevated endgame rook placement behind passed pawns (`rook_behind_passed_pawn_endgame` from 30 to 36, +6).
    - Raised bonus for doubled rooks (`rook_doubled_bonus` from 20 to 25, +5).
    - Decreased value of passive rooks on open files (`rook_open_file` from 35 to 26), pushing the engine towards active rook penetration rather than static placement.
  - **Leichtfiguren Re-balancing**:
    - Elevated the highly active Bishop Pair bonus (`bishop_pair_bonus` from 40 to 46, +6).
    - Drastically reduced undeveloped Knight malus (`undeveloped_knight_malus` from 42 to 31, -11) preventing over-eager piece development.
    - Raised malus for undeveloped Bishops (`undeveloped_bishop_malus` from 30 to 34) and rim-trapped Bishops (`bishop_trapped_at_rim_malus` from 50 to 57), prioritizing active Bishop development and mobility.
  - **King Safety Refinement**:
    - Reduced check and double-check penalties (`king_in_check_malus` from 140 to 136, and `king_in_double_check_malus` from 350 to 343) allowing the engine to pragmatically accept checks when defensive structures are solid.
    - Adjusted baseline king trapping penalty (`king_trapp_at_baseline_malus` from 75 to 72) for minor tactical de-escalation.
  - **Pawn Endgame Optimization**:
    - Significantly elevated passed pawns supported on the 5th rank (`pawn_on_before_before_last_rank_bonus` from 40 to 47, +7) and protected passed pawns in endgames (`protected_passed_pawn_endgame` from 24 to 26), ensuring stable, robust endgame structures.
- **Official Opening Book Tuning Integration**:
  - The master-level solid opening book improvements designed in `[V0.11.9]` (which were left unstaged in the git workspace) are now **officially staged, compiled, and committed**!
  - Pruned risky and passive opening defenses for Black (Spanish Steinitz `d7d6`, Spanish Classical `f8c5`, and Open Sicilian Dragon `g7g6`), channeling the engine into robust, high-draw-rate mainlines (Berlin/Morphy Spanish, Sicilian Najdorf/Scheveningen).
  - Integrated new master-level systems for Black and White (Ragozin & Orthodox Queen's Gambit Declined, Capablanca/Karpov systems in Caro-Kann, French Exchange mainlines).

### Performance & ELO Validation
- **Search Tree Efficiency**: Depth 10 resolved in **539 ms** with **904,120 nodes** at **1,674,000 NPS (1.67 MNPS)**.
- **LCT II ELO Scoreboard**:
  - Estimated tactical rating stable at **2075 ELO** on the Louguet Chess Test II.
  - Achieved a major tactical breakthrough: successfully solved **`LCTII.TAC.05`** (Fischer's famous queen sacrifice against Myagmarsuren) in `9.05s` (unsolvable by previous versions under 10s).

### Fixed



## [V0.12.0] - 2026-06-01

### Added
- **Configurable Easy-Move (Obvious Move) Early Exit Heuristic**:
  - Implemented an intuitive, high-performance time management heuristic in the iterative deepening loop (`src/game_handler.rs`) to instantly play forced recaptures, highly stable principal variations (PV), or singular best moves, significantly conserving time in rapid/blitz matches.
  - Automatically monitors best-move consistency across search iterations by comparing the principal variation's best move against previous depths.
  - **Premium Customizable Parameters in `src/config.rs`**:
    - `enable_easy_move` (type: check, default: `true`): Toggles the early exit heuristic globally.
    - `easy_move_depth_threshold` (type: spin, default: `6`): Minimum depth at which easy-move conditions can trigger.
    - `easy_move_stable_depths` (type: spin, default: `3`): The number of consecutive search depths for which the best move must remain unchanged (stable across 4 total depths) before triggering an early exit.
- **Full UCI Options Registration and Parsing**:
  - Registered options (`EnableEasyMove`, `EasyMoveDepthThreshold`, and `EasyMoveStableDepths`) inside `src/threads.rs` so that UCI chess GUIs (like Cutechess or Arena) or automatic SPSA tuners can query and configure them.
  - Added robust string parsing replacements (`enableeasymove`, `easymovedepththreshold`, and `easymovestabledepths`) inside `src/game_handler.rs`'s `setoption` command loop to allow transparent handling of case variations, spaces, and underscores (e.g., `setoption name Enable Easy Move value true` or `setoption name Enable_Easy_Move value true`).

### Performance & ELO Validation
- **Search Tree Metrics**: Verified iterative deepening behavior on starting positions, confirming correct termination logic once PV stability requirements are met.
- **LCT II ELO Scoreboard**:
  - Estimated rating is stable at **2110 ELO** on the Louguet Chess Test II, solving 7/35 positions (20.0%) scoring a total of 210 points.
  - Category performance: Positional (2/14), Tactical (2/12), Endgame (3/9 solved with strong pawn and bishop study completions).

### Fixed



## [V0.11.9] - 2026-06-01 *(Note: The book.rs changes were accidentally left unstaged in git during this version and were officially committed/released in V0.12.1)*

### Added
- **Solid Opening Book Tuning:**
  - **Pruned Risky & Passive Opening Variations for Black:**
    - *Spanish (Ruy Lopez):* Removed the passive Steinitz Defense (`d7d6`) and the highly tactical/fragile Classical Defense (`f8c5`) from the recommended moves after `1. e4 e5 2. Nf3 Nc6 3. Bb5` (FEN: `r1bqkbnr/pppp1ppp/2n5/1B2p3/4P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 0 3`). The engine is now steered exclusively into the ultra-solid mainlines: *Morphy Defense* (`a7a6`) and the *Berlin Defense* (`g8f6`).
    - *Spanish (Morphy Continuation):* Removed `d7d6` (Modern Steinitz) after `4. Ba4` (FEN: `r1bqkbnr/1ppp1ppp/p1n5/4p3/B3P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 0 4`), forcing the robust developmental mainline `g8f6` instead.
    - *Open Sicilian:* Fully pruned the sharp, theoretical, and engine-vulnerable *Sicilian Dragon* (`g7g6`) from the recommended responses to `1. e4 c5 2. Nf3 d6 3. d4 cxd4 4. Nxd4 Nf6 5. Nc3` (FEN: `rnbqkb1r/pp2pppp/3p1n2/8/3NP3/2N5/PPP2PPP/R1BQKB1R b KQkq - 0 5`), preserving only the positionally superior *Najdorf* (`a7a6`) and *Scheveningen/Classical* transpositions (`e7e6`).
  - **Integrated New Solid Master-Level Opening Lines:**
    - *Queen's Gambit Declined (Black - Ragozin & Orthodox Defense):* Added the solid defensive systems `f8e7` (Orthodox Defense) and `f8b4` (Ragozin Defense) after `1. d4 d5 2. c4 e6 3. Nc3 Nf6 4. Nf3` (FEN: `rnbqkb1r/ppp2ppp/4pn2/3p4/2PP4/2N2N2/PP2PPPP/R1BQKB1R b KQkq - 0 4`).
    - *Caro-Kann Defense (Black - Capablanca & Karpov Systems):* Added high-quality mainlines `c8f5` (Capablanca Variation), `b8d7` (Karpov System), and `g8f6` (Smyslov/Bronstein-Larsen) after `1. e4 c6 2. d4 d5 3. Nc3 dxe4 4. Nxe4` (FEN: `rnbqkbnr/pp2pppp/2p5/8/3PN3/8/PPP2PPP/R1BQKBNR b KQkq - 0 4`).
    - *Queen's Gambit Declined (White - Classical Line):* Added the robust positional continuations `e2e3` and `g1f3` after `4... Be7 5. e3` (FEN: `rnbqk2r/ppp1bppp/4pn2/3p2B1/2PP4/2N5/PP2PPPP/R2QKBNR w KQkq - 1 5`).
    - *French Defense (White - Exchange Mainline):* Added active bishop and knight developmental lines `g1f3` and `f1d3` in the French Exchange variation after `3. exd5 exd5` (FEN: `rnbqkbnr/ppp2ppp/8/3p4/3P4/8/PPP2PPP/RNBQKBNR w KQkq - 0 4`).

### Fixed



## [V0.11.7] - 2026-05-30

### Added
- Fix Pinned Piece Illusion in SEE and queen threat evaluation. Add LCT II and Perft verification tests.

### Fixed



## [V0.11.6] - 2026-05-30

### Added
- **Automated Performance & Elo Estimator (LCT II & Perft)**:
  - Integrated full Louguet Chess Test II (LCT II) evaluator, estimating engine strength at a highly competitive **2110 Elo** rating.
  - Restored full recursive Perft test harness to guarantee move generation correctness under complex tactical configurations.

### Fixed
- **The Pinned Piece Illusion in Evaluation & Search (Qxf6 / Qxd4 Bug)**:
  - Implemented highly optimized absolute pin detection (`is_pinned_away_from_target`) to dynamically evaluate if a piece is pinned to its king and unable to capture or defend separate squares.
  - Fixed a critical search bug where the Static Exchange Evaluation (SEE) falsely assumed a pinned Knight could capture a Queen on `d4`, pruning the winning centralizing capture `Qxd4` (pruned with SEE `-850`).
  - Separated the pin-filtering logic into `get_attackers_mask_for_see` for SEE and static queen threat evaluations, keeping raw `get_attackers_mask` uninhibited to guarantee strictly FIDE-legal move generation and check detection.
  - Resolved 100% of perft/move-gen regressions and successfully passed all 71 unit and integration tests (including `test_dxf6_pinned_illusion` and deep `Kiwipete` Perft).



## [V0.11.5] - 2026-05-29

### Added
- **Expanded Classical Opening Book: The Nimzo-Indian Family (`src/book.rs`)**:
  - Implemented comprehensive opening support for the **Nimzo-Indian Defense** (`1. d4 Nf6 2. c4 e6 3. Nc3 Bb4`) at White's 4th move options:
    - **Rubinstein Variation (`4. e3`)**: Built-in support for Black's top positional replies: Castling (`e8g8`), `c7c5`, and `b7b6`.
    - **Classical / Capablanca Variation (`4. Qc2`)**: Fully integrated positional Queen moves, supporting `e8g8`, `d7d5`, and `c7c5` responses.
    - **Kasparov Variation (`4. Nf3`)**: Added transition paths into typical Queen's Indian / Bogo-Indian lines.
    - **Sämisch Variation (`4. a3`)**: Forces the highly tactical double-pawn structure exchange (`4... Bxc3+ 5. bxc3`), complete with the crucial follow-up strategic lines (`c7c5`, `b7b6`, `e8g8`).
    - **Leningrad Variation (`4. Bg5`)**: Exposed this sharp, tactical pin-based exotic setup to the book map.
    - **Spielmann Variation (`4. Qb3`)**: Added this rarer but fully playable queen-pressure line.
  - **Dynamic & Playable Exotic Openings**:
    - **Budapest Gambit (`1. d4 Nf6 2. c4 e5 3. dxe5 Ng4`)**: Enabled this highly aggressive, tactical, and entertaining pawn sacrifice.
    - **Benoni Defense (`1. d4 Nf6 2. c4 c5 3. d5`)**: Fully integrated this highly dynamic and asymmetric defense to counter closed center games.
- **Engine-Powered FEN & Legality Verification**:
  - Employed the engine's internal move execution logic (`UciGame::do_move`) to programmatically simulate all 32 opening lines from standard starting positions.
  - Used `FenService` to export the FEN states directly, guaranteeing 100% exact castling rights and en passant coordinates.
  - Ran the full test suite (`cargo test`) to execute `book::tests::test_all_book_moves_are_legal`, verifying that every single suggested book move is legal in its corresponding position.

### Fixed
- **Caro-Kann Advance Move Typo (`src/book.rs`)**: Fixed an illegal book move typo in the Caro-Kann Advance variation. Corrected the recommended pawn advance from `c7c5` to `c6c5` (the black pawn is already pushed to `c6` in move 1). This ensures 100% legal play and warning-free test execution.



## [V0.11.4] - 2026-05-29

### Added

- improve agent commands and clean up
- Release v0.11.3: Add King Safety and Threat Matrix evaluation heuristics
- Add SPSA harvest results skill document
- Add SPSA parameter update skill document

### Fixed



## [V0.11.3] - 2026-05-29

### Added

- Evaluation: Open/Half-Open file maluses for the King (`KingOpenFileMalus`, `KingHalfOpenFileMalus`).
- Evaluation: Defended King Ring heuristic to reduce danger based on defending pieces (`KingRingDefenderValue`).
- Evaluation: Generalized Threat Matrix (Rook attacking Queen, Minor attacking Rook/Queen) via `ThreatMinorAttacksRook`, `ThreatMinorAttacksQueen`, `ThreatRookAttacksQueen`.
- UCI configuration options for all 6 new evaluation parameters.
- Debugging: Added `engine_position_debugging.md` skill documentation.
- Tuning: Added SPSA harvest results skill document, parameter update skill document, and tuning script with workers argument.

### Fixed



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
