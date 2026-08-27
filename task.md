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

#### 1.2.1 Move Generation Without `do_move` — ✅ Implemented (v0.32.0)
`MoveGenService` no longer plays a move in order to learn anything about it. It still emits a
fully **legal** move list, but derives legality and `gives_check` from bitboard masks computed
once per node. The original wording of this specification called for pseudo-legal generation with
legality deferred into the search; the two checkboxes below, which describe a pin mask and a
check mask evaluated at generation time, are what was built. Deferral would have removed the same
27.6% while additionally moving mate and stalemate detection into `search_service.rs`, and it
would have destroyed the acceptance test that made this change verifiable — see 1.3.

* **Worth up to 27.6% of runtime** (see 1.4), and it is what makes 1.2.2 possible at all: a lazy
  picker cannot be lazy while generation insists on playing every move it produces. A staged
  picker sits on top of this unchanged: each stage masks its target set with the same
  `NodeMasks`.
* The work was not the deferral itself but the two things `do_move` was used to discover:
    - `[x]` **Legality without playing the move.** `NodeMasks` carries an absolute-pin mask and a
      check mask, both built from a shared `blockers_toward` sniper walk over the `BETWEEN` ray
      table. A non-king, non-pinned, non-en-passant move is legal by construction. King moves are
      tested against the enemy attack set with the king lifted out of the occupancy — without
      that lift a king would shield the square it is retreating to from the slider checking it.
      En passant bypasses the masks entirely and gets an exact test on the occupancy it would
      produce, because it vacates two squares at once and can be the very capture that answers a
      check.
    - `[x]` **`gives_check` without playing the move.** A per-node `check_squares[6]` table gives
      the direct check by table lookup. Discovered check is `discovery & from` combined with an
      alignment test against `LINE[enemy_king][from]`. Castling adds the relocated rook,
      en passant recomputes on the post-move occupancy, and promotions recompute against
      `occupied ^ from` — a pawn promoting on e8 with the enemy king on e1 otherwise blocks its
      own new queen in a table built before the pawn left.
    - `[x]` Verified by a perft walk that checks **every generated move** against the
      `do_move` predicate it replaced, over the six standard positions and the published
      TalkChess special-case suite: 20 positions, ~54 million nodes. See 1.3.
* **Result: 4.80 → 6.46 M nodes/s, +34.8%, on a bit-identical search tree** (14 positions,
  126 iterations, 22,641,886 nodes on both builds). The 1.4 profile predicted 1.38x; the shortfall
  is the cost of the masks themselves.
* `config.skip_strong_validation` and the `force_skip_validation` parameter threaded through
  `minimax` are **removed**. With legality established by construction there is nothing left to
  skip, and the knob was already recorded as broken.

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
Two things to carry into this work:
* `MoveRawList.moves` is `[u8; 256]`, i.e. 128 from/to pairs, and `push` silently drops anything
  beyond that. Since v0.32.0 the buffer only ever receives legal moves, which makes overflow much
  rarer, but the theoretical maximum of 218 legal moves still exceeds it. A staged picker that
  generates per stage removes the problem rather than papering over it; widening the buffer
  instead would cost `MoveRawList::new()` initialisation time on every node.
* 1.4 prices `MoveList::new()` buffer initialisation at 3.1% of runtime, which is pure waste for
  a node that takes a cutoff on its first move.

##### 1.2.2.1 Measured stage sizing (2026-08-25, on v0.32.0)

Before any picker code exists, the question is which stages are worth building: a stage only pays
for itself if cutoffs are actually waiting behind it. `src/search_diag.rs`, behind the
`search-diag` Cargo feature (off by default), counts what the first searched move at every
interior node was and whether it cut. The instrumented build searches a **node-identical** tree to
the default build, verified per position, so the measurement does not perturb what it measures.
Corpus: 14 positions at fixed depth 10, 764,055 interior nodes.

| Measurement | Share of interior nodes |
| :--- | ---: |
| Cutoff on the first searched move | **58.1%** |
| A PV or Transposition Table move was available at all | 24.6% |
| — the search's own TT probe returned a move (the ceiling) | 24.2% |
| — of those, shadowed by `pv_nodes` or never matched | **0.0%** |

The stage breakdown of those first-move cutoffs, by what a `MovePicker` would have had to
generate to produce the cutting move:

| Stage | Cutoffs | of all first-move cutoffs | of interior nodes | cumulative |
| :--- | ---: | ---: | ---: | ---: |
| **0** PV/TT move | 145,885 | 32.9% | 19.1% | 19.1% |
| **1** capture | 191,973 | 43.3% | 25.1% | 44.2% |
| **1b** quiet giving check | 19,714 | 4.4% | 2.6% | 46.8% |
| **2** killer / counter move | 79,280 | 17.9% | 10.4% | **57.2%** |
| **3** ordinary quiet move | 6,878 | 1.6% | 0.9% | 58.1% |

Four conclusions:

* **Stage 0 on its own is worth about 19% of interior nodes, and that is a hard ceiling.** The
  PV/TT move cuts at 77.5% of the nodes where it exists — it is an excellent move when present.
  The limit is presence, not quality, and the 0.0% shadowed figure proves there is no lost
  availability to recover: 24.2% is simply the Transposition Table hit rate on interior nodes.
* **Quiet generation is almost never what finds the cutoff.** Only 0.9% of interior nodes need an
  ordinary quiet move to cut; 57.2% cut on something a picker can produce before generating a
  single quiet move. This is the actual prize in 1.2.2, and it is roughly three times Stage 0.
* **Stage 1b is the structural obstacle, and it is small.** A quiet move that gives check carries
  `give_check_rank_bonus * 10000` = 50,000, which is what lets quiets outrank captures today and
  is therefore why the current order cannot be produced lazily at all. It accounts for **2.6%** of
  interior nodes. Moving that bonus out of the rank function and into stage assignment costs very
  little in cutoff quality and is what unlocks every stage below it.
* **Killer and counter moves are worth a stage of their own at 10.4%**, and they need no
  generation whatsoever — two or three remembered moves validated against `NodeMasks`.

**Consequence for sequencing.** Stage 0 is the only part that is provably order-preserving: the
PV/TT move is ranked at 170,000 to 320,000 while every other move is bounded above by 140,000
(a queen capture at 90,000 plus a check at 50,000), so it always sorts first and searching it
before generating anything leaves the tree bit-identical. Everything from Stage 1 onwards changes
the move order and therefore the tree, and must be validated by matchplay — which `task.md` 2.2.6
shows this setup cannot do below roughly 40 Elo until the opening diversity item is resolved.

Reproduce with `scripts/measure_stage0.py` against a `--features search-diag` build.

##### 1.2.2.2 Stage-0 short-circuit — ⛔ Built, verified, measured negative, reverted

**Status 2026-08-25: implemented and node-identical, but slower than v0.32.0 in every
configuration measured. Reverted from `master`; the full implementation is preserved on the
branch `experiment/stage0-short-circuit` (commit `ca45d7a`) and was never released.**

Stage 0 searches the PV/TT move before generating anything; if it cuts, generation, ranking and
the `MoveList` buffer initialisation are never paid. `minimax` is not special-cased for the first
move: `turns` starts holding only the Stage-0 move and is refilled with the full list when the
loop runs dry, so every pruning gate, `turn_counter`, LMR, PVS and the killer/history/counter
updates run exactly as before.

###### Two findings that invalidate the original reasoning

* **The ordering bound in 1.2.2.1 is wrong.** It claimed every non-PV/TT move is capped at
  140,000 (queen capture 90,000 + check 50,000) against the PV/TT move's 170,000 minimum.
  It omits `add_promotion_moves`, which adds `give_promotion_rank_bonus_queen * 10000` =
  **170,000**. A promoting capture that gives check reaches 310,000 and outranks the PV/TT move,
  and promotions never receive the PV/TT bonus themselves, so the ranges overlap instead of
  nesting. Found by the duplication check, not by reasoning. Worked around in
  `build_stage0_move` by standing down whenever the side to move has a pawn on the pre-promotion
  rank — one bitboard test, deliberately conservative.
* **Deferring generation ranks the remaining moves against a mutated History Heuristic.** The
  first searched move's own subtree updates `history_table`, including the global halving, so the
  refilled list is ranked against a table that has moved on. This is the *sole* remaining source
  of divergence and it is decisive: with an entry-time snapshot the tree is bit-identical on
  14/14 positions, without it 0/14, with searched-node counts differing by up to a factor of
  four. `pv_nodes` is stable during a search; the Transposition Table was tested and pinning it
  changes nothing.

  This constrains the whole of 1.2.2, not just Stage 0: **no lazy or staged move picker in this
  engine can be verified by node identity** unless it ranks against entry-time history.

###### Measured throughput (14 positions, fixed depth 10, best of 3, wall time)

| Configuration | Total ms | vs. v0.32.0 |
| :--- | ---: | ---: |
| `EnableTtMoveFirst=false` (the v0.32.0 search) | 1993 | reference |
| Stage 0 + `Stage0HistorySnapshot=true` (tree bit-identical) | 2055 | **-3.0%** |
| Stage 0 + `Stage0HistorySnapshot=false` (live history, tree differs) | 2537 | **-21.4%** |

The 16 KB snapshot costs about as much as the generation it saves, and letting the history run
live makes the tree enough worse to lose a fifth of the throughput. The +15% projected from
1.2.2.1 did not materialise.

###### Paired A/B (2026-08-25, laptop on battery — absolute times not comparable to the above)

The machine was clocked down, so the wall times above and below are from different machine states
and must not be compared with each other. A *paired* ratio survives that: within one round every
position is measured under both configurations back to back, and the order is flipped each round
so warm-up and drift cancel. 14 positions, depth 10, 5 rounds, minimum per cell.

| Metric | Value |
| :--- | ---: |
| Positions where Stage 0 is faster | **1 / 14** (Sharp French, +1.9%, inside the noise) |
| Median paired ratio | **-9.1%** |
| Aggregate ratio | **-11.8%** |

13 of 14 positions lose. The sign does not depend on the clock, which is what the paired design
buys; the magnitude does, and on battery the spread between repeats is wide enough that only the
sign should be read. **Verdict: Stage 0 as built is a regression and is not shippable.**

###### `OrderingLookups` — measured, and it does not close the gap

The last change made shares the PV-map and Transposition Table lookups between Stage 0 and
generation via `OrderingLookups`, so they are performed once per node instead of twice — Stage 0
was adding a mutex lock and a table probe at every interior node, including the ~75% where it
finds no candidate. **Re-verified 2026-08-25. The double lookup was not the cost.**

- `[x]` `cargo test --release` — 141 passed, 0 failed, 4 ignored.
- `[x]` `scripts/verify_stage0_identity.py` — 14/14 identical, on both `searched` and `qsearch`.
      Sharing the lookups did not break node identity.
- `[x]` `scripts/measure_stage0_throughput.py` — still negative: -8.8% (best of 3), -3.7%
      (best of 5). Unchanged from the -3.0% measured before `OrderingLookups` existed.
- `[x]` It stayed negative, so the Stage-0 call sites were reverted from `src/search_service.rs`,
  `src/move_gen_service.rs`, `src/config.rs`, `src/game_handler.rs` and `src/threads.rs`.
  Verified afterwards against a fresh v0.32.0 build: 14/14 positions identical on iterations,
  scores, principal variations, node counts and `bestmove`.
- `[ ]` Never written: the `is_pseudo_legal` fuzz oracle over all 64x64 from/to pairs against the
  generated list, the rank-equality test, and the negative controls. The duplication check in
  `minimax` (behind `search-diag`, prints `STAGE0MISMATCH`) stood in for them and is what caught
  the promotion defect; it currently reports zero mismatches.

###### Where things are

**On `master`:** the measurement apparatus only — `src/search_diag.rs`, the `search-diag` Cargo
feature and its call sites in `src/search_service.rs`, and `scripts/measure_stage0.py`,
`scripts/verify_stage0_identity.py`, `scripts/measure_stage0_throughput.py`. Also retained, and
currently uncalled: `is_pseudo_legal`, `is_castling_shape`, `build_stage0_move`, `stage0_rank`
and `white_to_move_pawns_on_seventh` in `src/move_gen_service.rs`, all marked `#[allow(dead_code)]`
because a future `MovePicker` needs exactly this pair of primitives.

Two caveats on that retention. `build_stage0_move` and `stage0_rank` **mirror the ranking loop in
`get_valid_moves_from_move_list` and nothing enforces that they stay in step** — the test named in
their doc comment, `stage0_rank_matches_generator_test`, was never written, and the duplication
check that stood in for it lived at the call site that has now been removed. Retune the move
ordering and this copy rots silently. `is_pseudo_legal` is self-contained and carries no such
risk. And `scripts/verify_stage0_identity.py` and `scripts/measure_stage0_throughput.py` both set
`EnableTtMoveFirst`, a UCI option that no longer exists on `master`; they run only against the
branch.

**On `experiment/stage0-short-circuit`:** the complete implementation — the `enable_tt_move_first`
and `stage0_history_snapshot` config knobs and their UCI options, the short-circuit and refill
loop in `minimax`, `OrderingLookups` / `probe_ordering_lookups` /
`generate_valid_moves_list_with_masks`, and the `STAGE0MISMATCH` and `STAGE0BOARDDRIFT`
duplication checks. Not merged, not released.

Stage 0 stood down when `enable_one_reply_extension` was on, because that gate reads `turns.len`.

**Metric trap, still true and still undocumented in the changelog:** the UCI `nodes` field reports
`Stats::created_nodes`, i.e. *generated* moves, so Stage 0 makes `nodes` and `nps` fall while the
engine gets faster. `scripts/benchmark_nps.py` reads that field and will report a false
regression. Throughput must be measured as wall time to a fixed depth, and node identity on
`SEARCHTREE calculated=/eval=` from the `search-diag` build.

#### 1.2.3 Zero-Allocation Board & State Tracking — ✅ Implemented (v0.31.0)
* `Board.move_repetition_map: HashMap<u64, i32>` is replaced by a flat, stack-allocated
  `history_hashes: [u64; MAX_HISTORY_PLIES]` with a `history_len` cursor. `do_move` pushes one
  `u64` and `undo_move` pops it; neither touches the heap any more.
* Threefold repetition is detected by scanning backwards with a stride of two, bounded below by
  `irreversible_floor` — the index of the position produced by the last capture or pawn move.
  Nothing before it can recur, because the Zobrist hash covers material and pawn placement. Note
  the floor is the index *of* that position and not one past it: the position a pawn move
  produces can itself be repeated, and getting this wrong cost 22 nodes out of 17.6 million in
  the first attempt — small enough to look like success and large enough to be a real bug.
* **Result: 3.73 → 4.98 M nodes/s, +33.4%, on a bit-identical search tree** (14 positions,
  104 iterations, 17,662,630 nodes on both builds). Cross-version gauntlet: +8.7 Elo against
  v0.30.3 over 80 games and +43.7 against v0.29.1.

* ~~Remove `pv_nodes` mutex locking from the move generator inner loop.~~ — **measured, and the
  premise is wrong.** The lock is uncontended and the lookup is cheap: disabling the `pv_nodes`
  block entirely changes throughput from 5.26 to 5.13 M nodes/s, i.e. the block is free within
  measurement noise, and the move ordering it provides is worth more than it costs. Not worth
  doing.

### 1.3 Acceptance & TDD Criteria
- `[x]` **Perft Correctness**: `perft_verified` walks the complete move tree and, for every
  generated move, plays it and asserts the two facts the generator now derives instead of
  measuring — that the mover did not leave its own king attacked, and that `gives_check` matches
  whether the enemy king ends up attacked. It runs over the six standard positions and the
  published TalkChess special-case suite (illegal en passant, en passant giving check, castling
  giving check, castling prevented, promotion out of check, promotion giving check, stalemate and
  checkmate): 20 positions, ~54 million nodes, all node counts matching their published values.
  Perft node counts alone are **not** sufficient here, because a wrong `gives_check` does not
  change the number of nodes — it silently changes five pruning decisions instead.
- `[x]` **Node Identity**: v0.32.0 is node-for-node identical to v0.31.0 at fixed depth across
  14 positions and 126 iterations, 22,641,886 nodes on both builds.
- `[x]` **Zero Allocation**: `do_move` and `undo_move` do not touch the heap, and as of v0.32.0
  move generation no longer calls them at all. The remaining per-node allocation-shaped cost is
  the `MoveList::new()` / `MoveRawList::new()` buffer initialisation, which 1.2.2 addresses.
- `[ ]` **NPS Benchmark**: 3.0x is **not** achieved and the projection should be treated as
  unvalidated. Two of three components are now measured: 1.2.3 returned +33.4% and 1.2.1
  returned +34.8%, compounding to **1.80x over v0.30.3**. The remainder rests on 1.2.2 alone,
  and 1.4 caps the realistic joint ceiling at 2x to 2.5x.

### 1.4 Measured Cost Breakdown (2026-08-25, on v0.31.0)

`perf` is unavailable on this host (WSL2 kernel, no matching `linux-tools`), so the profile was
taken by **duplication**: a diagnostic build performs a given piece of work twice and discards
the copy, and the wall-time delta against the normal build is that work's cost. The search tree
stays bit-identical, so there is no measurement bias. Figures are over 14 positions at fixed
depth 10, 17,662,630 nodes, 2.5s baseline.

| Component | Share of total runtime |
| :--- | ---: |
| **Interior move generation, total** | **84.4%** |
| — of which `do_move` + `undo_move` | **27.6%** |
| — of which `MoveList::new()` buffer init | 3.1% |
| — of which the two `get_attackers_mask` calls | ~0% (within noise) |
| — remaining: bitboard generation and move ranking | ~54% |
| Everything else: evaluation, Transposition Table, search logic, Quiescence | ~16% |

Three conclusions, all of which change the roadmap:

* **The attacker masks are free; the validation cost *is* the move making.** `validate_and_add_move`
  plays and unplays every generated move purely to learn legality and `gives_check`. The engine
  therefore calls `do_move`/`undo_move` once per *generated* move — roughly 35 per node — where the
  search only ever plays 2 to 3 of them. That single fact is 27.6% of runtime.
* **Generation and ranking are larger still, at roughly 54%.** This is what a lazy `MovePicker`
  attacks, and it is the bigger half of the prize.
* **The 3.0x acceptance criterion is not reachable from these two items alone.** Even driving
  interior move generation to *zero* caps the speedup at 6.4x, and neither change does that. A
  realistic joint ceiling is 2x to 2.5x; the executive summary's "+300% to +600% NPS" and
  "+150 to +250 Elo" should be revised down accordingly.

`skip_strong_validation = true` is **not** a usable proxy for any of this: it admits illegal moves
into the search and hangs the engine.

**Outcome (v0.32.0).** The 27.6% item was removed in full and returned **+34.8%**, against the
1.38x the profile predicted. The shortfall is the cost of the masks that replaced the move
making: one attacker mask and two sniper walks per node, plus one attacker mask per king move.
`skip_strong_validation` no longer exists — legality is established by construction, so there is
nothing to skip.

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
    - `[x]` **Opening Diversity for Matchplay** — *prerequisite for everything below*. Delivered
      in v0.33.0; see 2.2.7 for what was built and what it measures. Matt-Magie played every game
      from the initial position with no book, so each match sampled whichever openings the clock
      happened to produce, and the three matches above are mutually inconsistent by **41 Elo**:
      `unfiltered - disabled = -10.1`, `deep - disabled = +34.2`, and `deep - unfiltered = +2.8`,
      where transitivity demands the last figure be near +44. Matches are now played from a file
      of book-derived opening lines, one line per colour-swapped game pair.
    - `[ ]` **Re-run the Deep Restriction** against v0.33.0 with 500+ games per pairing on the
      new harness, and set `check_extension_min_depth` from that result. The replay in 2.2.7 is
      too small to decide it, and it is the first thing the harness should be spent on.
    - `[ ]` **SPSA Tuning**: register the five parameters in `tuning/parameters.json` and
      `tuning/groups.json` and tune `check_extension_min_depth` jointly with the LMR and RFP
      depth thresholds it interacts with.
    - `[ ]` **Extend at the Root**: `get_moves` searches every root move at `depth - 1`
      unconditionally, so a checking move is treated differently at the root than at every other
      ply. The measurable consequence is one ply of mate-finding: the seven-ply smothered mate
      contains three checks after the root move and would resolve at nominal depth 4, but needs
      depth 5. Beyond the lost ply this leaves the root scoring forcing moves on a shallower tree
      than the interior does, which is a standing source of score instability between iterations.

#### 2.2.7 Matchplay Harness — ✅ Rebuilt and validated (v0.33.0)

The blocking item above was an opening book. Providing one uncovered that the engine could not
read a book at all: `src/polyglot.rs` generated its 781 Zobrist constants from a linear
congruential generator instead of transcribing the published table, ordered the piece kinds with
White first where the format puts Black first, mirrored the board vertically on top of a mailbox
that already matched PolyGlot's layout, and attached the side-to-move constant to Black where the
specification attaches it to White. Any one of the four makes every key wrong, and the test suite
could not see any of them because it built its test books from the engine's own key. Setting
`BookFile` had therefore always been silently ignored. See the v0.33.0 changelog entry.

**What the harness now is.** `Performance.bin` (92,954 entries) is compiled into the binary;
`scripts/make_opening_lines.py` samples unique fixed-length lines from it by driving the engine
over UCI; Matt-Magie reads them through a new `openings = <file>` key in the `.trn` and plays
**one line per colour-swapped game pair**. Two defects in Matt-Magie itself were fixed at the same
time and both had been present in every measurement in this document: `engine_options` reached
the White side only, so the Black engine ran on default `Hash`, `Threads` and `OwnBook` in every
match ever played, and the first `go` of a game omitted `winc`/`binc`.

`scripts/pairing_elo.py` reports Elo per pairing with two intervals, the unpaired one and the
paired one that treats an opening played with both colour assignments as a single observation.

**Validation, all at 1000ms + 100ms with 98 eight-ply opening lines.**

| Run | Expectation | Measured |
| :--- | :--- | :--- |
| v0.33.0 vs v0.32.0, 200 games | 0 Elo — identical search with `OwnBook=false` | **+8.7**, 95% CI [-26, +44] |
| v0.30.3 vs v0.30.0, 100 games | ~+209, the known fail-soft regression | **+219.9**, 95% CI [+164, +289] |

The null run establishes the figure this document has never had: **the resolution is +/-35 Elo at
200 games**, which scales to +/-16 Elo at 1000. The signal run establishes that the harness is
sensitive enough to see a real regression. Both are also the release test for v0.33.0 itself,
since the null run is what proves the release changed no playing strength.

**The transitivity replay is inconclusive, and says so.** The three check-extension builds were
replayed at 100 games per pairing:

| Comparison | Elo | 95% CI |
| :--- | ---: | :--- |
| deep - unfiltered | +34.9 | +/- 54.0 |
| unfiltered - off | +10.4 | +/- 56.9 |
| deep - off, measured | +3.5 | +/- 49.8 |
| deep - off, implied by transitivity | +45.3 | +/- 78.4 |
| **gap** | **+41.8** | **+/- 92.9, z = 0.88** |

The gap is again roughly 41 Elo, but at 100 games per pairing that is 0.88 standard deviations
and therefore exactly what ordinary sampling noise produces. This run neither reproduces nor
refutes the original inconsistency; it only shows nothing anomalous. A transitivity check needs
roughly 500 games per pairing to have any power, and that run is the same run the open Deep
Restriction item requires — it should be spent there rather than repeated in isolation.

**What is still not established.** That the *nominal* interval is now honest. The original defect
was variance *between* opening pools, which a single match cannot expose whatever its size. The
first genuine test is the next 500-games-per-pairing run: if its pairings are mutually consistent,
the pool variance is gone.

### 2.3 Fail-Soft Alpha-Beta — ⛔ Tried and Reverted (v0.30.0 → v0.30.3)

Converting `minimax` to fail-soft — initialising the running score to `i16::MIN` / `i16::MAX`
instead of to the window bound `alpha` / `beta` — was released in v0.30.0 and **reverted in
v0.30.3**. It is one of the most reliable Elo gains in the literature, and in this engine it
costs roughly two hundred rating points.

| Comparison | Games | Score | Elo |
| :--- | ---: | ---: | ---: |
| v0.29.1 vs. v0.30.0 | 60 | 76.7% | **+206.7** for v0.29.1 |
| v0.29.1 vs. v0.29.1 + fail-soft only | 60 | 72.5% | **+168.4** for v0.29.1 |
| v0.29.1 vs. v0.29.1 + bound classification only | 60 | 55.0% | +34.9, not significant |
| v0.30.3 (revert) vs. v0.30.2 | 80 | 76.9% | **+208.7** for the revert |
| v0.30.3 (revert) vs. v0.29.1 | 80 | 54.4% | +30.5, at parity |
| v0.30.3 (revert) vs. v0.28.4 | 80 | 50.6% | +4.3, at parity |

The bisection isolates the running-score initialisation as the sole cause; the Transposition
Table bound reclassification that shipped in the same commit is innocent and was kept.

#### Mechanism — identified

Nothing deterministic reveals the defect: fixed-depth node counts, scores, best moves and even
the reported principal variations are comparable to v0.29.1; the mean completed depth at one
second per move is identical (12.31 versus 12.40); the engine uses 930ms of a 1000ms budget and
never forfeits; all 134 unit tests pass. The reason is that **every benchmark in this repository
sends `ucinewgame` before each position and therefore searches with an empty Transposition
Table**, while a played game accumulates entries across roughly eighty moves.

Measuring the same build twice over one fixed 60-move sequence at fixed depth — once with a
Transposition Table cleared before every position, once with the table left to accumulate —
isolates it. The figure is the drift between a build's own cold and warm evaluation of the very
same position:

| Build | positions drifting > 50cp | mean drift | max drift |
| :--- | ---: | ---: | ---: |
| fail-hard (v0.29.1) | **0 / 60** | 5.5 | **31** |
| fail-hard, LMR disabled | 0 / 60 | 1.1 | 11 |
| fail-soft (v0.30.0) | 2 / 60 | 16.8 | **522** |
| fail-soft, LMR disabled | 2 / 60 | 5.0 | 120 |
| fail-soft + LMR fail-low clamped to `alpha` | 3 / 60 | 12.4 | 266 |
| **fail-soft + Transposition Table write clamped** | **0 / 60** | **5.0** | **37** |

Fail-hard is stable to within 31 centipawns whatever the table contains. Fail-soft disagrees with
itself by up to **five pawns** purely because the table is warm — and it then writes that
disagreement back, so the contamination compounds across a game. The Transposition Table bound
reclassification that shipped in the same v0.30.0 commit does not help: measured separately it
leaves the drift unchanged at a 522cp maximum.

**The damage channel is the Transposition Table write, and nothing else.** Clamping only the
value that is *written to the table* back into the window that was actually searched — while
still returning the unclamped fail-soft score to the parent — restores fail-hard's stability
completely, at 0/60 positions past 50cp and a 37cp maximum. Fail-soft's out-of-window values are
harmless as return values; they are poison once they enter a table that outlives the move.

Late Move Reductions are **not** the specific culprit, although an earlier reading of the
ablation above suggested they were. Disabling LMR improves *both* builds by a similar factor
(fail-hard 31 → 11, fail-soft 522 → 120), so it is a general amplifier of table sensitivity
rather than a fail-soft-specific trigger; measured as a ratio, fail-soft is 11x to 17x worse than
fail-hard whether LMR is on or off. Clamping only the LMR fail-low result closes about a third of
the gap, which is consistent with LMR being one contributor among several rather than the cause.

**The fix is only partial in Elo terms.** The table-write clamp recovers roughly 133 of the 168
Elo — from -168.4 against v0.29.1 down to -35.3 over 79 games, 95% CI [-97, +24], and -53.2
against v0.30.3, 95% CI [-111, +2]. It closes the measurable contamination channel entirely and
still does not reach parity. The residue is the fail-soft return value itself: it propagates
through pruning stages whose decisions were only ever sound against a clamped score, and it
reshapes the tree its parent searches. On the present evidence fail-soft's *benefit* in this
engine is somewhere between zero and slightly negative even once its poison is removed.

##### Remaining work, should fail-soft be attempted again

The cold-versus-warm drift measurement above is the cheap, deterministic gate: it exposed in
minutes what four 1000-game matches could not. Any retry must keep it at fail-hard levels
(0 positions drifting past 50cp, maximum in the low tens) *before* a single game is played.

* **Tasks**:
    - `[x]` **Mechanism identified** — the Transposition Table write is the sole damage channel.
      Clamping it restores full stability; see `ab-ttclamp` in the table above.
    - `[x]` **Candidate fix built and measured** — `ab-ttclamp` recovers roughly 133 of the 168
      Elo but stops at -35.3 against v0.29.1 (79 games, 95% CI [-97, +24]). **Not shippable as
      it stands**: the point estimate is negative against the release it would replace.
    - `[ ]` **Resolve the residual before considering it again.** Two questions, in order. First,
      is the remaining -35 real? 79 games resolve only about +/-60 Elo, so a run of 500 or more
      against v0.30.3 is needed to tell a genuine small regression from noise. Second, if it is
      real, it can only come from the fail-soft return value reshaping the parent's search, since
      the table now sees exactly what fail-hard would have written — audit whether the pruning
      stages that consume that value (LMR, futility, PVS null-window) remain sound against an
      unclamped score.
    - `[ ]` **Do not pursue "never store deeper than searched" as the fix.** It is a sound
      principle in general, but it is not what the measurement identified here and it was never
      shown to address this defect.
    - `[ ]` **Write the regression test from the drift measurement.** A warm-table consistency
      assertion is the natural form and would have caught this release: the current suite passes
      completely on the broken build.
    - `[ ]` **Gate any retry on the cross-version gauntlet** now mandatory in
      `skills/engine_release_procedure.md`. A self-A/B cannot see this class of defect.

##### Reproduction

The binaries and PGNs behind the table above are kept outside the repository, in
`../matt-magie/engines/` and `../matt-magie/`:

| Artefact | What it is |
| :--- | :--- |
| `ab-bisA` | v0.29.1 plus the fail-soft initialisation only |
| `ab-bisB` | v0.29.1 plus the bound reclassification only |
| `ab-revert` | v0.30.2 with fail-soft reverted, identical to the released v0.30.3 |
| `ab-fsfix` | v0.29.1 plus fail-soft plus the LMR fail-low clamp |
| `ab-ttclamp` | v0.29.1 plus fail-soft plus the Transposition Table write clamp |
| `ab_bisect.pgn` | the three-way bisection gauntlet |
| `ab_revert.pgn` | the revert verification gauntlet |
| `ab_gauntlet.pgn` | v0.30.2 against v0.28.4, v0.29.0, v0.29.1 and v0.30.0 |
| `ab_ttclamp.pgn` | `ab-ttclamp` against v0.29.1 and v0.30.3 |

The drift measurement itself is `drift.py` / `warm_tt.py` in `~/suprah-analysis-2026-08-25/`.

Evaluate them per pairing, never by the scoreboard rating.

### 2.4 Acceptance & TDD Criteria
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
- [x] Generate moves in `src/move_gen_service.rs` without `do_move`/`undo_move`, with legality and `gives_check` derived from per-node masks. *(v0.32.0)*
- [ ] Implement `MovePicker` with staged state machine in `src/move_gen_service.rs` and `src/search_service.rs`.
- [x] Refactor `Board` struct in `src/model.rs` to eliminate `HashMap` heap allocations. *(v0.31.0)*
- [x] Validate with full Perft suite. *(v0.32.0, `perft_verified_deep_sweep_test`)*

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
