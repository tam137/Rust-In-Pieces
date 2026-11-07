---
name: "spsa-harvest-results"
description: "Workflow for collecting SPSA tuned parameters from the tuning state and integrating them into the SupraH source code"
---

# SPSA Tuning Result Harvest Skill

This skill defines the standard operating procedure for extracting the tuned parameters from a running or completed SPSA tuning session and hardcoding them into the SupraH default configuration.

## Prerequisites
- An active or completed SPSA tuning run with state `tuning/spsa_state.json` (version 2; or the path given with `--state`).

## Workflow Steps

### 1. Snapshot the Latest Tuning State
Copy the state; the snapshot keeps its historical name, which `scripts/apply_spsa.py` reads.

```bash
cp tuning/spsa_state.json tuning/spsa_state_remote.json
```

### 2. Parse and Process the Tuned Parameters
- Read the snapshot.
- Log `k_done` and `fingerprint.n` (iteration reached of N), `fingerprint.group`, `fingerprint.engine` and the `counters`.
- Take the active parameters from `fingerprint.active`. Only these are harvested; the other entries in `theta` are just the unchanged `value`s.
- **Rounding:** `theta` holds floats, `config.rs` needs integers (`i16`/`i32`): round each active value to the nearest integer.
- Compare against `value` in `tuning/parameters.json`: a change much smaller than `c_end` is noise.

### 3. Update `src/config.rs`
- Open `src/config.rs`, locate `impl Default for Config` (`fn default()`).
- For every active parameter, update the hardcoded assignment to the rounded value.
- *Caution:* Do not overwrite non-tuning config variables (like `search_threads`) or parameters that were not active.

### 4. Verify the Build
```bash
cargo test
```

### 5. Document and Commit
```bash
git commit -am "Apply SPSA tuned parameters (<group>, iteration <k_done>/<N>)"
```

### 6. (Optional) Restart SPSA with New Baseline
- Update `value` in `tuning/parameters.json` to the newly baked-in defaults.
- Archive `tuning/spsa_state.json` and `tuning/spsa_history.csv` (a new engine changes the fingerprint, so the old state would be refused anyway).
- Release the new engine with `./build_and_release.sh`, point `tuning/tuning.sh` at it (`ENGINE_BIN`), and start a fresh run.
