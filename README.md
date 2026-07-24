# Rust-In-Pieces (Rust Chess Engine)

Rust-In-Pieces is a chess engine written in **Rust** (Edition 2024). It features an optimized search tree, selective pruning, and hand-crafted evaluation (HCE).

Rust-In-Pieces is engineered for computational speed, zero-allocation safety, and tactical strength, stabilizing at around **2000 - 2200 ELO** (on Lichess and the Louguet Chess Test II (LCT II) benchmark). Its primary limitation lies in long-term strategic decision-making, reflecting the author's modest chess knowledge rather than software engineering limits.

---

## Motivation & Background

This project was created as a personal learning experience to deepen practical knowledge in **Rust** (high-performance systems engineering and core chess engine architecture) and **Python** (automated SPSA parameter tuning pipelines, diagnostic tooling, and benchmarking).

While not every subsystem is fully micro-optimized - and starting the project over from scratch today would certainly lead to a few different architectural choices - it incorporates advanced chess programming concepts and achieves high tactical performance.

### Play on Lichess

When the host server is online, the engine is playable live on Lichess:  
[Rust-In-Pieces on Lichess](https://lichess.org/@/rust-in-pieces)

---

## Key Features & Architecture

Rust-In-Pieces utilizes a state-of-the-art **Minimax Search with Alpha-Beta Pruning** and highly selective pruning algorithms to search millions of positions efficiently. Below is a comprehensive reference of all search features and move-sorting heuristics implemented in the engine, complete with technical definitions and direct links to the English-language [Chess Programming Wiki (CPW)](https://www.chessprogramming.org).

### Core Search & Selective Pruning

| Feature | Technical Description | Wiki Reference |
| :--- | :--- | :--- |
| **Alpha-Beta Pruning** | The core recursive search algorithm, pruning branches that are mathematically proven to be worse than previously evaluated moves. | [Alpha-Beta](https://www.chessprogramming.org/Alpha-Beta) |
| **Principal Variation Search (PVS)** | A highly selective search method utilizing zero-width window searches `[alpha, alpha+1]` on non-PV nodes to aggressively prove that sub-trees cannot improve alpha. | [Principal Variation Search](https://www.chessprogramming.org/Principal_Variation_Search) |
| **Late Move Reductions (LMR)** | Reduces quiet moves searched further down the move list in deep sub-trees, dynamically adjusting reductions based on PV-node state, history heuristics, and killer moves. | [Late Move Reductions](https://www.chessprogramming.org/Late_Move_Reductions) |
| **Null Move Pruning (NMP)** | Bypasses standard search branches early by giving the opponent a free double move ("passing the turn"). If the search still yields a beta cutoff, the branch is safely pruned. Integrated with a deep **Verification Search** to avoid Zugzwang blunders. | [Null Move Pruning](https://www.chessprogramming.org/Null_Move_Pruning) |
| **Reverse Futility Pruning (RFP)** | Also known as Static Null Move Pruning; immediately prunes leaf nodes at shallow depths when the static evaluation (minus a depth-scaled margin) is greater than or equal to beta. | [Reverse Futility Pruning](https://www.chessprogramming.org/Reverse_Futility_Pruning) |
| **Aspiration Windows** | Bounds the initial search using a narrow window centered on the previous iteration's score, dynamically widening the window if search scores fail low or high. | [Aspiration Windows](https://www.chessprogramming.org/Aspiration_Windows) |
| **Quiescence Search (Q-Search)** | Extends leaf nodes recursively by searching only captures and promotions until a tactically stable position ("stand-pat") is reached, completely resolving the horizon effect. | [Quiescence Search](https://www.chessprogramming.org/Quiescence_Search) |
| **Static Exchange Evaluation (SEE)** | Evaluates the material balance of capture sequences on a single target square. Used to prune losing quiet captures in Quiescence Search (`SEE < 0`) and demote blunder captures below quiet moves in standard search move ordering. | [Static Exchange Evaluation](https://www.chessprogramming.org/Static_Exchange_Evaluation) |

### Move Ordering Heuristics

Optimal move ordering is crucial for triggering Alpha-Beta cutoffs as early as possible. Rust-In-Pieces achieves highly efficient sorting using these combined techniques:

| Heuristic | Technical Description | Wiki Reference |
| :--- | :--- | :--- |
| **Transposition Table (TT)** | A 100% lock-free table using Zobrist hashing and a double-check portable load/store mechanism. Instantly stores and retrieves exact, lower-bound, and upper-bound search evaluations to reuse search results and sort the best move at the absolute top. | [Transposition Table](https://www.chessprogramming.org/Transposition_Table) <br> [Zobrist Hashing](https://www.chessprogramming.org/Zobrist_Hashing) |
| **Killer Moves** | Tracks the two most recent quiet moves that caused a beta cutoff at each ply, prioritizing them immediately after captures. | [Killer Move](https://www.chessprogramming.org/Killer_Move) |
| **Countermove Heuristic** | Stores and ranks the best quiet response move that previously refuted the opponent's previous quiet move, providing context-aware sorting. | [Countermove Heuristic](https://www.chessprogramming.org/Countermove_Heuristic) |
| **History Heuristic & Aging** | Dynamically increments a weight table for quiet moves causing beta cutoffs (scaled by `depth * depth`), with built-in aging processes to keep sorting highly responsive to recent positions. | [History Heuristic](https://www.chessprogramming.org/History_Heuristic) |
| **Mate Distance Pruning** | Bounds alpha-beta thresholds based on the maximum possible distance to a checkmate, avoiding redundant calculations when a quicker mate has already been discovered. | [Mate Distance Pruning](https://www.chessprogramming.org/Mate_Distance_Pruning) |

### Hand-Crafted Evaluation (HCE)

*   **Passed Pawn Dominance**: Detailed endgame bonuses for passed pawn advancement, protected passed pawn coordination, and rooks standing directly behind passed pawns.
*   **King Safety**: King pawn shields, piece shields, and King Ring Attack evaluations to reward/penalize coordinate king assaults.
*   **Tactical Mobility**: Real-time evaluation of sliding and jumping piece mobilities.

---

## UCI Protocol & Command Specification

Rust-In-Pieces fully adheres to the standard Universal Chess Interface (UCI) protocol, enabling seamless integration into GUIs like Arena, Cute Chess, or Banksia.

### Supported UCI Commands

| Command | Arguments | Description | Example |
| :--- | :--- | :--- | :--- |
| **`uci`** | None | Initializes the engine, returning its name, author, and `uciok` token. | `uci` |
| **`isready`** | None | Pings the engine to verify it is fully loaded, returning `readyok`. | `isready` |
| **`ucinewgame`** | None | Informs the engine that a new game has started; clears search tables and state. | `ucinewgame` |
| **`position`** | `[fen <fen_str> \| startpos] [moves <move_list>]` | Sets the internal chessboard position and optional move list. | `position startpos moves e2e4 e7e5` |
| **`go`** | `[infinite] [wtime <ms> btime <ms> winc <ms> binc <ms> depth <d>]` | Starts calculating. Supports time controls, increments, search depths, or infinite search. | `go wtime 300000 btime 300000` |
| **`stop`** | None | Immediately halts the search thread and returns the best move found. | `stop` |
| **`quit`** | None | Safely terminates the engine execution. | `quit` |
| **`debug`** | `[on \| off]` | Toggles verbose engine logging. Writes log files to `rust-in-piece-<version>.log`. | `debug on` |
| **`setoption`** | `name <Option> value <v>` | Configure option variables (e.g., `BookFile`, `OwnBook`, `Move Overhead`, `Aggressiveness`). *(Note: Threads config is supported but prints single-threaded capability warnings)*. | `setoption name BookFile value /path/to/book.bin` |
| **`test`** | None | Triggers internal diagnostic checks, speed performance tests, and timing benchmarks. | `test` |

### Key UCI Options

| Option Name | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| **`BookFile`** | `string` | `<empty>` | Path to an external **PolyGlot (`.bin`)** opening book. When configured, PolyGlot book moves are prioritized regardless of `OwnBook`. |
| **`OwnBook`** | `check` | `true` | Controls whether the internal hardcoded opening book is used as a fallback when `BookFile` is empty or does not contain a move for the position. |
| **`Move Overhead`** | `spin` | `0` | Buffer in milliseconds subtracted from time controls to compensate for network/GUI latency. |
| **`LogPath`** | `string` | `<empty>` | File path for verbose engine debug logs. |

---

## SPSA Parameter Tuning

Rust-In-Pieces utilizes Simultaneous Perturbation Stochastic Approximation (SPSA) to optimize its hand-crafted evaluation (HCE) parameters. SPSA efficiently computes simultaneous gradient approximations across all active parameters using only two engine variant evaluations per iteration.

### Core Optimization & Mathematical Concepts

| Feature | Technical Description |
| :--- | :--- |
| **Simultaneous Perturbation** | Evaluates gradient vectors across all parameters simultaneously using a random Bernoulli distribution ($\pm 1$), requiring only 2 game-batch evaluations per iteration regardless of parameter count. |
| **Dynamic Scaling & Clamping** | Dynamically scales step sizes proportionally to each parameter's absolute magnitude, enforcing strict `[min, max]` boundary clamping to prevent unstable configurations. |
| **SGDM & EMA Momentum** | Applies Stochastic Gradient Descent with Exponential Moving Average (EMA) momentum tracking ($\beta = 0.9$) to smooth out game-outcome noise and stabilize the optimization trajectory. |

### Infrastructure & Workflow Integration

| Component | Technical Description |
| :--- | :--- |
| **Match Infrastructure & Fairness** | Integrates with the `Matt-Magie` match manager to execute parallel game batches (e.g., 500 games/iteration) with strict alternating color assignments to eliminate White/Black side bias. |
| **State Persistence & Fault Tolerance** | Automatically serializes iteration state, parameter vectors ($\theta$), and momentum ($m$) to `spsa_state.json` and logs historical trajectories in `spsa_history.csv` for seamless pause and resume capability. |
| **Parameter Schema & Scope** | Configures and tunes 75+ active evaluation parameters (piece values, pawn structures, king safety, mobility) defined in `parameters.json` without requiring manual per-parameter tuning loops. |
| **Parameter Harvesting** | Post-tuning workflow extracts optimal converged parameters from `spsa_state.json` and integrates them back into `src/config.rs` for production engine builds. |

### Tuning Invocation Example

```bash
python3 tuning/spsa_tuner.py \
    --engine target/release/rust-in-pieces \
    --mm ../target/release/Matt-Magie \
    --games 500 \
    --workers 4
```

---

## Build & Compilation Instructions

### Standard Production Build
To compile the optimized production release binary locally:
```bash
cargo build --release
```
The resulting binary will be located in `target/release/rust-in-pieces`.

### Automated Release Pipeline
To bump versions, run all unit tests, update `CHANGELOG.md`, and compile production binaries, run:
```bash
./build_and_release.sh "Release changelog entry"
```

### Cross-Compiling for Windows (from Linux)
```bash
cargo build --target x86_64-pc-windows-gnu --release
```

---

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.