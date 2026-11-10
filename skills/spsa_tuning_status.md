---
name: "spsa-tuning-status"
description: "Workflow for querying SPSA tuning status, checking the tmux session, and verifying the state and history for plausibility."
---

# SPSA Tuning Status & Technical Plausibility Verification Skill

This skill defines the standard operating procedure for monitoring an SPSA tuning session on this machine, displaying iteration statistics, and verifying technical and mathematical correctness.

## Prerequisites
- An SPSA tuning run on this machine, started from the repository's `tuning/` directory (state and history default to `tuning/spsa_state.json` and `tuning/spsa_history.csv`).

## Verification Workflow

### 1. Execute the Status Report
```bash
python3 scripts/check_spsa_status.py            # reads tuning/spsa_state.json
python3 scripts/check_spsa_status.py <state.json>   # or SPSA_STATE=<state.json>
```
It prints the running command, the tmux pane, iteration `k_done/N`, the counters, theta of the active parameters against `value` in `parameters.json`, and the last history rows.

The path the parameters took (Matt-Magie step 99):
```bash
python3 scripts/spsa_trajectory.py <state.json> [--param-file <parameters>]
```
It prints a header (k/N, pairs per hour, expected end, W/D/L of theta+ against theta-, time losses, illegal moves, discarded pairs, the watcher's alert file), a table of about ten checkpoints with the mean of theta over the pairs since the previous checkpoint, and per parameter the start, the current theta, the change, the change in units of `c_end` and `z`, the change over a random walk made of the run's own per-pair steps (Matt-Magie step 101). History, log and alert default to `<prefix>_history.csv`, `<prefix>_spsa.log` and `<prefix>_alert.txt` next to `<prefix>_state.json`.

A run with its own parameter file (for example a run on the NNUE branch, whose values differ from `master`) needs `--param-file`: without it both scripts compare theta against `value` in `tuning/parameters.json`, which is `master`'s start and not the run's.

### 2. Manual Verification Steps

#### A. Inspect the tmux output
```bash
tmux capture-pane -t spsa_tuning -p
```
- **Progress line**: `k <k>/<N> +W =D -L disc <discarded> time <time losses> ill <illegal> <rate> pairs/h | <theta>`.
- **Startup lines**: `preflight: <n> options confirmed by the engine` must be present; any `warning:` lines (value on a bound) deserve a look.
- **Command constraints**: `--group` is given and `--workers` does **not exceed 12** (12 of 16 cores, owner 2026-11-07; the tuner refuses more).

#### B. Read the state
```bash
cat tuning/spsa_state.json
```
- `version`: 2.
- `fingerprint`: group, active list, `n` (N), `r_end`, `c_end` per active parameter, engine basename.
- `k_done`: pairs finished (applied or discarded).
- `counters`: `pairs`, `discarded`, `wins`, `draws`, `losses`, `time_losses`, `illegal`, `redraws`.
- `theta`: all parameters; only the active ones move.

#### C. Read the history
`tuning/spsa_history.csv` has the columns `k, opening, result, w, d, l, <active params>`, one row per applied pair, theta after the update.

### 3. Critical Analysis & Interpretation
> [!IMPORTANT]
> **Thorough and Critical Review Required!**
> Do not just blindly read out the values. You must **critically analyze and interpret** the information.
> - **Are the parameters meaningful?** A malus for a trapped bishop should be a penalty, not a bonus. A value pinned at `min`/`max` or running away may indicate a bug or a bad range.
> - **Is the tuning stable?** Drift should be of the order of a few `c_end` over many pairs; wild oscillation is a flag.
> - **Failures:** `discarded` should stay near 0 (the tuner aborts after 3 in a row or > 1% once 100 pairs are played). Many `time_losses` mean the time control is too short for the host load; any `illegal` is an engine bug.
> - Inactive parameters are sent at their `value`; the preflight already confirms every option reaches the engine. Engine logs exist only if the run was started with `--logpath`.

### 4. Reporting to the Owner
Every interim report on a running SPSA session, whenever the owner asks for the status:
1. **Show the output of `scripts/spsa_trajectory.py` as a code block**: header, the trajectory table and the closing lines with the change in units of `c_end`. The trajectory is never left out; a table of the final values alone is not a status report. The owner does not see tool output, only the answer.
2. **Follow it with three to five sentences of qualitative assessment**: compare each movement with the current perturbation `c_k` (about `2.7 * c_end` early in a run, `c_end` at its end), name what moves steadily and what runs back, and whether anything approaches a bound or the watcher's rules. **Read a trend from `z`, not from the table**: the windowed means look smooth even for a random walk. Without a gradient `z` stays around +-1; `|z| >= 2` is a signal, and a real one grows as the run goes on.
3. This covers SPSA runs only. Acceptance runs (census, gauntlet, SPRT, control match) are reported with `scripts/gate.py status` and the rules of `matchplay_measurement_procedure.md`.

### 5. Parameter Adjustment Rules
> [!WARNING]
> **Never delete the entire tuning session just to adjust parameters!**
> - The tuner refuses to resume a state whose fingerprint differs from the command line (group, active list, N, `r_end`, `c_end`, engine basename). If an adjustment is needed, hot-patch the state by hand as described in `spsa_parameter_update.md`.
> - **Full resets are reserved** for a new engine variant or a new run definition.
