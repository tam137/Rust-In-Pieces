---
name: eval_improvement_procedure
description: Guidelines and procedure for analyzing, implementing, and tuning improvements to the evaluation function (HCE). Focuses on performance, orthogonality, and tapered evaluation.
---

# Eval Improvement Procedure

This skill defines the standard procedure for planning, implementing, and optimizing new evaluation features in the engine.

Since the evaluation function (`EvalService::calc_eval`) is extremely performance-critical and has a direct impact on playing strength, every change must be executed systematically and evidence-based (through SPSA Tuning / SPRT).

## 1. Checking for Redundancy and Orthogonality
Before implementing a new feature, critically question whether the desired knowledge is already implicitly covered by other parameters.
- **The Problem of Collinearity:** If two features correlate strongly (e.g., "Bishop pair bonus" and "Number of open diagonals for bishops"), the SPSA tuning parameters will start working against each other (one becomes huge, the other negative).
- **Checklist:** 
  - What exact chess knowledge is being added?
  - Are there already parameters (e.g., in `Config`) that cover similar positional patterns?
  - Can the new feature be isolated so that it *only* evaluates the new aspect?

## 2. Game Phases and Tapered Evaluation
The importance of almost every chess concept changes depending on the game phase. A king in the center is deadly in the middlegame, but essential in the endgame.
- **Tapered Eval:** The system uses a sliding interpolation (`calculate_weighted_eval(mg_eval, eg_eval, game_phase)`).
- Every new feature should default to having two parameters:
  - `feature_name_mg` (Middlegame weight)
  - `feature_name_eg` (Endgame weight)
- Consider beforehand: What effect does the feature have when almost all pieces are traded off? If the effect only exists in one phase, set the other parameter to 0 initially, but allow the tuner to verify this.

## 3. Performance and Bottlenecks
Performance is the most important criterion. A feature that brings +5 Elo in knowledge but reduces the node count (NPS) by 10% will result in a net Elo loss.
- **Forbidden Patterns:** 
  - No nested loops over all 64 squares inside `calc_eval`.
  - No expensive calculations that are not optimized by Bitboards.
- **Allowed/Desired Patterns:**
  - Bitboard operations (AND, OR, XOR, Shifts).
  - Iteration over set bits using `while bitboard != 0 { let sq = bitboard.trailing_zeros(); bitboard &= bitboard - 1; }`.
  - Caching / Precalculations (e.g., precalculating Passed Pawns or Outposts at the start of `calc_eval`).
- **Critical Trade-off:** Is the computation time proportionate to the chess benefit?

## 4. Implementation Workflow
When a change is planned, follow this process:

1. **Analysis & Hypothesis:** Define what chess knowledge is missing (e.g., by analyzing lost games).
2. **Code Integration:** 
   - Add the new parameters (`_mg`, `_eg`) to the `Config` struct in `src/config.rs`.
   - Set logical initial values (often very conservative, e.g., 5 or 10 centipawns).
   - Implement the Bitboard logic in `src/eval_service.rs`.
3. **Performance Test:** Run benchmarks (`cargo bench` or `run_bench.py`) to ensure NPS doesn't drop significantly.
4. **SPSA Tuning:** 
   - Add the new parameters to the tuning script.
   - Let the tuner find the optimal values for MG and EG.
5. **Verification (SPRT):** Run a self-play tournament against the master branch to ensure the change (with tuned parameters) yields an Elo gain.

## 5. Traceability
If a feature is complex, ensure it is printed correctly during debugging (`print_eval_per_figure`) so developers can understand *why* a specific position evaluates high or low.
