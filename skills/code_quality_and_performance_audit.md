---
name: Code Quality and Performance Audit
description: Guidelines for auditing Rust source code for compilation warnings, test coverage, performance bottlenecks, and redundant logic without changing code behavior.
---

# Code Quality and Performance Audit Procedure

This document outlines the mandatory guidelines for checking, auditing, and analyzing the codebase for compilation sanity, testing completeness, performance bottlenecks, and structural redundancy.

## 1. Compilation and Warning Audits
- **Zero Warnings Policy**: Ensure that `cargo check` and `cargo test` run with absolutely zero compiler warnings or errors.
- **No Warning Suppression**: Search for and prohibit any attributes or annotations that silence compiler warnings (e.g., `#[allow(dead_code)]`, `#[allow(unused_variables)]`, `#[allow(unused_imports)]`, `#[allow(unused_mut)]`). All unused code or variables must be refactored or deleted rather than silenced.
- **Clippy Audits**: Run `cargo clippy --all-targets -- -D warnings` to verify the codebase adheres to Rust idioms and style conventions.

## 2. Unit Testing Completeness
- **Feature Coverage**: For every feature, module, or helper function modified or added, verify there is a corresponding, robust unit test.
- **Boundary Checks**: Ensure unit tests cover edge cases (e.g., empty inputs, overflow states, invalid chess FENs, pinned pieces, double checks).

## 3. Performance & Allocation Audits (Hot Paths)
Chess engines are extremely performance-critical. The search and evaluation paths run millions of times per second.
- **Zero Heap Allocations in Hot Paths**: Ensure that functions in the search, evaluation (`calc_eval`), and move generator modules do NOT trigger heap allocations. Avoid using `String`, `Vec`, `Box`, or collection initializations during search.
- **Cloning Audit**: Audit all uses of `.clone()` or `.to_owned()`. Ensure they do not occur in hot recursive paths (such as cloning `Config` or search states).
- **Copy/Clone Trait Checks**: Ensure large structures are not passing by value or unnecessarily implementing `Copy` when pass-by-reference (`&`) is more efficient.
- **Profiling & Benchmarking**: Use cargo benchmarks or simple search runs (`go depth 10` on startpos FEN) to verify NPS (Nodes Per Second) changes before proposing edits.

## 4. Code Redundancy & Cleanliness
- **Dead/Unused Code**: Identify functions, imports, or structures that are defined but never used.
- **Redundant Logic**: Check for duplicate evaluations, redundant condition checks (e.g. double checking if a move is legal when it was already validated), or overlapping helper modules.

## 5. Non-Destructive Analysis Policy
- **No Code Changes During Audit**: The audit phase must be purely investigatory. Do NOT edit code files while analyzing them. Only generate detailed reports.
- **Logic Preservation**: Any refactoring proposed as a result of the audit must preserve behavioral logic 100% identically. It must be a pure refactoring with zero functional side-effects (no changes to the search tree, evaluation scores, or playing strength) unless explicitly requested.
