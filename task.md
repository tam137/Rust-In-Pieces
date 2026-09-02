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
| Blocked on | nothing. The staged `MovePicker` (5) is half built in the working tree, uncommitted, with `enable_tt_move_first` shipping `false`: stage 0 is node-identical, the capture stage is not. **Read 5.3 first** |
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

The next action is section **5, the staged `MovePicker`**, and it is **half built and unfinished
in the working tree** — read **5.3** before writing a line of it. Stage 0 is done and identical;
the capture stage is not, and there is a 6.5% regression on the unstaged path to undo first.

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
2026-08-28 and is dead: damping the exemption is worth 4.5% of the tree and nothing in games.

### Everything still open, in one place

A new session can start from this table. The sections these rows once pointed at were deleted when
the document was trimmed on 2026-09-02; the write-ups are still in git, at revision `2a280c0`.

| Open | Kind |
| :--- | :--- |
| The Transposition Table stores an unproven bound at Black nodes on an empty window | defect, measured not to drift a warm table, unpriced |
| The root can hand a node an empty `alpha == beta` window | open question |
| Lazy Evaluation compares a `cheap_eval` that is missing the pawn structure on first visit | defect, measured not to drift a warm table, unpriced |
| `singular_margin`, `singular_tt_depth_margin` and `singular_depth_reduction` shipped untuned | open tuning |
| `MovePicker` stages 1 and 2 are built and not yet node-identical — section 5, and 5.3 before anything else | large item, half done |
| The unstaged path is 6.5% slower than v0.39.0 because the history snapshot sits in `NodeBuffers` — 5.3.3 | regression, cause known, blocks any release |
| NNUE incremental accumulator, and making `use_nnue` the default — section 6, and not while work is HCE-only on `master` | large item, parked |
| Whether the 64% White score at 1s + 100ms is worth attacking — it is the engine's, not the pool's, and it inflates pair variance in every run | open question, cheap to test against another engine pairing |
| `scripts/measure_stage0.py`, `measure_stage0_throughput.py` and `verify_stage0_identity.py` still drive the engine with a fixed `sleep` instead of `scripts/uci_driver.py` | unsafe measurement; the last two also set `EnableTtMoveFirst` as the abandoned branch defined it, so they run against nothing on `master` |
| `MoveRawList.moves` holds 128 from/to pairs against a legal maximum of 218, and `push` drops the rest silently | latent defect, needs a position with more than 128 moves to fire |
| `truncate_bad_moves = 99` truncates an unsorted list during search, so it drops moves in generation order rather than the worst ones | latent defect, same class |
| Whether mirror-invariant move generation is worth measuring at all | open question, no prior reason to gain |
| `mm.sh` takes its opening as `opening_lines[r % num_openings]`, so a run at `rounds = 50` only ever sees the **first 50 lines** of the pool, whatever its size | measurement mechanic, established 2026-09-02 |
| The negative extension, the other half of the singular rebate, is untried | proposal, unmeasured |

Closed in v0.37.2: the twelve stale advertised UCI defaults and the thirteen inert option names,
the dead `Config::search_threads` field, the dead `lmp_max_depth` tuning range, and the evaluation
half of the colour asymmetry. The facade is not sound yet — the same repair left `UseNNUE` inert,
and that branch is no longer maintained.

Closed on 2026-09-01, and not to be reopened: what v0.36.0, v0.37.0 and v0.37.2 are each worth,
and the scoreboard configuration that could not price them.

### Rules that are not optional

1. **Every search change is priced by matchplay, not by depth or test-suite accuracy.** The
   clearest evidence is the Check Extension frontier restriction: it was the best of four
   axes on fixed-time depth and on LCT II, and measured **-26.8 Elo** in games.
2. **A self-A/B cannot see a defect both sides share.** v0.30.0 shipped a regression of roughly
   two hundred Elo -- the fail-soft running score, reverted in v0.30.3 -- that four separate
   1000-game self-A/B runs could not detect, because
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
   check the rule fires there.** The scoreboard run that tried to price v0.36.0 through v0.37.2
   plays at a median root depth of 5,
   where Singular Extensions are provably inert — node counts with the rule on and off are
   identical on 24 of 24 positions at root depth 5 and 6 — so v0.37.x plays v0.36.0's chess in
   about 88% of its moves there. The cost of such a rule also grows with root depth: the census
   of the Singular Extension read +18.0% tree at depth 9 and the same corpus reads +51.2% at
   depth 11. A number
   taken below the depth of play transfers in neither direction.
7. **Gate with a sequential test if you like, but never quote its score as the effect size.** The
   stopping rule and the number have to come from different runs. Every feature since v0.35.0 was
   gated with `--elo0 -10 --elo1 0`, which cannot establish a gain at all, and its stopping score
   went into the release notes: +14.1 for razoring, +5 to +10 for singular extensions. Re-priced
   with fixed-N runs of 6000 games the same two changes measure **+4.2** and **-1.4**.
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
  bit-identical tree. A staged picker can no longer claim the buffer initialisation, 3.1% of
  runtime in the v0.31.0 profile, as part of its prize; what is left to win is generation and
  ranking, the ~54% that profile put beside it.
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
    - `[x]` The five `#[allow(dead_code)]` attributes in `src/move_gen_service.rs` are resolved.
      `is_pseudo_legal`, `is_castling_shape` and `build_stage0_move` are in use; `stage0_rank` and
      `white_to_move_pawns_on_seventh` are deleted, because the bands made both obsolete — the
      table move ranks at a constant, and a promotion can no longer outrank it.
    - `[x]` Stage 0, node-identical, **+1.0%** at fixed depth 11.
    - `[ ]` Stages 1 and 2: built, **not node-identical yet**. See 5.3.
    - `[ ]` `MoveRawList.moves` is `[u8; 256]`, i.e. 128 from/to pairs, and `push` silently drops
      anything beyond that while the legal maximum is 218. Generating per stage removes the
      problem; widening the buffer instead would cost initialisation time on every node.

### 5.3 Where the build stands — 2026-09-02, unfinished, read this before touching the picker

**The work is in the working tree, uncommitted, and `enable_tt_move_first` ships `false`, so the
released search is untouched.** With the flag off the build is 14 of 14 identical to v0.39.0.
Nothing here has been released and nothing may be until 5.3.2 is closed.

#### 5.3.1 What is proven

* **Stage 0 is node-identical and worth +1.0%** at fixed depth 11 (median +0.7%). The scheme that
  measured **-9.1%** when it was built on v0.32.0 and reverted -- it survives on
  `experiment/stage0-short-circuit` -- is dead: it needed a 16 KB history snapshot because the table
  move's rank carried history terms, and under the bands that rank is the constant `BAND_TT`.
* **Two rules read the move count before the loop and a staged node must stand down for both.**
  The One-Reply Extension asks whether the node has exactly one legal move. Singular Extensions
  ask whether the table move has alternatives, through `turns.len > 1` — and the table move is
  exactly what stage 0 searches, so a staged node switched the extension *and* the multicut off
  entirely: tree three times smaller, score shifted, everything faster and everything wrong. The
  guard is `!(config.enable_singular_extensions && depth >= config.singular_min_depth)`.
* **The identity criterion cannot include `nodes`.** It reports generated moves, which a staged
  picker legitimately reduces by about 15%. Depth, score and the principal variation of every
  completed iteration are what identity is read from. `scripts/measure_throughput.py` excludes it
  and says so in its output; it also takes `--cand-options EnableTtMoveFirst=true`, so one binary
  can be A/B'd against itself.

#### 5.3.2 The open defect: stages 0+1 are not identical

Bisected. Stage 0 alone is identical; adding the capture stage breaks it, with or without the
killer stage. Four bugs were found and fixed on the way there and are **not** the remaining one:

1. Quiet promotions were missing from the capture stage. They rank a band above the captures.
2. The same bug again, one level down: the raw pass ran with `only_captures`, so the promotion
   moves were never generated for the filter to keep. The capture stage now takes the full raw
   pass and saves the per-move work behind it instead.
3. Bad captures (SEE < 0) were searched inside the capture stage. They rank below every quiet
   move, so they are dropped there and regenerated by the last stage.
4. A killer slot and the counter move can hold the same move, which was then searched twice. The
   compaction now dedupes within a stage as well as against the searched prefix.

**Do not continue by guessing.** The instrument for this is the duplication check the old
`experiment/stage0-short-circuit` branch carried: under `search-diag`, generate the eager list at
every staged node and assert that the move the picker hands over next is the move the ranking loop
would have selected next. That reports the divergence at the first node where it happens instead
of as a score eleven plies later, and it is the same "do the work twice and compare" method the
move-generation cost breakdown was taken with, `perf` being unavailable on these hosts.

#### 5.3.3 A regression to undo first

With the flag off the build is identical to v0.39.0 and **6.5% slower**, 0 of 14 positions faster.
The cause is known: the history snapshot was put into `model::NodeBuffers`, which grows an arena
level from 6 KB to 22 KB and the arena from 1.5 MB to 5.6 MB. That is exactly the locality v0.38.1
was measured +3.5% for. **Move the snapshot into a second, parallel arena** that is split
alongside the buffers and touched only when a node actually stages.

#### 5.3.4 The gates that still stand

Unchanged, and fixed before the work resumed: **14 of 14 node-identical**, then **median
throughput >= +5.0% at equal tree with >= 11 of 14 positions faster**. Below that the item dies
with a negative result and no game run spent. A run costs under 4 hours, so if the identity gate
proves unreachable, pricing the staged picker in games is affordable — but only after 5.3.2 is
understood, because an unexplained divergence is a defect, not a design choice.

## 6. NNUE: incremental accumulator and SIMD

`[Impact: High]` `[Complexity: High]` — only worth starting once `use_nnue` is the default path.

NNUE currently runs on **full recomputation per leaf** and is off by default on `master`.

* **Tasks**:
    - `[ ]` Add `AccumulatorStack` to `Board`, updated incrementally in `do_move` / `undo_move`.
    - `[ ]` AVX2 / SSE4.1 / NEON intrinsics for accumulator updates and the SCReLU forward pass.
    - `[ ]` Validate the incremental accumulator against full recomputation.
    - `[ ]` Set `use_nnue = true` as default in `src/config.rs` and `src/threads.rs`.
