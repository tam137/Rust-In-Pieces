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

#### 2.2.6 Check Extension Cost Control — 🔬 Four axes built and measured (v0.30.2)

A controlled A/B of the v0.29.0 feature against itself (identical binary, feature toggled)
measured it at **-4.9 Elo over 1000 games**, 95% CI [-27, +17], at 5s+100ms. The same test
repeated on v0.30.0 gave **-10.1 Elo**, 95% CI [-29, +9]. Two independent 1000-game runs, both
negative: the feature is at best Elo-neutral as delivered.

It is not broken. It resolves Philidor's Legacy a nominal ply earlier than the baseline and
solves more LCT II positions at fixed depth. The problem is exclusively the price of the ply:

| Measurement | Result |
| :--- | :--- |
| Depth reached at 1s per move | **-1.06 to -1.49 ply** |
| Depth reached at 300ms per move | **-0.89 ply** |
| Nodes, Kiwipete at fixed depth 9 | 916,814 → 1,602,137 (**1.75x**) |
| Extensions granted to checks with $SEE < 0$ | **59%** |
| In-check share of interior nodes | 17.8% → 38.9% |

The mechanism is that an extension spends its extra ply at precisely the node class where every
pruning stage is disabled: Null Move Pruning, Reverse Futility Pruning and Futility Pruning are
all guarded by `!turn.gives_check`, and LMR never reduces a checking move.

##### Measured results per axis

All four axes are implemented and exposed via UCI, so any combination is A/B-testable on a
single binary without a rebuild. Node counts are Kiwipete at fixed depth 9; depth figures are
the mean completed iteration over the 35-position LCT II suite at 1s per move.

| Axis | Parameter | Nodes | Depth vs. disabled | Elo vs. disabled |
| :--- | :--- | ---: | ---: | ---: |
| *(disabled)* | `enable_check_extension = false` | 916,814 | +0.00 | reference |
| *(unfiltered)* | shipped in v0.29.0 | 1,602,137 | -1.06 | -4.9 / -10.1 (2x1000) |
| Material filter | `check_extension_require_safe` | 1,388,235 | — | not played |
| Per-path budget | `check_extension_budget_divisor` | 1,509,969 | -0.91 | not played |
| One-Reply Extension | `enable_one_reply_extension` | 1,685,013 | -0.20 | not played |
| Frontier only | `check_extension_max_depth = 2` | 946,608 | -0.23 | **-26.8** (1000) |
| Deep only | `check_extension_min_depth = 4` | 1,321,166 | — | **+34.2** (500) |

* **Material filter — rejected.** Gating on $SEE \ge 0$ recovers much of the cost but makes the
  engine fail its own smothered-mate test: the key move `3.Qg8+` of Philidor's Legacy is a queen
  sacrifice with strongly negative SEE, so the filter deletes exactly the extension that makes
  the mate visible. The axis is kept as a tunable but must not be enabled on its own.
* **Per-path budget — ineffective.** Capping the number of extensions a path may accumulate
  removes only 6% of the tree. The cost is dominated by the *first* extensions on each path,
  which any budget necessarily grants; capping the tail changes almost nothing.
* **One-Reply Extension — cheap but not a substitute.** Extending nodes with a single legal move
  costs almost nothing (-0.20 ply) and preserves forced sacrificial lines that a material filter
  discards, but on its own it does not reproduce the tactical benefit of the check extension.
* **Frontier restriction — harmful.** Granting extensions only at low remaining depth cuts the
  overhead to 3% above a fully disabled search, and the fixed-time depth measurement rated it the
  most promising axis by a wide margin. Matchplay reversed the verdict completely: **-26.8 Elo**
  over 1000 games against a disabled extension, 95% CI [-45, -9]. Near the horizon the Quiescence
  Search already resolves checks, so the extension there is close to pure cost. This is the
  clearest evidence in the whole investigation that depth and test-suite accuracy are not
  adequate proxies for playing strength.
* **Deep restriction — the mirror image, and unresolved.** Granting extensions only at high
  remaining depth measured **+34.2 Elo** over 500 games against a disabled extension, 95% CI
  [+9, +60]. The two restrictions therefore separate a strongly positive component from a
  strongly negative one, which is a coherent explanation for why the unfiltered feature that
  bundles both lands near zero. **But it did not confirm.** Played directly against the release
  it would replace, v0.30.1, the same build scored only **+2.8 Elo** over 500 games, 95% CI
  [-22, +27] — see the measurement caveat below.

* **Tasks**:
    - `[x]` **Frontier Restriction**: `check_extension_max_depth` in `Config`, UCI
      `CheckExtensionMaxDepth`. `0` disables the restriction.
    - `[x]` **Deep Restriction**: `check_extension_min_depth` in `Config`, UCI
      `CheckExtensionMinDepth`. `0` disables the restriction.
    - `[x]` **Per-Path Extension Budget**: `check_extension_budget_divisor` in `Config`, UCI
      `CheckExtensionBudgetDivisor`, capping path extensions at `root_depth / divisor`. Derived
      from `depth + ply - root_depth` without widening the `minimax` signature; `root_depth` is
      carried on `SearchContext`.
    - `[x]` **Material Filter**: `check_extension_require_safe` in `Config`, UCI
      `CheckExtensionRequireSafe`.
    - `[x]` **One-Reply Extension**: `enable_one_reply_extension` in `Config`, UCI
      `EnableOneReplyExtension`. Applied at the node once the move list is known, so the single
      legal move is already counted and the extra ply costs one node.
    - `[ ]` **Opening Diversity for Matchplay** — *prerequisite for everything below*. Matt-Magie
      starts every game from the initial position with no book, so each match samples whichever
      openings the clock happens to produce. The three matches above are mutually inconsistent by
      **41 Elo**: `unfiltered - disabled = -10.1`, `deep - disabled = +34.2`, and
      `deep - unfiltered = +2.8`, where transitivity demands the last figure be near +44. The
      real resolution of this setup is therefore roughly +/-40 Elo, not the +/-25 the intervals
      report, because the interval only models correlation *within* one match and not the
      variance *between* opening pools. Until a start-position book or a seeded FEN set exists,
      no search change smaller than about 40 Elo can be validated here, and no default should be
      changed on matchplay evidence alone.
    - `[ ]` **Re-run the Deep Restriction** against v0.30.1 with 2000-3000 games once opening
      diversity is in place, and set `check_extension_min_depth` from that result.
    - `[ ]` **SPSA Tuning**: register the five parameters in `tuning/parameters.json` and
      `tuning/groups.json` and tune `check_extension_min_depth` jointly with the LMR and RFP
      depth thresholds it interacts with.
    - `[ ]` **Extend at the Root**: `get_moves` searches every root move at `depth - 1`
      unconditionally, so a checking move is treated differently at the root than at every other
      ply. The measurable consequence is one ply of mate-finding: the seven-ply smothered mate
      contains three checks after the root move and would resolve at nominal depth 4, but needs
      depth 5. Beyond the lost ply this leaves the root scoring forcing moves on a shallower tree
      than the interior does, which is a standing source of score instability between iterations.

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
