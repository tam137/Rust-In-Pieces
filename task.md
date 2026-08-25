# Suprah Engine Strength Enhancement Roadmap (`task.md`)

This document defines the technical roadmap and architectural specifications for the three highest-impact performance and playing-strength enhancements for **Suprah**.

---

## 🎯 Executive Summary & Elo Projection

| Milestone | Core Domain | Architectural Focus | Expected NPS Impact | Projected Elo Gain |
| :--- | :--- | :--- | :--- | :--- |
| **Milestone 1** | **Move Generation & Picker** | Pseudo-legal Movegen, Staged `MovePicker`, Zero-allocation Board state | **+300% to +600% NPS** | **+150 to +250 Elo** |
| **Milestone 2** | **Search Architecture** | Negamax Refactor, Check Extensions, Singular Extensions, LMP, QSearch TT | **-40% Branching Factor** | **+120 to +200 Elo** |
| **Milestone 3** | **Neural Evaluation (NNUE)** | Incremental Accumulator Stack, AVX2/NEON SIMD Vectorization | **+2000% NNUE Eval Speed** | **+200 to +350 Elo** |
| **Total** | **Combined Engine Upgrade** | **Full System Modernization** | **Multi-tier Scaling** | **+470 to +800 Elo** |

---

## 🏗️ Milestone 1: Move Generation & Staged Move Picker Architecture

### 1.1 Architectural Problem Analysis
* **Full Pre-validation Overhead**: In `src/move_gen_service.rs`, `generate_valid_moves_list` generates all pseudo-legal moves and immediately executes `board.do_move()` + `board.undo_move()` + dual `get_attackers_mask()` checks on every candidate move to confirm absolute legality and check flags before searching.
* **Wasted Computation on Cut Nodes**: Over 85–90% of Alpha-Beta search nodes produce an immediate beta-cutoff on the first candidate move (TT Best Move or Killer Move). Pre-validating 30+ moves per node wastes >80% of CPU search time.
* **Heap Contention in Search Inner Loop**: `Board` maintains `move_repetition_map: HashMap<u64, i32>`, causing heap lookups/insertions on every `do_move` and `undo_move`. Additionally, `pv_nodes` mutex locks are acquired during move generation.

### 1.2 Target Architecture & Specifications

#### 1.2.1 Pure Pseudo-Legal Move Generation
* Refactor `MoveGenService` to produce strictly **pseudo-legal moves** directly from bitboards without simulating `do_move`/`undo_move` or running check validations during generation.
* Legality verification is deferred until a move is actually selected in search. In `do_move`, verify if the moving side's king is left in check; if illegal, reject and proceed to the next move.

#### 1.2.2 Staged `MovePicker` State Machine
Implement a lazy `MovePicker` struct that yields moves on demand one-by-one:
```
 Stage 0: TT Hash Move (from Transposition Table)
    ↓ (if no cutoff)
 Stage 1: Good Captures & Queen Promotions (Generated on-the-fly, ordered by MVV-LVA / SEE >= 0)
    ↓ (if no cutoff)
 Stage 2: Killer Moves (Killer 1, Killer 2) & Counter Move
    ↓ (if no cutoff)
 Stage 3: Quiet Moves (Generated on-the-fly, ordered by History Heuristic)
    ↓ (if no cutoff)
 Stage 4: Bad Captures (Captures with SEE < 0)
```

#### 1.2.3 Zero-Allocation Board & State Tracking
* Replace `HashMap<u64, i32>` in `Board` with a flat, stack-allocated 1D history array `history_hashes: [u64; 256]` indexed by game/search ply.
* Remove `pv_nodes` mutex locking from the move generator inner loop.

### 1.3 Acceptance & TDD Criteria
- `[ ]` **Perft Correctness**: All standard Perft positions (Initial position, Kiwipete, Position 3, 4, 5) match exact node counts at depth 1 to 6.
- `[ ]` **Zero Allocation**: `do_move` and `undo_move` perform zero heap allocations in release builds.
- `[ ]` **NPS Benchmark**: Nodes per second (NPS) increases by at least 3.0x on standard midgame bench positions.

---

## ⚡ Milestone 2: Negamax Search Refactoring & Selective Pruning Extensions

### 2.1 Architectural Problem Analysis
* **Minimax Code Duplication**: `src/search_service.rs` uses an asymmetric `minimax` structure with parallel `if white { ... } else { ... }` blocks across all pruning rules, PVS null-windows, and TT updates. This increases maintenance complexity and risks boundary bugs.
* **Missing Singular Extensions (SE)**: When a TT move is uniquely superior to all alternative moves, the engine does not extend the search depth, risking tactical blindness in sharp forcing sequences.
* **Missing Late Move Pruning (LMP)**: Quiet moves late in the move loop at shallow depths ($d \le 4$) are searched rather than pruned.
* ~~**Missing TT in Quiescence Search (QSearch)**~~ — **RESOLVED in v0.28.1**. `QuiescenceSearch` now probes and stores Transposition Table entries with a collision-safe replacement policy that prevents shallow QSearch entries from evicting deep main-search entries.
* ~~**No Search Extensions of any kind**~~ — **RESOLVED in v0.29.0**, but **Elo-neutral as delivered**. Every recursive call previously descended with `depth - 1`, so forcing check sequences were truncated at the nominal horizon and only partially recovered by the in-check branch of QSearch. Check Extensions now grant `+1` ply on checking moves; the cost of that ply currently cancels its benefit, see specification 2.2.6.

### 2.2 Target Architecture & Specifications

#### 2.2.1 Clean Negamax Formulation
Refactor the search core into canonical Negamax:
$$\text{eval} = -\text{negamax}(\text{board}, -\beta, -\alpha, \text{depth} - 1, \dots)$$
* Symmetric score bounds where $\alpha$ and $\beta$ are always relative to the side to move.
* Unified Transposition Table storage and retrieval using relative perspective values.

#### 2.2.2 Singular Extensions (SE)
* **Trigger Condition**: At non-root PV nodes with depth $\ge 8$, when a TT entry exists with `depth >= search_depth - 3`, `entry_type == LowerBound | Exact`, and a valid TT best move.
* **Verification Search**: Perform a shallow, reduced search ($\text{depth} = (\text{depth} - 1) / 2$) with a singular window:
  $$[\text{tt\_eval} - s, \text{tt\_eval} - s + 1]$$
  where $s = 2 \cdot \text{depth}$. Exclude the TT move from this verification search.
* **Action**: If no other move meets the threshold (fail-low), extend the TT move search depth by $+1$ ply.

#### 2.2.3 Late Move Pruning (LMP)
* At low depths ($1 \le \text{depth} \le 4$), when not in check and after searching $N$ quiet moves, prune all subsequent quiet moves:
  $$\text{QuietMoveCountThreshold}(\text{depth}) = 3 + 2 \cdot \text{depth}^2$$

#### 2.2.4 TT Probing & Storage in Quiescence Search — ✅ Implemented (v0.28.1)
* Probe the Transposition Table at the start of QSearch. If a valid entry with `depth >= 0` meets the cutoff criteria, return immediately.
* Store exact/bound evaluations upon QSearch completion.

#### 2.2.5 Check Extensions — ✅ Implemented (v0.29.0)
* **Trigger Condition**: In the `minimax` move loop, when the selected move gives check and the current `ply` is below `check_extension_max_ply`.
* **Action**: The child is searched at `depth - 1 + 1`, keeping the remaining depth constant along the forcing line so that the tactical sequence is resolved rather than truncated.
* **Interaction with LMR**: No interaction — the LMR stage already excludes checking moves, so the extension applies exclusively to the PVS/full-depth path.
* **Termination Guarantee**: A constant remaining depth breaks the implicit `ply + depth == root_depth` invariant the search relied upon. Termination is therefore enforced structurally by a hard `MAX_PLY` ceiling at node entry, which returns a static evaluation instead of recursing further.
* **Configuration**: `enable_check_extension: bool` and `check_extension_max_ply: i32`, both exposed via UCI (`EnableCheckExtension`, `CheckExtensionMaxPly`) for SPSA tuning. Setting the ply bound to `0` neutralises the feature without touching the enable flag.

#### 2.2.6 Check Extension Cost Control — ⚠️ Open (measured Elo-neutral in v0.29.0)

A controlled A/B of v0.29.0 against itself (identical binary, feature toggled) measured the
extension at **-4.9 Elo over 1000 games, 95% CI [-27, +17]** at 5s+100ms. The feature is
correct and its benefit is real — it solves more LCT II positions at every fixed depth tested
(21/105 versus 13/105) and resolves Philidor's Legacy a nominal ply earlier — but the tree it
costs cancels the gain exactly:

| Measurement | Result |
| :--- | :--- |
| Nodes at fixed depth, 35-position LCT II suite | 1.22x (d9), 1.52x (d10), 1.87x (d11) |
| Cumulative nodes to depth 12 | 1.54x on v0.29.0, 1.56x on v0.30.0 |
| Depth reached at 1s per move | **-1.14 ply** (26 of 35 positions shallower, 5 deeper) |
| LCT II accuracy at 1s per move | 10/35 with and without — unchanged |
| Extensions granted to checks with $SEE < 0$ | **59%** |
| In-check share of interior nodes | 17.8% → 38.9% |

The mechanism is that an extension spends its extra ply at precisely the node class where
every pruning stage is disabled: Null Move Pruning, Reverse Futility Pruning and Futility
Pruning are all guarded by `!turn.gives_check`, and LMR never reduces a checking move.

> [!WARNING]
> **Do not gate the extension on `see_ge(mv, 0)`.** It was built and measured: it recovers
> almost the whole cost (1.54x → 1.07x nodes to depth 12) but makes the engine fail its own
> smothered-mate test, because the key move `3.Qg8+` of Philidor's Legacy is a queen sacrifice
> with strongly negative SEE. The axis that needs bounding is the *number* of extensions along
> a line, not the material balance of the checking move.

* **Tasks**:
    - `[ ]` **Per-Path Extension Budget**: Thread an `extensions_used: i32` counter through
      `minimax` and refuse further extensions once it exceeds a configurable fraction of the
      root depth. Add `check_extension_budget_divisor: i32` to `Config` and expose it via UCI.
      This converts the compounding cost multiplier into a bounded constant, which is what
      `check_extension_max_ply` was intended to do.
    - `[ ]` **Retire or Repurpose `check_extension_max_ply`**: Instrumentation shows the bound
      is never reached — across 502,045 extensions at depth 10 not one was granted at a ply
      anywhere near the default of 64, and the deepest ply observed was 29. As an SPSA axis it
      is dead: only `0` (off) and "unbounded" are reachable. Replace it with the budget above.
    - `[ ]` **One-Reply Extension**: Extend when the child node has exactly one legal move.
      The move count is already known after generation, so it is free; it fires on a small
      fraction of nodes; and it preserves the sacrificial forcing lines a material filter
      discards (`Qg8+ Rxg8` is a single reply). Evaluate as a replacement for, or a companion
      to, the blanket check extension.
    - `[ ]` **Restrict Extensions to Early Moves**: 9.5% of extensions are currently granted to
      moves ordered fifth or later. Require `turn_counter <= check_extension_max_move_rank`.
    - `[ ]` **Extend at the Root**: `get_moves` searches every root move at `depth - 1`
      unconditionally, so a checking move is treated differently at the root than at every
      other ply. The measurable consequence is one ply of mate-finding: the seven-ply smothered
      mate contains three checks after the root move and would resolve at nominal depth 4, but
      needs depth 5. Beyond the lost ply this leaves the root scoring forcing moves on a
      shallower tree than the interior does, which is a standing source of score instability
      between iterations.

### 2.3 Acceptance & TDD Criteria
- `[ ]` **Negamax Symmetry**: Search results and evaluations are strictly symmetric between White and Black across mirrored positions.
- `[ ]` **Singular Extension Trigger**: Tactical test suite confirms $+1$ ply extension on forced tactical moves.
- `[x]` **QSearch Node Reduction**: QSearch node count decreases with Transposition Table caching enabled — covered by `test_qs_tt_search_consistency_and_node_reduction`.
- `[x]` **Check Extension Horizon Resolution**: A forced mate lying exactly one ply beyond the nominal horizon is found with Check Extensions enabled and missed with them disabled — covered by `test_check_extension_resolves_forcing_mate_beyond_horizon` (Philidor's Legacy at depth 5).
- `[x]` **Extension Termination**: An unbounded Check Extension budget still terminates within the `MAX_PLY` ceiling — covered by `test_unbounded_check_extension_terminates_within_ply_ceiling`.
- `[ ]` **Extension Cost Ceiling**: With the per-path extension budget in place, the cumulative node count to a fixed depth 12 stays within 1.15x of an extension-free search on the LCT II suite, while the forced-mate resolution tests continue to pass.

---

## 🧠 Milestone 3: NNUE Incremental Accumulator Pipeline & SIMD Vectorization

### 3.1 Architectural Problem Analysis
* **Full Recomputation Bottleneck**: In `src/nnue_service.rs`, `NNUEService::evaluate` calls `compute_accumulator`, looping over all 64 squares and 12 bitboards on **every single leaf evaluation**.
* **Scalar Inference**: The forward pass and SCReLU activations are computed in scalar integer loops without SIMD hardware vectorization (AVX2, SSE4.1, or ARM NEON).
* **Default Disabled**: Because NNUE full recomputation is computationally expensive, `Config::use_nnue` is disabled by default, leaving the engine on handcrafted evaluation (HCE).

### 3.2 Target Architecture & Specifications

#### 3.2.1 Incremental Accumulator Stack (`AccumulatorStack`)
Maintain a persistent accumulator stack along the search path:
```rust
pub struct Accumulator {
    pub white: [i16; 256],
    pub black: [i16; 256],
    pub computed: bool,
}
```
* **Normal Move**: Accumulator delta update:
  $$\text{Acc}_{\text{new}} = \text{Acc}_{\text{old}} - \mathbf{W}[\text{piece}, \text{from}] + \mathbf{W}[\text{piece}, \text{to}]$$
* **Capture Move**:
  $$\text{Acc}_{\text{new}} = \text{Acc}_{\text{old}} - \mathbf{W}[\text{piece}, \text{from}] + \mathbf{W}[\text{piece}, \text{to}] - \mathbf{W}[\text{captured\_piece}, \text{to}]$$
* **King Moves**: When a king changes buckets, trigger a full perspective refresh; otherwise maintain incremental updates.

#### 3.2.2 SIMD Vectorization (AVX2 / SSE4.1 / NEON)
* Implement vector intrinsics for feature addition/subtraction (`_mm256_add_epi16`, `_mm256_sub_epi16`).
* Implement vectorized SCReLU squared clipped activation and output dot-product:
  $$\text{output} = \sum \text{clamp}(x, 0, 255)^2 \cdot w_i$$
  using `_mm256_madd_epi16` and horizontal sum accumulators.

#### 3.2.3 Default NNUE Engine Configuration
* Integrate `use_nnue: true` as the primary evaluation pipeline in `src/config.rs`.
* Provide seamless fallback to HCE if model file loading fails gracefully.

### 3.3 Acceptance & TDD Criteria
- `[ ]` **Bit-for-Bit Accumulator Equivalence**: Invariant test confirming incrementally updated accumulators match fully recomputed accumulators on all legal move sequences.
- `[ ]` **Evaluation Throughput**: NNUE leaf evaluation throughput reaches $> 10,\!000,\!000$ evaluations/second on modern x86_64 / ARM64 processors.
- `[ ]` **UCI Stability**: `quantised.bin` network loads reliably at startup without search latency spikes.

---

## 📋 Implementation Checklist

### Phase 1: Move Generation & Move Picker
- [ ] Implement `src/move_gen_service.rs` pseudo-legal move generation for all piece types.
- [ ] Implement `MovePicker` with staged state machine in `src/move_gen_service.rs` and `src/search_service.rs`.
- [ ] Refactor `Board` struct in `src/model.rs` to eliminate `HashMap` heap allocations.
- [ ] Validate with full Perft suite.

### Phase 2: Search Architecture Refactoring
- [ ] Convert `SearchService::minimax` to unified `negamax` in `src/search_service.rs`.
- [x] Implement Transposition Table probing and storage in Quiescence Search. *(v0.28.1)*
- [x] Implement Check Extensions with a hard `MAX_PLY` termination ceiling. *(v0.29.0)*
- [ ] Implement Late Move Pruning (LMP) at depths $1 \le d \le 4$.
- [ ] Implement Singular Extensions (SE) at depths $d \ge 8$.
- [ ] Add configurable parameters to `Config` in `src/config.rs`.

### Phase 3: NNUE Incremental & SIMD Pipeline
- [ ] Add `AccumulatorStack` to `Board` and update in `do_move`/`undo_move`.
- [ ] Implement AVX2 / NEON vector intrinsics for accumulator updates and SCReLU forward pass.
- [ ] Validate incremental accumulator against full recomputation.
- [ ] Set `use_nnue = true` as default in `src/config.rs` and `src/threads.rs`.
