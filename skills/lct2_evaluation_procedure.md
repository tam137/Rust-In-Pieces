---
name: LCT2 Evaluation Procedure
description: Procedure for compiling, running, and documenting Louguet Chess Test II (LCT II) evaluations. Use when LCT II evaluation is explicitly requested.
---

# LCT2 Evaluation Procedure

This document outlines the mandatory procedure for compiling, executing, and documenting a Louguet Chess Test II (LCT II) evaluation.

## 1. Prerequisites & Ordering
Compiling the optimized release binary **first** is mandatory before running the evaluation.
```bash
cargo build --release
```
> [!WARNING]
> **CRITICAL ORDER WARNING**: You MUST compile the release binary (`cargo build --release`) BEFORE running the LCT II evaluator script. Running the evaluator on a stale binary will yield invalid results representing old code, potentially missing critical bugs or regression.

## 2. Running the Evaluator
To run the LCT II tactical/positional evaluator, execute the Python script:
```bash
python3 scripts/lct2_evaluator.py
```
This script will search 35 positions (positional, tactical, and endgame studies) and calculate:
- Total Points
- Estimated ELO rating
- Solved/unsolved counts per category

## 3. Documentation in LCT.md
You MUST document all LCT II test results in the file [LCT.md](file:///home/tam137/git/suprah/LCT.md) using the following guidelines:

- **Historical Comparison Table**: Update or prepend the row for the evaluated version under the `## Historical Comparison` table. The row must contain:
  `| Version | ELO | Total Points | Solved | Positional | Tactical | Endgame |`
- **Scoreboard & Detailed Results**: Prepend the detailed Scoreboard by Category and the Detailed Results table for the new version.
- **Language Policy**: All text, headers, and descriptions inside `LCT.md` must be written in English.
- **Prepend History**: Always prepend or append the new version section in a structured format to allow historical comparisons over time.
