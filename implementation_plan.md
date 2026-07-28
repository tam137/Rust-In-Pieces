# Implementation Plan - Aggressive Futility Pruning Release V0.22.3

This plan describes tuning Futility Pruning (FP) to be more aggressive and extending its maximum depth range to `depth <= 4`, releasing version `v0.22.3`.

---

## Proposed Changes

### 1. `src/config.rs`
Adjust default Futility Pruning configuration parameters:
- `futility_max_depth`: `4` (extended from 3)
- `futility_margin_base`: `100` (reduced from 150)
- `futility_margin_slope`: `70` (reduced from 100)

*Marginal breakdown:*
- Depth 1: 170 cp
- Depth 2: 240 cp
- Depth 3: 310 cp
- Depth 4: 380 cp

### 2. `src/search_service.rs`
- Update unit tests in `mod tests` (`test_futility_pruning_node_reduction` and `test_futility_pruning_tactical_safety_guards`) to reflect `futility_max_depth = 4` and the updated margin parameters.
- Add dedicated unit test `test_mate_score_normalization_overflow_safety` for saturating arithmetic and mate score TT normalization.

---

## Verification & Release Procedure

1. **Unit Tests & Clean Compilation:**
   - Execute `cargo test` and verify all tests pass without compiler warnings.
2. **Release Binary Compilation:**
   - Execute `cargo build --release` locally.
3. **Automated Pipeline Script:**
   - Run `./build_and_release.sh "Aggressive Futility Pruning (futility_max_depth = 4, base = 100, slope = 70)"`.
4. **Post-Release Finalization:**
   - Enrich `CHANGELOG.md` with technical explanations.
   - Commit changes, create git tag `v0.22.3`, and push to origin master with tags.
