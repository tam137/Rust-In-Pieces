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

### 2. Manual Verification Steps

#### A. Inspect the tmux output
```bash
tmux capture-pane -t spsa_tuning -p
```
- **Progress line**: `k <k>/<N> +W =D -L disc <discarded> time <time losses> ill <illegal> <rate> pairs/h | <theta>`.
- **Startup lines**: `preflight: <n> options confirmed by the engine` must be present; any `warning:` lines (value on a bound) deserve a look.
- **Command constraints**: `--group` is given and `--workers` does **not exceed 4** (the tuner refuses more).

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

### 4. Parameter Adjustment Rules
> [!WARNING]
> **Never delete the entire tuning session just to adjust parameters!**
> - The tuner refuses to resume a state whose fingerprint differs from the command line (group, active list, N, `r_end`, `c_end`, engine basename). If an adjustment is needed, hot-patch the state by hand as described in `spsa_parameter_update.md`.
> - **Full resets are reserved** for a new engine variant or a new run definition.
