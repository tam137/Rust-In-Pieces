# Implementation Plan - Suprah Chess Engine Release V0.22.1

This plan details the implementation of Futility Pruning (FP) at low search depths (depth <= 3) and the release of version `v0.22.1`.

---

## Proposed Changes

### 1. `src/config.rs`
- Add configuration parameters for Futility Pruning:
  - `pub enable_futility_pruning: bool` (default: `true`)
  - `pub futility_max_depth: i32` (default: `3`)
  - `pub futility_margin_base: i16` (default: `150`)
  - `pub futility_margin_slope: i16` (default: `100`)

### 2. `src/search_service.rs`
- Centralize `static_eval` calculation per node prior to RFP and Futility Pruning when the side to move is not in check.
- Store the Transposition Table move (`tt_move`) extracted during TT lookup so it can be protected from pruning.
- Insert Futility Pruning in the move search loop for non-PV nodes when:
  - `config.enable_futility_pruning` is true
  - `depth <= config.futility_max_depth`
  - Current node is not in check
  - Candidate move is a quiet move (not a capture, promotion, or check-giving move)
  - Candidate move is not a priority move (not TT move, Killer move, or Counter move)
  - Alpha is not near mate (`alpha.abs() < 20000`)
  - Condition: `static_eval + margin <= alpha` (where `margin = base + slope * depth`)
- When condition triggers, skip searching the candidate move (`continue`).

---

## Verification & Release Procedure

1. **Unit Tests & Clean Compilation:**
   - Execute `cargo test` and verify zero test failures and zero compiler warnings.
2. **Release Binary Compilation:**
   - Execute `cargo build --release` locally.
3. **Automated Pipeline Script:**
   - Run `./build_and_release.sh "Implemented Futility Pruning at low depths"`.
4. **Post-Release Finalization:**
   - Manually enrich `CHANGELOG.md` with technical explanations.
   - Commit changes, create git tag `v0.22.1`, and push to origin master with tags.
