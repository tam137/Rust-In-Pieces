# SupraH (Rust Chess Engine)

SupraH is a highly optimized, high-performance chess engine written in **Rust** (Edition 2021). It features a micro-optimized search tree, highly selective pruning, state-of-the-art hand-crafted evaluation (HCE), and full multi-platform capability (Ubuntu, Windows, and native ARM64 compilation).

SupraH is engineered for maximum computational speed, zero-allocation safety during search paths, and robust tactical accuracy, stabilizing at its peak playing strength of **2260 ELO** on the Louguet Chess Test II (LCT II) benchmark.

---

## 🚀 Key Features & Architecture

### 1. Search & Pruning Upgrades
*   **Principal Variation Search (PVS)**: Zero-window search for non-PV nodes to aggressively compress search branches.
*   **Late Move Reductions (LMR)**: Dynamically reduces quiet moves further down the move list in deep sub-trees, fully configurable via LMR parameters.
*   **Null Move Pruning (NMP)**: Bypasses branches early if the position is so strong that passing the turn still yields a beta-cutoff, protected by robust non-pawn material checks to prevent Zugzwang blunders.
*   **Reverse Futility Pruning (RFP)**: Also known as Static Null Move Pruning, immediately crops futile leaf nodes at shallow depths.
*   **Aspiration Windows**: Restricts search bounds at the root, widening bounds progressively if scores fail low or high.

### 2. Move Ordering Heuristics
*   **Transposition Table (TT)**: Zobrist hash-keyed transposition table with smart replacement policies to instantly retrieve exact, lower-bound, and upper-bound scores.
*   **Killer Moves**: Prioritizes successful threat-refuting quiet moves at each ply.
*   **Counter-Moves Heuristic**: Tracks and ranks the best quiet response moves to specific opponent quiet moves.
*   **History Heuristic & Ageing**: Rewards quiet moves that cause beta cutoffs, with built-in aging protection to keep table sorting highly responsive.

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