# Implementation Plan - Rust-in-Pieces V0.20.0

This plan describes the proposed changes to the chess engine `rust-in-pieces` (Suprah) on the GitHub repository to resolve key search regressions and release version `V0.20.0`.

---

## Proposed Changes

### 1. File: `Cargo.toml`
* **Goal:** Bump the package version to `0.20.0`.
* **Diff:**
```diff
@@ -3,2 +3,2 @@
-version = "0.19.4"
+version = "0.20.0"
```

### 2. File: `src/config.rs`
* **Goal:** Restore the optimal SPSA-tuned parameters:
  * Re-enable positional evaluation capping (`enable_positional_cap = true`), resolving the ~99 Elo regression from V0.19.4.
  * Restore `lmr_divisor = 180` and adjust the lookup table divisor accordingly, resolving the ~19 Elo regression from V0.19.3.
* **Diff:**
```diff
@@ -190,3 +190,3 @@
-            enable_positional_cap: false,
+            enable_positional_cap: true,
@@ -301,5 +301,5 @@
-            lmr_divisor: 225,
+            lmr_divisor: 180,
 
             lmr_table: {
                 let mut table = [[0i16; 64]; 64];
-                let divisor = 225.0 / 100.0;
+                let divisor = 180.0 / 100.0;
```

---

## Verification & Deployment Plan

1. **Compilation Test:**
   * Run `cargo build --release` inside `/opt/git/rust-in-pieces/` to ensure no syntax errors.
2. **Unit Tests:**
   * Run unit tests via `cargo test` to ensure chess logic correctness.
3. **Commit & Push:**
   * Commit the changes: `git commit -am "Release v0.20.0: Restore optimal SPSA-tuned parameters (enable_positional_cap=true, lmr_divisor=180)"`
   * Push changes to origin (GitHub): `git push origin github-master:master`
