# Simultaneous Perturbation Stochastic Approximation (SPSA) Tuning

This directory contains the SPSA tuner (`spsa_tuner.py`) for the parameters of the Suprah chess engine. SPSA suits objective functions that are costly and very noisy to evaluate, such as the result of chess games. The tuner follows the form Fishtest uses: one iteration is one game pair.

## Core Concept

SPSA perturbs all active parameters at once in a random direction and estimates the gradient from a single comparison of `theta+` against `theta-`, instead of perturbing each parameter on its own.

## Algorithm

Let `N` be the number of iterations (`--iters`, game pairs) and `k = 1..N` the iteration. Each active parameter `i` has an absolute final perturbation `c_end_i` (from `parameters.json`); `r_end` (`--r-end`, default `0.002`) is global.

```
gamma = 0.101, alpha = 0.602, A = 0.1 * N
c_k_i = c_end_i * N^gamma / k^gamma
a_k_i = r_end * c_end_i^2 * (A + N)^alpha / (A + k)^alpha
```

At the last iteration the perturbation is `c_end` and the step per unit of result is `r_end * c_end`.

One iteration:
1. Draw `delta_i = +1 / -1` for every active parameter.
2. `theta+ = theta + c_k * delta`, `theta- = theta - c_k * delta`, both clamped to `[min, max]` and rounded to integers. If the two sets round to the same values the draw is repeated (counted as `redraws`).
3. Play two games from the same opening line (a random line of `--openings`), colours swapped: `theta+` as White, then `theta+` as Black. Inactive parameters are sent at their `value`.
4. Update every active parameter, then clamp to `[min, max]`:
   ```
   theta_i += (a_k_i / c_k_i) * (W - L of theta+ in the pair) * delta_i
   ```

Up to `--workers` pairs (at most 12, i.e. 12 of the 16 cores of the local host) are in flight asynchronously. Each pair is drawn from the theta current when it was issued, and each update is applied as soon as its pair finishes.

## `parameters.json`

One entry per engine option:

```json
"king_open_file_malus": { "value": 40, "min": 0, "max": 150, "c_end": 4 }
```

- `value`: start value (and the fixed value of inactive parameters).
- `min`, `max`: hard bounds.
- `c_end`: absolute perturbation at the last iteration. Required for every active parameter; it must be `>= 1` and the range `max - min` must be at least `4 * c_end`. A value outside `[min, max]` is an error; a value on a bound is a warning.

Active parameters come from the group in `groups.json` (`--group`), or from `--params a,b,c` (then the group is only a label).

## Usage

Run from this directory; `<mm>` is the Matt-Magie working directory, resolved once at session start:

```bash
python3 spsa_tuner.py \
    --group search_and_ordering \
    --engine <mm>/engines/suprah-0.51.0 \
    --mm <mm>/target/release/Matt-Magie \
    --openings <mm>/openings_wide.txt \
    --iters 20000 \
    --workers 12
```

Options:
- `--group` (required), `--params`: active parameters.
- `--engine`, `--mm`, `--openings` (required): engine, Matt-Magie binary, opening lines (one line of UCI moves each).
- `--iters` (required): `N`, the number of game pairs.
- `--r-end` (default `0.002`).
- `--workers` (default `12`, at most `12`: 12 of the 16 cores of the local host, owner 2026-11-07).
- `--time`, `--inc`: time per game and increment per move in milliseconds (defaults `1000`, `50`; at 10 ms the engine lost 10 of 16 games on time, step 94). More than 2% time losses after 50 pairs abort the run.
- `--hash` (default `16` MB), `--seed`.
- `--param-file` (default `parameters.json`), `--start name=value,...`: starting theta other than `value` (e.g. a displaced value).
- `--state`, `--history`: default `spsa_state.json`, `spsa_history.csv` in this directory.
- `--run-dir` (default `run/`): temporary PGNs.
- `--logpath`: engine log directory; engine logs are off by default.
- `--dry-run`: preflight and print the schedule (`c_1`, `c_N`, `step_1`, `step_N` per parameter), play nothing.

`tuning.sh` and `tune_singular.sh` are example invocations.

## Preflight

Before playing, the tuner starts the engine once with a temporary log directory and sends every option a game sends (all parameters, each active one at a value other than its default, plus `Hash`). Each option must appear in the engine log as `Received option: <name> = <value>`; an `Unknown option ignored:` line or a missing line aborts the run. A parameter that never reaches the engine cannot be tuned.

## State and History

`spsa_state.json` (version 2) is written after every pair:
- `version`, `fingerprint` (group, active list, `N`, `r_end`, `c_end` of the active parameters, engine basename), `seed`, `k_done` (pairs finished), `counters` (`pairs`, `discarded`, `wins`, `draws`, `losses`, `time_losses`, `illegal`, `redraws`), `theta` (all parameters, floats).

On start an existing state is resumed at `k_done + 1`. A state whose fingerprint differs from the current run is refused (move it aside or edit it by hand). A version-1 state is moved to `spsa_state.v1.json` and a fresh run starts. Pairs in flight at an interruption (Ctrl+C) are discarded and reissued on resume.

`spsa_history.csv` gets one row per applied pair: `k, opening, result, w, d, l, <active params>` (`result = w - l` of `theta+`, theta after the update). A fresh run rewrites it.

## Failures

A pair in which a game has no result is discarded: no update, counted in `discarded`, the iteration is still used up. The run aborts after 3 discarded pairs in a row, or once 100 pairs are played when more than 1% of the pairs are discarded.

## Known limits

- The preflight sees that an option name reached the engine, not that its value parsed: the engine logs `Received option` even when it ignores an unparsable value. Every registered parameter is an integer, so this does not arise today.
- Early in a run `c_k` is about `2.4 * c_end` (at N = 5000); a parameter close to a bound is then perturbed on one side only, which shrinks its effective step. Fishtest behaves the same.
- On resume the inactive parameters keep the values stored in the state; a changed default in `parameters.json` for an inactive parameter is not picked up.
