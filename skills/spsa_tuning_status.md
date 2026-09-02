---
name: "spsa-tuning-status"
description: "Workflow for querying SPSA tuning status, checking tmux sessions, and verifying engine parameters and option logs for plausibility."
---

# SPSA Tuning Status & Technical Plausibility Verification Skill

This skill defines the standard operating procedure for monitoring an active SPSA tuning session on the remote server, displaying iteration statistics, and verifying technical and mathematical correctness.

## Prerequisites
- An active SPSA tuning run on the remote server (`$EODSERVERIP` or `<SERVER_IP>`).
- The tuner has parameter logging enabled (setting the `logpath` UCI option).

## Verification Workflow

### 1. Execute Automated Plausibility Report
Run the automated verification script locally. This script connects to the remote server, captures the tmux window, parses SPSA state, downloads the latest engine log files, and performs safety/plausibility checks:

```bash
python3 scripts/check_spsa_status.py
```

### 2. Manual Plausibility Verification Steps
If manual verification is needed or in case of script failure, follow these steps:

#### A. Inspect Tmux Running output
Attach to or capture the remote tmux session to verify the script is printing progress cleanly and not throwing python errors:
```bash
ssh root@$EODSERVERIP "tmux capture-pane -t spsa_tuning -p"
```
- **Check Progress**: Verify the current iteration (e.g. `Iter 4`) and progress of the game batch (e.g. `Progress: 12% (90/750)`).
- **Check Command & Constraints**:
  - Look for the active command line (e.g. `python3 spsa_tuner.py --group pawns ...`).
  - **Mandatory `--group` Parameter**: Verify that a `--group` argument (e.g. `pawns`, `king_safety`, or `all`) is explicitly supplied.
  - **Maximal 4 Workers auf dem Server**: Verify that `--workers` does **not exceed 4** (`--workers 4`).

#### B. Fetch SPSA State
Check the current state parameters being adjusted:
```bash
ssh root@$EODSERVERIP "cat <REMOTE_MM_DIR>/tuning/spsa_state.json"
```
This contains:
- `k`: Current iteration number.
- `theta`: Current baseline values of the parameters.

#### C. Verify Engine Parameter Logs
Inspect the latest engine log files inside the remote logs directory:
```bash
# List latest logs
ssh root@$EODSERVERIP "ls -t <REMOTE_MM_DIR>/tuning/enginelogs/*.log | head -n 3"

# Cat the content of the latest log file
ssh root@$EODSERVERIP "cat <REMOTE_MM_DIR>/tuning/enginelogs/engine_<pid>.log"
```

Verify the following:
1. **logpath Option**: Look for the string `Received option: logpath = <REMOTE_MM_DIR>/tuning/enginelogs`. This confirms that the engine received the custom option and successfully initialized its custom file writer log callback.
2. **Current Parameter Dump**: Look for `Current Engine Parameters:` followed by the list of parameters.
3. **Parameter Plausibility Check**:
   - Compare the parameter values in the log file with the `theta` values in the remote `spsa_state.json`.
   - **Active Parameters**: The active parameters (those listed in `--params` in the command line, or all parameters if no `--params` was specified) should differ from the SPSA baseline `theta` by their perturbation step.
   - **Inactive Parameters**: The inactive parameters (those *not* listed in `--params`) **MUST be exactly equal** to their starting baseline defaults in `parameters.json` / `spsa_state.json`. If they differ, SPSA is incorrectly modifying non-targeted parameters.

### 3. Critical Analysis & Interpretation
> [!IMPORTANT]
> **Thorough and Critical Review Required!**
> Do not just blindly read out the values. You must **critically analyze and interpret** the information.
> - **Are the parameters meaningful ("sinnvoll")?** For example, a malus for a trapped bishop should mathematically be a penalty, not a bonus. A value that explodes to absurd numbers might indicate a bug.
> - **Is the tuning stable?** If the values oscillate wildly or drift into nonsensical ranges, raise a flag.

### 4. Parameter Adjustment Rules
> [!WARNING]
> **Never delete the entire tuning session (starting at iteration 1) just to adjust parameters!**
> - If parameters need to be adjusted or hot-patched, **retain the current iteration and progress**. You should carefully edit the state files (`spsa_state.json`, `spsa_history.csv`) instead of resetting.
> - **Full resets (Iteration 1) are strictly reserved** for releasing a completely new engine variant.
