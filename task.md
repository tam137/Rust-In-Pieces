# Suprah Engine Strength Enhancement Roadmap (`task.md`)

What to build next in **Suprah**, and the record of what has already been tried and failed.
Read "Negative results" before proposing anything: five of the ideas in this document were built,
measured and reversed, and two of them looked excellent on every metric except games won.

---

## 🧭 Start Here

### Where the engine is

| | |
| :--- | :--- |
| Released | **v0.39.0** on `master` (HCE) since 2026-09-02 — the move order as a total order with nesting bands, **+25.6 Elo**. `feature/nnue-evaluation` is not maintained: work is on `master` in HCE mode only, decided 2026-09-02 |
| Throughput | **1.86x** over v0.30.3, from three measured changes on bit-identical search trees |
| Matchplay resolution | **+/-23 Elo at 500 games**, **+/-13 at 3000**, per pairing — measured on host A. On host C with paired openings: **+/-11 at 2000**, **+/-6.5 at 6000**, the last of these confirmed by v0.39.0's run, which returned [+19, +32] around +25.6 |
| Run cost | a **6000-game** fixed-N run is **2.3 s per game** at concurrency 5, i.e. **under 4 hours**. A 200-game smoke gauntlet is 8 minutes. Pricing one change per run is affordable; bundling changes to save a run is not a saving worth having |
| Blocked on | nothing. The next item is the staged `MovePicker` (5), and v0.39.0 has just removed the obstacle that made it unverifiable |
| Runs on | **host C (ARM, 8 cores)** since 2026-08-28 — resolve `<mm>` and rebuild the binaries there; nothing from host A or host B runs or transfers. Concurrency cap here is **5**, from `floor(nproc * 0.75) - 1` |


### What has shipped

See the Engines Changelog if needed.


### The next action — the staged `MovePicker` (5)

**v0.39.0 turned the move order into a total order, and that is what unblocks this item.** Rank
alone was never total: every quiet without a history entry ranks equal, a capture whose attacker
penalty exceeded its victim's value was clamped into the same class, and the selection scans broke
those ties by array position and then `swap`ped the winner into place, permuting the part of the
list they had not examined. The searched order was therefore a function of the swap history rather
than of the position, and **no picker that generates its moves in a different sequence could have
reproduced it.** The generation index now lives in the low `RANK_TIEBREAK_BITS` of the rank, so
the order is a property of the move set alone and a staged picker can be held to it by node
identity — the gate 5.2 demands.

The bands are what paid. Measured **+25.6 Elo** over 6000 games, 95% interval **[+19, +32]**,
against a bound fixed before the run at -5. Deterministically, **21.4% less work to fixed depth 10**
over 300 pool positions. The three inversions it repaired are listed in the changelog; none of
them was a missing heuristic, all three were the rank scale contradicting itself.

Two lessons from the measurement, both of which change how the next one is run:

* **The 14-position corpus cannot rank an ordering change.** Re-permuting a tie class moves single
  positions by factors in *both* directions — one variant read -1.5% on the corpus and +2.7% over
  300 pool positions, and Kiwipete alone swung from 1.40M to 3.83M nodes between two orderings
  worth the same in aggregate. The corpus is sized for node identity, which is binary. Use
  `scripts/measure_tree_size.py` for anything that re-shapes the tree.
* **Measure the instrument before believing it.** The first reading of the total order looked like
  a 5% penalty on the median position. It was the comparison: `precedes` cost a second branch in a
  scan that is quadratic in the move count and runs at every node. The same ordering measured
  -0.5%, +1.4% and +2.7% depending only on whether the comparison was two branches, a packed i64
  or a single i32 — identical trees, on the digit, all three times.

**`openings_wide.txt` is qualified.** v0.39.0's run is the first to see all 613 opening families:
design effect **1.01**, effective sample 2978 of 3000 pairs, no losses on time, one identical game
in 6000. Against the old pool's **1.74** over the same game count, that is a third of the sample
recovered. White scores **64.24%** against the old pool's 64.03%, so the colour bias is a property
of this engine at 1s + 100ms and not of either pool — the 69% and 72% seen in two gauntlets were
the first 50 lines of the file, because **`mm.sh` takes its opening as `opening_lines[r % n]`** and
a run at `rounds = 50` never reaches line 51.

The next action is section **5, the staged `MovePicker`** — a throughput change rather than a tree
change, so it is priced differently: bit-identity against v0.39.0 on 14 of 14 corpus positions
first, then nodes per second at equal tree, and games only if the identity gate cannot be met.

### The backlog, in order

| # | Item | Where | Why this order |
| ---: | :--- | :--- | :--- |
| 1 | `MovePicker` stages 1-3 | 5 | The throughput prize, but read 5.2 before starting |
| 2 | NNUE incremental accumulator | 6 | Only worth it once `use_nnue` is the default path |


The `MovePicker` item is the most dangerous in this document — **read 5.2 before writing
any code**. 5.2 states the constraint that governs the whole item (the history table
must be snapshotted at node entry). It is a throughput change, not a tree change, so it is priced
differently from sections 1 to 4: the question is nodes per second at equal tree, and only then
games.

The proposal that used to be section 11 — damping the check exemption — was measured on
2026-08-28 and is dead. It is written up as a negative result in 8.5.

### Everything still open, in one place

A new session can start from this table; each row says where the detail is.

| Open | Where | Kind |
| :--- | :--- | :--- |
| The Transposition Table stores an unproven bound at Black nodes on an empty window | 7.1, 10.11 | defect, measured not to drift a warm table, unpriced |
| The root can hand a node an empty `alpha == beta` window | 7.1 | open question |
| Lazy Evaluation compares a `cheap_eval` that is missing the pawn structure on first visit | 10.8, 10.11 | defect, measured not to drift a warm table, unpriced |
| `singular_margin`, `singular_tt_depth_margin` and `singular_depth_reduction` shipped untuned | 4.4 | open tuning |
| `MovePicker` stages 1-3: needs an entry-time history snapshot, and five `#[allow(dead_code)]` attributes to resolve | 5, 5.2 | large item, constraints known |
| NNUE incremental accumulator, and making `use_nnue` the default | 6 | large item |
| Whether the 64% White score at 1s + 100ms is worth attacking — it is the engine's, not the pool's, and it inflates pair variance in every run | 10.7a | open question, cheap to test against another engine pairing |
| `scripts/measure_stage0.py` still uses a fixed `sleep` instead of `uci_driver.py` | 10.1 | unsafe measurement |
| Whether mirror-invariant move generation is worth measuring at all | 7.2 | open question, no prior reason to gain |
| `mm.sh` takes its opening as `opening_lines[r % num_openings]`, so a run at `rounds = 50` only ever sees the **first 50 lines** of the pool, whatever its size | 10.7a | measurement mechanic, established 2026-09-02 |
| `scripts/measure_stage0.py`, `measure_stage0_throughput.py` and `verify_stage0_identity.py` still drive the engine with a fixed `sleep` | 10.1 | unsafe measurement, and they are the instruments section 5 needs |
| The negative extension, the other half of the singular rebate, is untried | 4.4 | proposal, unmeasured |

Closed in v0.37.2: the twelve stale advertised UCI defaults and the thirteen inert option names
(1.1), the dead `Config::search_threads` field (10.6a), the dead `lmp_max_depth` tuning range
(10.6), and the evaluation half of the colour asymmetry (7.2). The facade is not sound yet — the
same repair left `UseNNUE` inert, which is the row above.

Closed on 2026-09-01, and not to be reopened: what v0.36.0, v0.37.0 and v0.37.2 are each worth
(10.10), and the scoreboard configuration that could not price them (10.9).

### Rules that are not optional

1. **Every search change is priced by matchplay, not by depth or test-suite accuracy.** The
   clearest evidence is the Check Extension frontier restriction in 8.2: it was the best of four
   axes on fixed-time depth and on LCT II, and measured **-26.8 Elo** in games.
2. **A self-A/B cannot see a defect both sides share.** v0.30.0 shipped a regression of roughly
   two hundred Elo (8.1) that four separate 1000-game self-A/B runs could not detect, because
   every one of them pitted a v0.30.x build against another v0.30.x build.
   `skills/engine_release_procedure.md` mandates a cross-version gauntlet for any change to
   `search_service.rs`, `eval_service.rs`, `move_gen_service.rs` or a search parameter default.
   **In `mode = gauntlet` only the challenger plays everyone.** A configuration that is not the
   challenger gets no direct pairing and only a chained rating with a wide interval, so if a run
   exists to qualify a configuration, that configuration must be the challenger or the mode must
   be `round_robin`. This cost the LMP decision one extra 9000-game run.
3. **Read Elo per pairing, never off the scoreboard.** The Matt-Magie scoreboard is normalised to
   a pool average, so two ratings from different PGNs are not comparable. Use
   `scripts/pairing_elo.py`.
4. **`nodes` and `nps` in UCI output report *generated* moves, not searched nodes.** Any change to
   move generation moves that number for reasons unrelated to speed, and
   `scripts/benchmark_nps.py` reads exactly that field. Measure throughput as wall time to a fixed
   depth instead.
5. **Prefer a node-identity check to a match wherever one exists.** Both throughput wins in
   Milestone 1 were verified as bit-identical search trees before a single game was played, which
   is why they needed no Elo measurement at all.
6. **Before pricing a depth-gated rule, measure the root depth the time control produces, and
   check the rule fires there.** The scoreboard run of 10.9 plays at a median root depth of 5,
   where Singular Extensions are provably inert — node counts with the rule on and off are
   identical on 24 of 24 positions at root depth 5 and 6 — so v0.37.x plays v0.36.0's chess in
   about 88% of its moves there. The cost of such a rule also grows with root depth: the census
   of 4.1 read +18.0% tree at depth 9 and the same corpus reads +51.2% at depth 11. A number
   taken below the depth of play transfers in neither direction.
7. **Gate with a sequential test if you like, but never quote its score as the effect size.** The
   stopping rule and the number have to come from different runs. Every feature since v0.35.0 was
   gated with `--elo0 -10 --elo1 0`, which cannot establish a gain at all, and its stopping score
   went into the release notes: +14.1 for razoring, +5 to +10 for singular extensions. Re-priced
   with fixed-N runs of 6000 games the same two changes measure **+4.2** and **-1.4** (10.10).
   Decide the game count before the run, publish the interval, and treat any reading taken before
   that count as an excursion rather than a preview.


## 5. Staged `MovePicker`

`[Impact: High]` `[Complexity: High]` — the throughput prize, and the most dangerous item here.

```
 Stage 0: TT Hash Move
 Stage 1: Good Captures & Queen Promotions (MVV-LVA / SEE >= 0)
 Stage 2: Killer Moves (Killer 1, Killer 2) & Counter Move
 Stage 3: Quiet Moves (ordered by History Heuristic)
 Stage 4: Bad Captures (SEE < 0)
```

### 5.1 What the stages are actually worth

Measured on v0.32.0 via `src/search_diag.rs` behind the `search-diag` Cargo feature, over 14
positions at fixed depth 10 and 764,055 interior nodes, on a node-identical tree. Reproduce with
`scripts/measure_stage0.py` against a `--features search-diag` build.

| Stage | of interior nodes | cumulative |
| :--- | ---: | ---: |
| **0** PV/TT move | 19.1% | 19.1% |
| **1** capture | 25.1% | 44.2% |
| **1b** quiet giving check | 2.6% | 46.8% |
| **2** killer / counter move | 10.4% | **57.2%** |
| **3** ordinary quiet move | 0.9% | 58.1% |

* **Build stages 1-3 together, never Stage 0 alone.** Stage 0 is worth 19.1% of interior nodes
  and was measured at **-9.1% throughput** (median, paired, 13 of 14 positions slower; -3.0% even
  in the configuration whose tree was bit-identical). It was built, verified and reverted; it
  survives on `experiment/stage0-short-circuit`. The 16 KB history snapshot cost about as much as
  the generation it saved. The prize here is the **57.2%** of interior nodes that cut before a
  quiet move is generated — roughly three times Stage 0, against the same snapshot.
* **Stage 1b is the structural obstacle and it is small.** A quiet move giving check carries
  `give_check_rank_bonus * 10000` = 50,000, which is what lets quiets outrank captures today and
  is why the current order cannot be produced lazily at all. It is 2.6% of interior nodes. Moving
  that bonus out of the rank function into stage assignment unlocks every stage below it.
  A later measurement sizes the same class from the other side: **5.9% of all searched moves
  give check, 3.4% of them quiet.** The bonus is *not* what keeps those moves out of LMR and LMP,
  a claim this document made until 2026-09-02 and which reading the code refutes: both rules carry
  an explicit `!current_turn.gives_check` guard, and so does the SEE pruning of bad captures.
  Moving the bonus therefore cannot cost an exemption. What it changes is where checking moves are
  searched and what `turn_counter` every later move sees — an ordering change, to be priced as
  one.
* **The per-node buffers are already gone.** v0.38.1 hoisted the move list and the
  principal-variation arrays into an arena allocated once per search, worth **+3.5%** on a
  bit-identical tree. A staged picker can no longer claim the buffer initialisation of 8.4's 3.1%
  as part of its prize; what is left to win is generation and ranking.
* **Killers and counters need no generation at all** — two or three remembered moves validated
  against `NodeMasks`, for 10.4%.
* Stage 0's ceiling is presence, not quality: the PV/TT move cuts at 77.5% of nodes where it
  exists, and 24.2% is simply the TT hit rate on interior nodes.

### 5.2 The constraint that governs the whole item

**Since v0.39.0 the move order is total** — the generation index sits in the low
`RANK_TIEBREAK_BITS` of the rank and the bands nest — so a staged picker can be held to the order
by node identity at all. Before that it could not: ties were broken by array position and then
permuted by the scan's own `swap`, so the order depended on the swap history rather than on the
position. The constraint below is what remains.

**No lazy or staged move picker in this engine can be verified by node identity unless it ranks
against an entry-time history snapshot.** Deferring generation ranks the remaining moves against a
History Heuristic that the first searched move's own subtree has already mutated, including the
global halving. With an entry-time snapshot the tree is bit-identical on 14/14 positions; without
it, 0/14, with searched-node counts differing by up to a factor of four.

A second trap, found by a duplication check rather than by reasoning: `add_promotion_moves` adds
`give_promotion_rank_bonus_queen * 10000` = 170,000, so a promoting capture giving check reaches
310,000 and **outranks the PV/TT move**. The rank ranges overlap instead of nesting.

* **Tasks**:
    - `[ ]` Implement stages 1-3 in `src/move_gen_service.rs` and `src/search_service.rs`, ranking
      against an entry-time history snapshot.
    - `[ ]` Remove the five `#[allow(dead_code)]` attributes in `src/move_gen_service.rs`
      (`is_pseudo_legal`, `is_castling_shape`, `build_stage0_move`, `stage0_rank`,
      `white_to_move_pawns_on_seventh`) by putting them to use. The release procedure forbids the
      attribute, so the next release that touches this file has to resolve it.
      **`build_stage0_move` and `stage0_rank` mirror the ranking loop in
      `get_valid_moves_from_move_list` and nothing enforces that they stay in step** — retune the
      move ordering and this copy rots silently.
    - `[ ]` `MoveRawList.moves` is `[u8; 256]`, i.e. 128 from/to pairs, and `push` silently drops
      anything beyond that while the legal maximum is 218. Generating per stage removes the
      problem; widening the buffer instead would cost initialisation time on every node.

## 6. NNUE: incremental accumulator and SIMD

`[Impact: High]` `[Complexity: High]` — only worth starting once `use_nnue` is the default path.

NNUE currently runs on **full recomputation per leaf** and is off by default on `master`.

* **Tasks**:
    - `[ ]` Add `AccumulatorStack` to `Board`, updated incrementally in `do_move` / `undo_move`.
    - `[ ]` AVX2 / SSE4.1 / NEON intrinsics for accumulator updates and the SCReLU forward pass.
    - `[ ]` Validate the incremental accumulator against full recomputation.
    - `[ ]` Set `use_nnue = true` as default in `src/config.rs` and `src/threads.rs`.
