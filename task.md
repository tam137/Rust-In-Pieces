# Automated Parameter Tuning via SPSA

This document outlines the roadmap and procedure for implementing **SPSA (Simultaneous Perturbation Stochastic Approximation)** to fine-tune the evaluation and search parameters in SupraH.

## 1. Concept of SPSA in Chess Engines

SPSA is an algorithmic method for optimizing systems with multiple unknown parameters by approximating the gradient of the objective function from stochastic (noisy) measurements. In chess engine development, the "objective function" is the engine's ELO/playing strength, and the "measurements" are the results of automated match-ups (games).

Instead of testing one parameter at a time (which is too slow for 50+ parameters), SPSA randomly perturbs *all* parameters simultaneously (some up, some down), plays a batch of fast games, and uses the match results to calculate a gradient step for all parameters at once.

## 2. Requirements for SPSA

To run an SPSA tuning session, we need:
1. **Configurable Engine Parameters**: All tunable weights must be accessible dynamically (already done in `src/config.rs`).
2. **UCI Command Interface for Tuning**: The engine must allow external tools to inject parameter values at startup.
   - Example UCI command: `setoption name positional_cap_damping value 5`
3. **An SPSA Tuner**: A script or external tool (like `OpenBench`, `Cutechess-cli` paired with an SPSA script, or a custom Python script) that orchestrates the matches and updates the parameters.
4. **Opening Book / PGNs**: A balanced set of opening positions to ensure both engines play the same positions as White and Black.

## 3. Implementation Roadmap

### Phase 1: Engine Preparation (UCI Exposure)
- `[ ]` **Expose all parameters via UCI**: Ensure every parameter in `src/config.rs` (e.g., LMR depth threshold, Killer move bonuses, PST values, material values) is accessible via standard UCI `setoption name <Name> value <Value>` commands.
- `[ ]` **Print current parameters**: Implement a UCI command (e.g., `uci` output or custom command) that prints all parameters and their valid min/max ranges so the tuner can read them.

### Phase 2: SPSA Tooling Setup
- `[ ]` **Select SPSA Tooling**: We will write a custom Python SPSA orchestration script or integrate an existing open-source tool (like `tuning.py` often used with `cutechess-cli`).
- `[ ]` **Prepare the Match Environment**: Ensure `cutechess-cli` or a similar match runner is installed.
- `[ ]` **Prepare Opening Book**: Download a robust, short-depth opening book (e.g., UHO - Unbalanced Human Openings) in EPD/PGN format for the matches.

### Phase 3: The Tuning Loop
- `[ ]` **Initialize**: Define the starting values, minimum bounds, maximum bounds, and step sizes (c, a, A, alpha, gamma variables for SPSA).
- `[ ]` **Perturbation**: The tuner generates two parameter sets (Set A = Base + Perturbation, Set B = Base - Perturbation).
- `[ ]` **Match Batch**: Run N fast bullet games (e.g., 10+0.1s) between `SupraH (Set A)` and `SupraH (Set B)`.
- `[ ]` **Gradient Update**: Calculate the score. If Set A won more, parameters move in the direction of Set A's perturbation. If Set B won, they move toward Set B.
- `[ ]` **Iterate**: Repeat for thousands of games until convergence.

## 4. Prioritized Tuning Targets

The following parameter groups should be tuned first, as they offer the highest potential ELO gains:

1. **Search Parameters**:
   - `lmr_reduction`, `lmr_depth_threshold`, `lmr_base_divisor`
   - `nmp_reduction`, `nmp_depth_threshold`, `nmp_dynamic_divisor`
   - `killer_move_1_rank_bonus`, `counter_move_rank_bonus`
2. **Evaluation Tuning (Base Material)**:
   - Values of Knight, Bishop, Rook, Queen.
3. **Evaluation Tuning (Positional & PSTs)**:
   - `positional_cap_damping`
   - Piece-Square Table exact square values (MG and EG).

## 5. Execution Strategy

Since SPSA requires thousands of games, it should be run asynchronously on a dedicated server (like the remote ARM machine or a fast local workstation). Once a tuning session yields a stable, improved parameter set, the values are permanently baked into `src/config.rs` in a new Patch/Minor release.

## 6. Fix Pinned Piece Illusion in Evaluation & Search (Qxf6 / Qxd4 Bug)

### Problem Description
In the Richter-Rauzer opening after `1... Qxf6 2. Nd5 Qd8`, the engine completely overlooks the winning move `3. Qxd4` (capturing the d4 pawn). The root cause is that **pinned pieces are treated as active attackers and defenders**:
1. **Quiescence Search Pruning (SEE)**: The engine's Static Exchange Evaluation (SEE) sees the pinned Black Knight on c6 and falsely concludes that `Qxd4` blunders the Queen to the Knight (SEE score `-850`). Consequently, `Qxd4` is pruned entirely from Quiescence Search.
2. **Evaluation Penalty (Queen in Attack)**: In static evaluation, the engine penalizes `3. Qxd4` by `90` points (`queen_in_attack` + `tempo`), falsely believing the White Queen on d4 is under threat by the pinned Knight on c6.

### Tasks to Complete
- `[ ]` **Absolute Pin Detection**: Implement a helper function in `move_gen_service.rs` to detect if a piece of a given color is absolutely pinned to its king.
- `[ ]` **Filter Pinned Pieces in `get_attackers_mask`**: Update the `get_attackers_mask` function in `move_gen_service.rs` to filter out or ignore attackers that are absolutely pinned (unless they are pinned along the line of the target square).
- `[ ]` **SEE Integration**: Ensure `get_least_valuable_attacker` (which is used by SEE) correctly respects the absolute pin check.
- `[ ]` **Static Eval Integration**: Ensure `calc_eval` (specifically `white_queen` and `black_queen` attack checks) ignores threats from absolutely pinned opponent pieces.
- `[ ]` **Unit & Integration Testing**: Add dedicated tests in `search_service.rs` (similar to `test_dxf6_bug`) to verify that the FEN `r1bqkb1r/ppp2ppp/2np1B2/1B6/3pP3/2N5/PPP2PPP/R2QK1NR b KQkq - 0 6` results in `d8f6` (Qxf6) for Black and that White correctly finds `Qxd4` as the superior follow-up.
