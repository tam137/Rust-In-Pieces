---
name: "spsa-harvest-results"
description: "Workflow for downloading SPSA tuned parameters from the server and integrating them into the SupraH source code"
---

# SPSA Tuning Result Harvest Skill

This skill defines the standard operating procedure for extracting the latest optimized parameters from a running or completed SPSA tuning session on the remote server and hardcoding them into the SupraH default configuration.

## Prerequisites
- An active or completed SPSA tuning run on the remote server (`/root/mattmagie/tuning/spsa_state.json`).

## Workflow Steps

### 1. Download the Latest Tuning State
Connect to the remote server via SCP or SSH to retrieve the current mathematical state of the tuning process.

```bash
# Example command to fetch the state file
scp root@<SERVER_IP>:/root/mattmagie/tuning/spsa_state.json tuning/spsa_state_remote.json
```

### 2. Parse and Process the Tuned Parameters
- Read the downloaded `spsa_state_remote.json`.
- Extract the `"k"` value to log which iteration these results belong to.
- Extract the `"theta"` dictionary containing the raw tuned values.
- **Rounding:** Since `spsa_tuner.py` works with floating-point math for gradients, but `config.rs` requires exact integers (`i16` or `i32`), round every value in `"theta"` to the nearest whole integer.

### 3. Update `src/config.rs`
The engine must use the newly optimized values as its baseline out of the box.
- Open `src/config.rs`.
- Locate the `impl Default for Config` block (specifically the `fn default()` function).
- For every parameter present in the `"theta"` dictionary, update the hardcoded assignment in `default()` to match the newly rounded integer.
- *Caution:* Ensure no structural logic or non-tuning config variables (like `search_threads`) are accidentally overwritten.

### 4. Verify the Build
Run Rust's compiler checks to ensure that no types were mismatched during the update (e.g. accidentally inserting a float).
```bash
cargo test
```

### 5. Document and Commit
Create a commit that clearly indicates the parameters were updated via an SPSA harvest, including the iteration number for future reference.

```bash
git commit -am "Apply SPSA tuned parameters from iteration <K>"
```

### 6. (Optional) Restart SPSA with New Baseline
If tuning is meant to continue:
- Update `tuning/parameters.json` so the `"value"` attributes match the newly baked-in defaults.
- Wipe or archive the remote `spsa_history.csv` and `spsa_state.json`.
- Deploy the newly compiled engine to the server to begin a fresh tuning run anchored at the new optimized baseline.
