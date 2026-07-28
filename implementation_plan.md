# Implementation Plan - Futility Pruning Unit Tests & Release V0.22.2

This plan describes adding comprehensive unit tests for Futility Pruning (FP) and releasing version `v0.22.2`.

---

## Proposed Changes

### `src/search_service.rs`
Add two dedicated unit tests in `mod tests`:
1. `test_futility_pruning_node_reduction`:
   - Verifies that `enable_futility_pruning: true` strictly reduces the total number of searched nodes compared to `enable_futility_pruning: false` at depth 4 on a standard quiet position.
2. `test_futility_pruning_tactical_safety_guards`:
   - Verifies that Futility Pruning operates safely on tactical positions without missing key moves or producing invalid scores.

---

## Verification & Release Procedure

1. **Unit Tests & Clean Compilation:**
   - Execute `cargo test` and verify all tests pass without compiler warnings.
2. **Release Binary Compilation:**
   - Execute `cargo build --release` locally.
3. **Automated Pipeline Script:**
   - Run `./build_and_release.sh "Add Futility Pruning unit tests for node reduction and tactical safety"`.
4. **Post-Release Finalization:**
   - Enrich `CHANGELOG.md` with technical explanations.
   - Commit changes, create git tag `v0.22.2`, and push to origin master with tags.
