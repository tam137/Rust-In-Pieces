# SupraH (Rust Chess Engine)

SupraH is a highly optimized, high-performance chess engine written in **Rust** (Edition 2021). It features a micro-optimized search tree, highly selective pruning, state-of-the-art hand-crafted evaluation (HCE), and full multi-platform capability (Ubuntu, Windows, and native ARM64 compilation).

SupraH is engineered for maximum computational speed, zero-allocation safety during search paths, and robust tactical accuracy, stabilizing at its peak playing strength of **2260 ELO** on the Louguet Chess Test II (LCT II) benchmark.

---

## 🚀 Key Features & Architecture

SupraH utilizes a state-of-the-art **Minimax Search with Alpha-Beta Pruning** and highly selective pruning algorithms to search millions of positions efficiently. Below is a comprehensive reference of all search features and move-sorting heuristics implemented in the engine, complete with technical definitions and direct links to the English-language [Chess Programming Wiki (CPW)](https://www.chessprogramming.org).

### 1. Core Search & Selective Pruning

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

### 2. Move Ordering Heuristics

Optimal move ordering is crucial for triggering Alpha-Beta cutoffs as early as possible. SupraH achieves highly efficient sorting using these combined techniques:

| Heuristic | Technical Description | Wiki Reference |
| :--- | :--- | :--- |
| **Transposition Table (TT)** | A 100% lock-free table using Zobrist hashing and a double-check portable load/store mechanism. Instantly stores and retrieves exact, lower-bound, and upper-bound search evaluations to reuse search results and sort the best move at the absolute top. | [Transposition Table](https://www.chessprogramming.org/Transposition_Table) <br> [Zobrist Hashing](https://www.chessprogramming.org/Zobrist_Hashing) |
| **Killer Moves** | Tracks the two most recent quiet moves that caused a beta cutoff at each ply, prioritizing them immediately after captures. | [Killer Move](https://www.chessprogramming.org/Killer_Move) |
| **Countermove Heuristic** | Stores and ranks the best quiet response move that previously refuted the opponent's previous quiet move, providing context-aware sorting. | [Countermove Heuristic](https://www.chessprogramming.org/Countermove_Heuristic) |
| **History Heuristic & Aging** | Dynamically increments a weight table for quiet moves causing beta cutoffs (scaled by `depth * depth`), with built-in aging processes to keep sorting highly responsive to recent positions. | [History Heuristic](https://www.chessprogramming.org/History_Heuristic) |
| **Mate Distance Pruning** | Bounds alpha-beta thresholds based on the maximum possible distance to a checkmate, avoiding redundant calculations when a quicker mate has already been discovered. | [Mate Distance Pruning](https://www.chessprogramming.org/Mate_Distance_Pruning) |

### 3. Hand-Crafted Evaluation (HCE)
*   **Passed Pawn Dominance**: Detailed endgame bonuses for passed pawn advancement, protected passed pawn coordination, and rooks standing directly behind passed pawns.
*   **King Safety**: King pawn shields, piece shields, and King Ring Attack evaluations to reward/penalize coordinate king assaults.
*   **Tactical Mobility**: Real-time evaluation of sliding and jumping piece mobilities.

---

## ⚙️ UCI Protocol & Command Specification

SupraH fully adheres to the standard Universal Chess Interface (UCI) protocol, enabling seamless integration into GUIs like Arena, Cute Chess, or Banksia.

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
| **`setoption`** | `name Threads value <n>` | Configure option variables. *(Note: Threads config is supported and prints single-threaded capability warnings)*. | `setoption name Threads value 1` |
| **`test`** | None | Triggers internal diagnostic checks, speed performance tests, and timing benchmarks. | `test` |

---

## 🛠️ Build & Compilation Instructions

### 1. Standard Production Build
To compile the optimized production release binary locally:
```bash
cargo build --release
```
The resulting binary will be located in `target/release/suprah`.

### 2. Automated Release Pipeline
To bump versions, run all unit tests, update `CHANGELOG.md`, compile production binaries, and deploy locally and remotely to the ARM matchup server, run:
```bash
./build_and_release.sh "Release changelog entry"
```

### 3. Cross-Compiling for Windows (from Linux)
```bash
cargo build --target x86_64-pc-windows-gnu --release
```

### 4. Cross-Compiling for ARM64 (from Linux)
1. Add target and toolchain linker:
```bash
rustup target add aarch64-unknown-linux-gnu
sudo apt install gcc-aarch64-linux-gnu
```
2. Configure `.cargo/config.toml`:
```toml
[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"
```
3. Compile with standard target argument:
```bash
cargo build --target=aarch64-unknown-linux-gnu --release
```