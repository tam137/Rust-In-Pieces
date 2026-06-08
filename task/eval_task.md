# Evaluation Tasks & Upgrades

This document outlines the proposed tasks for upgrading the Hand-Crafted Evaluation (HCE) function in the **Suprah** chess engine.

---

## ⚠️ Configuration Principle

Every new evaluation feature **must** be fully configurable via the `Config` struct. No hardcoded evaluation weights should be introduced.
* Each new term must have dedicated middlegame and endgame parameters (e.g. `feature_mg` and `feature_eg`).
* All parameters must be exposed in `Config` and mapped to UCI options inside `src/threads.rs` and `src/game_handler.rs` to allow SPSA tuning.

---

## Active Evaluation Tasks

### 1. Connected Passed Pawns
*   **Description**: Connected passed pawns are significantly stronger than isolated passed pawns in the endgame because they can defend each other and march together.
*   **Metadata**: `[Impact: High]` `[Complexity: Low-Medium]`
*   **Tasks**:
    - `[ ]` Define `connected_passed_pawn_mg: i16` and `connected_passed_pawn_eg: i16` in `Config`.
    - `[ ]` Implement detection in `eval_service.rs` (checking if two passed pawns are on adjacent files and either on the same rank or supporting each other).
    - `[ ]` Apply the bonus dynamically based on game phase.

### 2. True Outposts for Knights and Bishops
*   **Description**: Suprah currently rewards any knight supported by a pawn in the center. A "true outpost" is a square on ranks 4, 5, or 6 supported by a friendly pawn that **cannot be attacked by any enemy pawns** (either because they have passed or are on files where they cannot reach the outpost).
*   **Metadata**: `[Impact: High]` `[Complexity: Medium]`
*   **Tasks**:
    - `[ ]` Add `knight_outpost_true_mg: i16`, `knight_outpost_true_eg: i16`, `bishop_outpost_true_mg: i16`, and `bishop_outpost_true_eg: i16` to `Config`.
    - `[ ]` Implement outpost square verification (checking that the square is on rank 4-6, supported by a friendly pawn, and the enemy has no pawn that can attack the square).
    - `[ ]` Add bonuses for pieces occupying these true outposts.

### 3. Asymmetric Castling and King Safety
*   **Description**: Kingside and queenside castled positions require different pawn shields. Suprah currently evaluates them symmetrically, which leads to weak queenside defenses.
*   **Metadata**: `[Impact: Medium]` `[Complexity: Medium]`
*   **Tasks**:
    - `[ ]` Add separate configuration parameters for Kingside and Queenside pawn/piece shields: `king_pawn_shield_kingside: i16`, `king_pawn_shield_queenside: i16`.
    - `[ ]` Modify `white_king` and `black_king` evaluations to check if the king is on files a-c (Queenside) versus files f-h (Kingside) and apply the appropriate shield patterns.

### 4. Trapped Piece Penalties (Knights & Rooks)
*   **Description**: Currently, only trapped bishops are penalized. Knights can be trapped on the corner files (e.g. a8/h8/a7/h7) when they have no legal squares. Rooks can be trapped in the corner behind unadvanced pawns.
*   **Metadata**: `[Impact: Medium]` `[Complexity: Medium]`
*   **Tasks**:
    - `[ ]` Add `trapped_knight_malus: i16` and `trapped_rook_malus: i16` to `Config`.
    - `[ ]` Implement trapped knight detection (knight on edge, mobility of 0 or 1, and blocked by friendly pieces).
    - `[ ]` Implement trapped rook detection (rook trapped on a/b/g/h files, blocked by its own un-castled king).

### 5. Scale Down for Opposite-Colored Bishops Endgames
*   **Description**: Endgames with only kings, pawns, and one bishop of opposite colors on each side are highly drawish, even with a 1- or 2-pawn advantage. The engine should scale down the evaluation towards 0 to avoid trading into drawish endgames.
*   **Metadata**: `[Impact: High]` `[Complexity: Medium-High]`
*   **Tasks**:
    - `[ ]` Add `opposite_bishops_draw_scale: i16` (scaled by 100, e.g. 50 representing 50% scale-down) to `Config`.
    - `[ ]` Implement opposite-colored bishops endgame detection (only Kings, Pawns, and exactly 1 Bishop per side on different square colors).
    - `[ ]` Scale the final positional + material evaluation down towards 0 if this endgame condition is met.

### 6. Tarrasch Rule (Rook Behind Enemy Passed Pawn)
*   **Description**: The Tarrasch rule states that rooks belong behind passed pawns. Suprah currently evaluates this for friendly passed pawns, but not for defending against enemy passed pawns.
*   **Metadata**: `[Impact: Medium]` `[Complexity: Low-Medium]`
*   **Tasks**:
    - `[ ]` Add `rook_behind_enemy_passed_pawn_mg: i16` and `rook_behind_enemy_passed_pawn_eg: i16` to `Config`.
    - `[ ]` In rook evaluation, if the file contains an enemy passed pawn and the rook is placed behind it (e.g., rook rank is lower than white passed pawn rank, or higher than black passed pawn rank), reward the rook.
