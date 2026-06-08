---
name: Task Management Procedure
description: Standard Operating Procedure for organizing, tracking, and prioritizing development tasks in Suprah.
---

# Suprah Task Management Procedure

This document outlines the mandatory procedure for managing, prioritizing, and executing development tasks in the Suprah repository.

## 1. Task File Organization
All development tasks are organized under the `task/` directory in the root of the workspace. Tasks are separated into three files based on their functional domain:
*   **[task/search_task.md](file:///home/tam137/git/suprah/task/search_task.md)**: Search heuristics, alpha-beta pruning, LMR logic, and minimax improvements.
*   **[task/eval_task.md](file:///home/tam137/git/suprah/task/eval_task.md)**: Hand-crafted evaluation (HCE) heuristics, material/positional weights, and endgame scaling.
*   **[task/misc_task.md](file:///home/tam137/git/suprah/task/misc_task.md)**: Book expansions, UCI settings, convenience features, and helper utilities.

---

## 2. Documenting Tasks
When a new task is requested or brainstormed:
*   Identify the correct file under `task/` based on the category.
*   Document the task description and specific checkboxes (`- [ ]`).
*   Include metadata flags for impact and complexity directly in the file:
    ```markdown
    * **Metadata**: `[Impact: High/Medium/Low]` `[Complexity: Low/Medium/High]`
    ```

---

## 3. Working with Tasks & Implementation Rules
Whenever implementing tasks or answering queries, the AI MUST adhere to these rules:

1. **Delete Completed Tasks:**
   Once a task is fully implemented and released, it MUST be **completely deleted** from the active task files to keep the backlog pristine and uncluttered.
2. **Prioritization and Analysis:**
   When the user asks which tasks should be implemented next, the AI MUST first perform a thorough analysis of the source code (`src/`) to assess feasibility and then rank the options based on **Impact** and **Complexity**.
3. **One Task File Focus per Release:**
   During a release cycle, only tasks from **exactly one** task file (either `search_task.md`, `eval_task.md`, or `misc_task.md`) may be implemented. Mixing search, evaluation, and miscellaneous changes in a single release is strictly forbidden to isolate regression causes.
