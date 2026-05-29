# Evaluation Tasks & Optimizations

## 1. Missing or Incorrect Position Evaluation


---

## 2. Performance Bottlenecks

### 2.1 Lack of Incremental Evaluation (Lazy Evaluation)
- **Problem**: For every leaf node during the search, the entire board is scanned (`while temp != 0`). PST and material values are summed up entirely from scratch every single time. This is the biggest performance killer.
- **Solution**: Store material and PST sums as state variables (`eval_score`) in the `Board` struct and update them incrementally during `do_move` and `undo_move` (Incremental Updates). The evaluation function will then only need `O(1)` time for the base score calculation.
- **Complexity**: High - Requires adjustments in `model.rs` (`Board` and `do_move`/`undo_move`).

