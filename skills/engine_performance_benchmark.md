# Engine Performance Benchmark Procedure

## Purpose
This skill defines the standard procedure for comparing the true search performance (Nodes Per Second - NPS) between two or more engine versions. 

## Background
Relying solely on "Perft" (raw move generation) benchmarks is often misleading. Perft only tests pseudo-legal or legal move generation without executing the actual Alpha-Beta search tree, Transposition Table (TT) lookups, Null-Move-Pruning, or static evaluation. Therefore, architectural improvements to the search tree (such as avoiding hash recalculations) will not be visible in Perft, and might even show a false negative if the move generator was slightly slowed down to benefit the search.

To get reliable real-world performance metrics, we must benchmark the **Alpha-Beta Search** on a complex middlegame position (e.g., Kiwipete) and measure the NPS reported by the engine's UCI `info` output.

## Procedure

When instructed to compare the performance of multiple engine versions (git tags or branches), follow these steps:

### 1. Using the Utility Script
We now maintain a permanent, dedicated utility script in the repository for this exact purpose: `scripts/benchmark_nps.py`.
This script automatically performs git checkouts, builds the engine in release mode, and parses the correct Alpha-Beta NPS output over the Kiwipete FEN.

To benchmark, simply run the script and pass the git tags or branches as arguments:
```bash
./scripts/benchmark_nps.py v0.22.9 v0.22.10 v0.22.11
```

If you are writing a script yourself because the utility script is unavailable, here is what the script does:
1. Iterates over the given list of git tags/branches.
2. Checks out the target version (`git checkout <tag> -q`).
3. Compiles the engine in release mode (`cargo build --release -q`).
4. Executes the engine binary using a piped shell command to feed UCI commands (e.g. `setoption name OwnBook value false`, `position fen r3k2r...`, `go depth 8`).
5. Extracts the maximum NPS reached at a specific depth (e.g., `depth 7`).

### 3. Key Considerations
* **Disable OwnBook:** Always send `setoption name OwnBook value false`. If the engine has a book hit, it will return immediately without searching, breaking the benchmark.
* **Target Depth vs Target Time:** `go depth X` is safer than `go movetime X` for deterministic node pathing, but make sure the `sleep Y` command is long enough to cover the search. If depth 8 is too slow, extract NPS from `info depth 7`.
* **Subprocess stdin Piping:** Engines using `std::io::stdin().lines()` will terminate instantly if stdin is closed early. The piped `sleep` ensures the `stdin` remains open while the search background thread executes.
* **Report the Results:** Calculate the percentage increase/decrease relative to the baseline version and report it clearly to the user. Document the true Alpha-Beta NPS, NOT the Perft NPS, in the `CHANGELOG.md`.
