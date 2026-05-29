# Evaluation Tasks & Optimizations

## 1. Missing or Incorrect Position Evaluation

### 1.1 PST (Piece-Square Tables) for Queen and Rook
- **Problem**: There are no position-dependent evaluations (PST) for queens and rooks in the evaluation (`eval_service.rs`). While rooks on open files are evaluated, general centralization is missing.
- **Solution**: Add dedicated PST arrays `ROOK_PST` and `QUEEN_PST` and apply them in `white_rook`, `black_rook`, `white_queen`, `black_queen`.
- **Complexity**: Low

### 1.2 Incorrect Game-Phase Calculation (`get_game_phase`)
- **Problem**: The game-phase is calculated using all pieces on the board, including pawns (`board.occupied.count_ones()`). Pure pawn endgames (many pawns, no minor/major pieces) are incorrectly classified as middlegames, which means endgame knowledge is not applied.
- **Solution**: In `get_game_phase`, only minor and major pieces (knights, bishops, rooks, queens) should be counted, ideally weighted by their material value.
- **Complexity**: Low


### 1.4 Missing King Safety Concept (Attacker Count Weighting)
- **Problem**: Attacks on the king-ring are only added linearly. In reality, two attackers are far more than twice as dangerous as a single attacker.
- **Solution**: Introduce a "King Danger" score that takes the number of attacking pieces (Attacker Count) into account and weights them exponentially/quadratically, rather than just summing the attacks linearly.
- **Complexity**: Medium


---

## 2. Performance Bottlenecks

### 2.1 Lack of Incremental Evaluation (Lazy Evaluation)
- **Problem**: For every leaf node during the search, the entire board is scanned (`while temp != 0`). PST and material values are summed up entirely from scratch every single time. This is the biggest performance killer.
- **Solution**: Store material and PST sums as state variables (`eval_score`) in the `Board` struct and update them incrementally during `do_move` and `undo_move` (Incremental Updates). The evaluation function will then only need `O(1)` time for the base score calculation.
- **Complexity**: High - Requires adjustments in `model.rs` (`Board` and `do_move`/`undo_move`).

### 2.2 Dynamic Calculation of `get_king_ring`
- **Problem**: `get_king_ring` is dynamically and expensively calculated for every single eval call (millions of times per second) using two nested loops.
- **Solution**: Precalculate a static array `const KING_RING_MASKS: [u64; 64]` and use it as a lookup table.
- **Complexity**: Low

### 2.3 Loops for Passed Pawn Detection
- **Problem**: `is_white_passed_pawn` and `is_black_passed_pawn` iterate through a loop for each pawn up to the promotion rank.
- **Solution**: Use precalculated constant arrays: `const PASSED_PAWN_MASKS: [[u64; 64]; 2]`. Query via bitwise AND (`(black_pawns & PASSED_PAWN_MASKS[WHITE][sq]) == 0`).
- **Complexity**: Low

### 2.4 Expensive Static Move List Sorting (Selection Sort)
- **Problem**: The entire MoveList is fully sorted in `search_service.rs` using `O(N^2)` Selection Sort before even the first move is tried. However, the 1st or 2nd move usually leads to a beta cutoff.
- **Solution**: Implement "Lazy Move Sorting" or a "Move Picker". Only evaluate and pick the next best move when the loop in the search tree actually requests it.
- **Complexity**: Medium

### 2.5 SEE Calculation in Move Generation / Pre-Sorting
- **Problem**: SEE (Static Exchange Evaluation) is currently called preemptively for all captures to sort them (`see_ge`). However, SEE is essentially a small search function and relatively expensive.
- **Solution**: Evaluate SEE "on-the-fly" within the search loop only when a move is actually going to be considered (especially in Quiescence Search for pruning decisions).
- **Complexity**: Medium
