---
name: "spsa-parameter-update"
description: "Workflow for adding new SupraH evaluation parameters to the SPSA tuning pipeline"
---

# SPSA Tuning Parameter Update Skill

This skill defines the standard operating procedure for integrating newly developed evaluation parameters into the automated SPSA tuning pipeline for the SupraH chess engine.

## Prerequisites
- The developer has added new parameter fields to the `Config` struct in `src/config.rs`.
- The developer has implemented the evaluation logic using these new parameters in `src/eval_service.rs` or similar files.

## Workflow Steps

### 1. Identify New Parameters
Analyze `src/config.rs` to identify any newly added fields in the `Config` struct. Check both the struct definition and the `default()` implementation to understand the baseline values.

### 2. Verify UCI Compatibility
Before a parameter can be tuned via SPSA, the engine must be able to receive it via the UCI `setoption` command.
- Open `src/game_handler.rs`.
- Locate the `setoption` parsing block.
- **Action Required:** Ensure that every new parameter from `config.rs` has a corresponding matching string in the `match` statement that maps the UCI string to the `config` field.
  - *Example:* `"king_open_file_malus" => config.king_open_file_malus = value as i16,`

### 3. Update the Tuning JSON (`tuning/parameters.json`)
The `spsa_tuner.py` script relies on `tuning/parameters.json` to know which parameters to perturb and what their absolute bounds are.
- Open `tuning/parameters.json`.
- Add a new JSON block for each new parameter.
- Set the `value` to the default value defined in `config.rs`.
- Determine reasonable `min` and `max` bounds (typically `value - 100` and `value + 100`, but capped appropriately so e.g. bonuses don't become negative if it breaks logic).
  
```json
"king_open_file_malus": {
    "value": 40,
    "min": 0,
    "max": 150
}
```

### 4. Handle Existing SPSA State
If a tuning session is currently active on the server, modifying `parameters.json` is not enough, because `spsa_tuner.py` loads its current parameter set from `tuning/spsa_state.json`.
- **Option A (Reset):** Delete `tuning/spsa_history.csv` and `tuning/spsa_state.json` to start a completely fresh tuning run from iteration 1.
- **Option B (Hot-Patch):** If you wish to keep the tuning progress of the old parameters, you must manually edit `tuning/spsa_state.json` to inject the new parameters into the `"theta"` dictionary with their default values. Also, you must manually add the new parameter column names to the header row of `tuning/spsa_history.csv` and pad all previous rows with their default values. *(Option A is usually preferred and significantly less error-prone).*

### 5. Compile and Deploy
Once the code and tuning files are updated:
1. Run `./build_and_release.sh` to compile the new native binary and deploy it to the remote server.
2. Upload the updated tuning files to the server:
   ```bash
   scp tuning/parameters.json root@<SERVER_IP>:/root/mattmagie/tuning/
   ```
3. SSH into the server, ensure `spsa_state.json` is handled (deleted or patched), and restart the `spsa_tuner.py` script inside a tmux session.
