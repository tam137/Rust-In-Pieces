# Engine Performance Benchmark Procedure

## Purpose
This skill defines the standard procedure for comparing the true search performance (Nodes Per Second - NPS) between two or more engine versions. 

## Background
Relying solely on "Perft" (raw move generation) benchmarks is often misleading. Perft only tests pseudo-legal or legal move generation without executing the actual Alpha-Beta search tree, Transposition Table (TT) lookups, Null-Move-Pruning, or static evaluation. Therefore, architectural improvements to the search tree (such as avoiding hash recalculations) will not be visible in Perft, and might even show a false negative if the move generator was slightly slowed down to benefit the search.

To get reliable real-world performance metrics, we must benchmark the **Alpha-Beta Search** on a complex middlegame position (e.g., Kiwipete) and measure the NPS reported by the engine's UCI `info` output.

## Procedure

When instructed to compare the performance of multiple engine versions (git tags or branches), follow these steps:

### 1. The Benchmark Script
Create a Python script (e.g., `parse_nps.py`) in the repository root that automates the checkout, build, and measurement process for all requested versions. 

The script must:
1. Iterate over the given list of git tags/branches.
2. Check out the target version (`git checkout <tag> -q`).
3. Compile the engine in release mode (`cargo build --release -q`).
4. Execute the engine binary using a piped shell command to feed UCI commands.
5. Extract the maximum NPS reached at a specific depth (e.g., `depth 7`).

### 2. Python Script Template
```python
import subprocess
import re

# Add the versions to compare
tags = ["v0.22.9", "v0.22.10", "v0.22.11"]

for tag in tags:
    # 1. Checkout and Build
    subprocess.run(["git", "checkout", tag, "-q"], check=True)
    subprocess.run(["cargo", "build", "--release", "-q"], check=True)
    
    # 2. Piped UCI Commands
    # We turn off the opening book to enforce a real search!
    # Kiwipete FEN is highly recommended for complex middlegame branching.
    # Sleep is used to give the engine enough time to process and output the info strings before closing stdin.
    cmd = '''(
    echo "uci"
    echo "setoption name OwnBook value false"
    echo "isready"
    echo "position fen r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1"
    echo "go depth 8"
    sleep 3
    echo "quit"
    ) | ./target/release/suprah'''
    
    p = subprocess.run(cmd, shell=True, capture_output=True, text=True)
    
    # 3. Parse highest NPS from target depth
    max_nps = 0
    for line in p.stdout.split("\\n"):
        if "info depth 7" in line and "nps" in line:
            parts = line.split()
            if "nps" in parts:
                idx = parts.index("nps")
                nps = int(parts[idx+1])
                if nps > max_nps:
                    max_nps = nps
                
    print(f"{tag}: {max_nps} NPS")

# Cleanup: Return to master
subprocess.run(["git", "checkout", "master", "-q"])
```

### 3. Key Considerations
* **Disable OwnBook:** Always send `setoption name OwnBook value false`. If the engine has a book hit, it will return immediately without searching, breaking the benchmark.
* **Target Depth vs Target Time:** `go depth X` is safer than `go movetime X` for deterministic node pathing, but make sure the `sleep Y` command is long enough to cover the search. If depth 8 is too slow, extract NPS from `info depth 7`.
* **Subprocess stdin Piping:** Engines using `std::io::stdin().lines()` will terminate instantly if stdin is closed early. The piped `sleep` ensures the `stdin` remains open while the search background thread executes.
* **Report the Results:** Calculate the percentage increase/decrease relative to the baseline version and report it clearly to the user. Document the true Alpha-Beta NPS, NOT the Perft NPS, in the `CHANGELOG.md`.
