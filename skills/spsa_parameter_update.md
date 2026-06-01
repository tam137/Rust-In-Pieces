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

> [!IMPORTANT]
> **Why Engine Recompilation & Redeployment is Mandatory:**
> If you are adding a **brand-new parameter** (such as `easy_move_margin` in the `Config` struct and UCI thread commands), the chess engine binary itself **MUST** be recompiled and redeployed to both local and remote directories before launching the tuner.
> 
> If you only upload `parameters.json` without compiling the new binary, `spsa_tuner.py` will send the new UCI option command (e.g. `setoption name EasyMoveMargin value 150`) to the old engine binary. Because the old binary does not recognize this new option, it will throw a UCI error or crash.
> 
> Recompiling and deploying a new engine version ensures that the engine fully supports and parses the new UCI option correctly on the remote ARM server.

1. **Recompile and Deploy the Engine:**
   Run the automated build & release pipeline script to compile the binary and automatically deploy it locally and remotely:
   ```bash
   ./build_and_release.sh "Release vX.Y.Z: Registered new tuning parameters"
   ```
2. **Update tuning.sh:**
   Open `tuning/tuning.sh` and update the `--engine` parameter to point to the newly released binary path (e.g., `../engines/suprah-X.Y.Z`), then upload it to the server:
   ```bash
   scp tuning/tuning.sh root@<SERVER_IP>:/root/mattmagie/tuning/
   ```
3. **Upload the updated parameter configurations:**
   ```bash
   scp tuning/parameters.json root@<SERVER_IP>:/root/mattmagie/tuning/
   ```
4. **Restart SPSA on the Server:**
   SSH into the server, stop the active SPSA process (Ctrl+C in tmux), and clean up the remote state/history files along with any cluttered temporary match PGN files (`tmp_*.pgn`) inside `~/mattmagie/tuning/`:
   ```bash
   # Clean up state and temporary pgns
   rm -f ~/mattmagie/tuning/spsa_history.csv ~/mattmagie/tuning/spsa_state.json ~/mattmagie/tuning/tmp_*.pgn
   ```
   Finally, launch `./tuning.sh` in the `spsa_tuning` tmux window to start a fresh, mathematically clean SPSA tuning run.
