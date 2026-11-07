---
name: "spsa-parameter-update"
description: "Workflow for adding new SupraH evaluation parameters to the SPSA tuning pipeline"
---

# SPSA Tuning Parameter Update Skill

This skill defines the standard operating procedure for integrating newly developed evaluation parameters into the SPSA tuning pipeline for the SupraH chess engine.

## Prerequisites
- The developer has added new parameter fields to the `Config` struct in `src/config.rs`.
- The developer has implemented the evaluation logic using these new parameters in `src/eval_service.rs` or similar files.

## Workflow Steps

### 1. Identify New Parameters
Analyze `src/config.rs` to identify any newly added fields in the `Config` struct. Check both the struct definition and the `default()` implementation to understand the baseline values.

### 2. Verify UCI Compatibility
Before a parameter can be tuned, the engine must receive it via the UCI `setoption` command.
- Open `src/game_handler.rs` and locate the `setoption` parsing block.
- **Action Required:** Every new parameter needs a matching string in the `match` statement that maps the UCI name to the `config` field.
  - *Example:* `"king_open_file_malus" => config.king_open_file_malus = value as i16,`
- The tuner's preflight checks this: it aborts if the engine log shows `Unknown option ignored: <name>` or no `Received option: <name> = <value>` line.

### 3. Update the Tuning JSON (`tuning/parameters.json`)
- Add a JSON block for each new parameter.
- `value`: the default from `config.rs`.
- `min`, `max`: hard bounds that keep the logic sane (e.g. a bonus must not become negative if that breaks the logic).
- `c_end`: required for every parameter that is tuned. Choose roughly the change that costs a few Elo; it must be `>= 1` (integer parameters), and the range `max - min` must be at least `4 * c_end`. The tuner refuses to start otherwise.

```json
"king_open_file_malus": {
    "value": 40,
    "min": 0,
    "max": 150,
    "c_end": 4
}
```

- Add the parameter to the fitting group in `tuning/groups.json` if it should be tuned with that group.

### 4. Handle Existing SPSA State
The tuner resumes from `tuning/spsa_state.json` only if its `fingerprint` matches the run: group, active list, `n` (N), `r_end`, `c_end` of every active parameter, engine basename. Adding an active parameter, changing a `c_end` or releasing a new engine therefore makes the tuner refuse the state.
- **Option A (Hot-Patch, keeps progress):** Edit the state by hand, consistently:
  - `fingerprint`: update `active`, `c_end` (and `engine` for a new binary) to exactly what the new command line and `parameters.json` produce; `group`, `n`, `r_end` must match the command line.
  - `theta`: add the new parameter with its start value.
  - `k_done` and `counters` stay as they are; the schedules continue at `k_done + 1`.
  - `spsa_history.csv`: the header must list the active parameters in the new order; add the new column to the header and pad the previous rows (e.g. with the start value), or archive the old file.
  - Run `python3 spsa_tuner.py ... --dry-run` to confirm the state is accepted and the preflight passes.
- **Option B (Full Reset):** Archive or delete `tuning/spsa_state.json` and `tuning/spsa_history.csv` and start from iteration 1. Use this for a new engine variant or a new run definition.

### 5. Compile and Deploy

> [!IMPORTANT]
> A **brand-new parameter** needs a recompiled engine released into `/home/tam137/chess/mattmagie/engines` before tuning. An old binary does not know the option; the preflight then aborts the run.

1. **Recompile and Deploy the Engine:**
   ```bash
   ./build_and_release.sh
   ```
2. **Point the run at it:** set `ENGINE_BIN` (or edit the default) in `tuning/tuning.sh`. The tuner runs from the repository's `tuning/` directory; nothing needs to be copied into the Matt-Magie directory.
3. **Tuning Configuration Rules:**
   > [!IMPORTANT]
   > - **At most 12 workers:** `--workers` must not exceed 12, i.e. 12 of the 16 cores of the local host (owner, 2026-11-07; the tuner refuses more). One game uses about one core.
   > - **Mandatory group (`--group`):** groups are defined in `tuning/groups.json`. `--params a,b,c` overrides the group's list (the group is then only a label).
   > - **`--iters`** is the number of game pairs N; `--r-end` defaults to 0.002. There are no `--mutate`, `--lr`, `--book` or `--games` options any more.
4. **Restart SPSA:** stop the run (Ctrl+C in the `spsa_tuning` tmux session; in-flight pairs are discarded, the state is saved). Apply Option A or B, check with `--dry-run`, then launch `./tuning.sh` in the `spsa_tuning` tmux window.
