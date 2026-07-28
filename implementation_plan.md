# Implementation Plan - Baseline Fidelity Release V0.22.4

This plan describes disabling both Lazy Evaluation (`enable_lazy_eval: false`) and Futility Pruning (`enable_futility_pruning: false`) by default in version `v0.22.4` to establish a full search tree evaluation fidelity baseline.

---

## Proposed Changes

### 1. `src/config.rs`
Set default flags in `Config::new()`:
- `enable_lazy_eval`: `false` (remains disabled for full positional accuracy)
- `enable_futility_pruning`: `false` (disabled by default)

### 2. `src/search_service.rs`
- Ensure all existing unit tests pass cleanly regardless of default configuration.

---

## Verification & Release Procedure

1. **Unit Tests & Clean Compilation:**
   - Execute `cargo test` and verify 100% clean test execution without warnings.
2. **Release Binary Compilation:**
   - Execute `cargo build --release` locally.
3. **Automated Pipeline Script:**
   - Run `./build_and_release.sh "Disable both Lazy Evaluation and Futility Pruning by default for full search fidelity"`.
4. **Post-Release Finalization:**
   - Enrich `CHANGELOG.md` with technical explanations.
   - Commit changes, create git tag `v0.22.4`, and push to origin master with tags.
