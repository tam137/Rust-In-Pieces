
## 6. Fix Pinned Piece Illusion in Evaluation & Search (Qxf6 / Qxd4 Bug)

### Problem Description
In the Richter-Rauzer opening after `1... Qxf6 2. Nd5 Qd8`, the engine completely overlooks the winning move `3. Qxd4` (capturing the d4 pawn). The root cause is that **pinned pieces are treated as active attackers and defenders**:
1. **Quiescence Search Pruning (SEE)**: The engine's Static Exchange Evaluation (SEE) sees the pinned Black Knight on c6 and falsely concludes that `Qxd4` blunders the Queen to the Knight (SEE score `-850`). Consequently, `Qxd4` is pruned entirely from Quiescence Search.
2. **Evaluation Penalty (Queen in Attack)**: In static evaluation, the engine penalizes `3. Qxd4` by `90` points (`queen_in_attack` + `tempo`), falsely believing the White Queen on d4 is under threat by the pinned Knight on c6.

### Tasks to Complete
- `[x]` **Absolute Pin Detection**: Implement a helper function in `move_gen_service.rs` to detect if a piece of a given color is absolutely pinned to its king.
- `[x]` **Filter Pinned Pieces in `get_attackers_mask`**: Update the `get_attackers_mask` function in `move_gen_service.rs` to filter out or ignore attackers that are absolutely pinned (unless they are pinned along the line of the target square).
- `[x]` **SEE Integration**: Ensure `get_least_valuable_attacker` (which is used by SEE) correctly respects the absolute pin check.
- `[x]` **Static Eval Integration**: Ensure `calc_eval` (specifically `white_queen` and `black_queen` attack checks) ignores threats from absolutely pinned opponent pieces.
- `[x]` **Unit & Integration Testing**: Add dedicated tests in `search_service.rs` (similar to `test_dxf6_bug`) to verify that the FEN `r1bqkb1r/ppp2ppp/2np1B2/1B6/3pP3/2N5/PPP2PPP/R2QK1NR b KQkq - 0 6` results in `d8f6` (Qxf6) for Black and that White correctly finds `Qxd4` as the superior follow-up.
