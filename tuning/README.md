# Simultaneous Perturbation Stochastic Approximation (SPSA) Tuning

This directory contains the implementation of the SPSA algorithm used for tuning evaluation parameters of the Suprah chess engine. SPSA is a highly efficient stochastic optimization method specifically suited for objective functions that are costly to evaluate and highly noisy, such as playing hundreds of chess games.

## Core Concept

Unlike traditional gradient descent, which requires perturbing each parameter individually to estimate partial derivatives (requiring `2 * N` evaluations for `N` parameters), SPSA estimates the gradient for all parameters simultaneously using only two evaluations (two engine variants playing against each other).

By running this process iteratively, the optimizer finds the global maximum of the objective function (the win rate of the engine).

## Technical Workflow

A single iteration of the SPSA tuning script (`spsa_tuner.py`) consists of the following technical steps:

### 1. Simultaneous Perturbation
For every active parameter, a random perturbation direction `delta` is chosen using a Bernoulli distribution (`+1` or `-1`). 
The step size of the perturbation is dynamically calculated based on the current absolute value of the parameter. This dynamic scaling ensures that parameters with large values (e.g., 300) receive proportionally larger perturbations than parameters with very small values (e.g., 2).

### 2. Clamping and Variant Creation
Two parameter sets are generated:
- `theta_plus`: The current parameters shifted by `+step_size * delta`.
- `theta_minus`: The current parameters shifted by `-step_size * delta`.

Before these sets are finalized, every parameter is strictly clamped to its predefined `[min, max]` boundaries defined in `parameters.json`. This guarantees that the engine is never exposed to invalid or unstable parameter configurations.

### 3. Match Execution
The script spawns a batch of matches (e.g., 500 games) using a parallelized thread pool. `Matt-Magie` acts as the match manager. The games are played with alternating colors:
- Even-indexed games: `theta_plus` plays as White, `theta_minus` as Black.
- Odd-indexed games: `theta_minus` plays as White, `theta_plus` as Black.

### 4. Gradient Estimation
Once the match batch is completed, the win rate (score) of `theta_plus` against `theta_minus` is calculated. The difference in performance (`diff = 2.0 * score - 1.0`) is used to estimate the gradient `g_k` for all parameters simultaneously.

Because the calculation avoids dividing by the individual step sizes, the gradient directly reflects the shift in the objective function.

### 5. Pure SPSA Parameter Update (Direct Gradient Descent)
SPSA gradients directly drive parameter updates per iteration without artificial momentum damping:

1. A raw update vector is calculated by multiplying the gradient `g_k` with the dynamically decaying learning rate `a_k` and the parameter's step size scale (`step_sizes[k]`).
2. The baseline parameters `theta` are updated directly: `theta[k] += raw_update`.
3. Finally, the updated parameters are strictly clamped to their legal `[min, max]` boundaries defined in `parameters.json`.

### 6. State Persistence
The current state of the tuning process (the iteration counter `k` and the parameter vector `theta`) is serialized into `spsa_state.json`. A history of all scores and parameter trajectories is appended to `spsa_history.csv`. This allows the tuning process to be safely interrupted and resumed at any time.

## Usage

The tuner is invoked via the command line, requiring paths to the engine and the match manager.

Example usage (as seen in `tuning.sh`):
```bash
python3 spsa_tuner.py \
    --group search_and_ordering \
    --engine ../engines/suprah-0.20.1 \
    --mm ../target/release/Matt-Magie \
    --book ../books/Performance.bin \
    --games 800 \
    --workers 3 \
    --time 0.5 \
    --inc 25 \
    --lr 2.0
```

### Logging and Telemetry
By default, the script passes a `logpath` argument to the UCI options of the engines. The engines initialize a custom file writer and log their complete active parameter sets for debugging and plausibility verification to `enginelogs/`.
