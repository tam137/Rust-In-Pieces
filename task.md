# Suprah Engine Strength Enhancement Roadmap (`task.md`)

What to build next in **Suprah**, and the record of what has already been tried and failed.
Read "Negative results" before proposing anything: seven of the ideas in this document were built,
measured and reversed. Two of them looked excellent on every metric except games won, and one --
the staged `MovePicker` -- was correct, cut the generated moves in half, and was still slower.

---

## 🧭 Start Here

### Where the engine is

| | |
| :--- | :--- |
| Released | **v0.39.1** on `master` (HCE) since 2026-09-03 — Quiescence Search en passant generation & ranking, smoke gauntlet 56.25% (+61.5% vs v0.39.0). Porting to `feature/nnue-evaluation` is governed by `skills/nnue_porting_and_release_procedure.md` |
| Throughput | **1.86x** over v0.30.3, from three measured changes on bit-identical search trees |
| Matchplay resolution | **+/-23 Elo at 500 games**, **+/-13 at 3000**, per pairing — measured on host A. On host C with paired openings: **+/-11 at 2000**, **+/-6.5 at 6000**, the last of these confirmed by v0.39.0's run, which returned [+19, +32] around +25.6 |
| Run cost | a **6000-game** fixed-N run is **2.3 s per game** at concurrency 5, i.e. **under 4 hours**. A 200-game smoke gauntlet is 8 minutes. Pricing one change per run is affordable; bundling changes to save a run is not a saving worth having |
| Blocked on | nothing. The staged `MovePicker` was finished, priced and reversed on 2026-09-03: section 5 is a negative result, and `master`'s search is v0.39.0's again |
| Runs on | **host C (ARM, 8 cores)** since 2026-08-28 — resolve `<mm>` and rebuild the binaries there; nothing from host A or host B runs or transfers. Concurrency cap here is **5**, from `floor(nproc * 0.75) - 1` |


### What has shipped

See the Engines Changelog if needed.


### The next action

**Item 1 (QS En Passant) is shipped in v0.39.1.** The SPSA pipeline for `singular_*` parameters
is integrated and verified (`tuning/tune_singular.sh`). The next actions on the backlog are:

1. **Execute SPSA tuning for `singular` parameters** (`singular_margin`, `singular_tt_depth_margin`,
   `singular_depth_reduction`) using `tuning/tune_singular.sh`, then harvest results via
   `skills/spsa_harvest_results.md`.
2. **The negative extension**, the other half of the singular rebate, is still unmeasured.
3. **A search audit against published practice**, done 2026-09-04, produced sections 20 to 26:
   seven rules that are either absent from this engine or present in a form that cannot fire at
   the depths it plays. **None of it is measured** — every section is a proposal with a mechanism
   and a measurement plan, and nothing in them may be quoted as an effect size. Sections 20, 22
   and 23.1 to 23.3 are between five and thirty lines each; 24, 25 and 26 are reworks. The order
   to take them in is the backlog table below.

   The new sections are numbered from 20 deliberately. Numbers 1 to 12 belong to the write-ups
   deleted on 2026-09-02, and `src/threads.rs`, `src/search_service.rs` and `src/config.rs` still
   carry back-references to `task.md` 10.5, 10.6, 10.10, 10.12 and section 11 that resolve against
   revision `2a280c0`. Reusing those numbers would silently redirect them.

**The bands are what paid.** Measured **+25.6 Elo** over 6000 games, 95% interval **[+19, +32]**,
against a bound fixed before the run at -5. Deterministically, **21.4% less work to fixed depth 10**
over 300 pool positions. The three inversions it repaired are listed in the changelog; none of
them was a missing heuristic, all three were the rank scale contradicting itself.

Three lessons from the measurements, all of which change how the next one is run:

* **The 14-position corpus cannot rank an ordering change.** Re-permuting a tie class moves single
  positions by factors in *both* directions -- one variant read -1.5% on the corpus and +2.7% over
  300 pool positions, and Kiwipete alone swung from 1.40M to 3.83M nodes between two orderings
  worth the same in aggregate. The corpus is sized for node identity, which is binary. Use
  `scripts/measure_tree_size.py` for anything that re-shapes the tree.
* **Measure the instrument before believing it.** The first reading of the total order looked like
  a 5% penalty on the median position. It was the comparison: `precedes` cost a second branch in a
  scan that is quadratic in the move count and runs at every node. The same ordering measured
  -0.5%, +1.4% and +2.7% depending only on whether the comparison was two branches, a packed i64
  or a single i32 -- identical trees, on the digit, all three times.
* **The throughput instrument has a noise floor, and it is wider than most of the differences
  worth arguing about.** `measure_throughput.py` with `suprah-0.39.0` on *both* sides reads a
  median between -0.4% and +0.0% and a corpus total of -0.3% at `--repeats 3`; the candidate runs
  second and pays about a third of a point for it. Three separate attributions made during the
  picker work at `--repeats 1` and `2` did not survive being re-measured at `3`, and one of them
  reversed sign. Read the **corpus total**, not the mean -- the mean is dominated by the sub-20 ms
  positions -- and treat anything under about half a point as no difference at all.

**`openings_wide.txt` is qualified.** v0.39.0's run is the first to see all 613 opening families:
design effect **1.01**, effective sample 2978 of 3000 pairs, no losses on time, one identical game
in 6000. Against the old pool's **1.74** over the same game count, that is a third of the sample
recovered. White scores **64.24%** against the old pool's 64.03%, so the colour bias is a property
of this engine at 1s + 100ms and not of either pool -- the 69% and 72% seen in two gauntlets were
the first 50 lines of the file, because **`mm.sh` takes its opening as `opening_lines[r % n]`** and
a run at `rounds = 50` never reaches line 51.

### The backlog, in order

| # | Item | Where | Why this order |
| ---: | :--- | :--- | :--- |
| 1 | En passant is invisible to the Quiescence Search | 7 | Shipped in v0.39.1 |
| 2 | `singular_*` SPSA tuning | 8 | Infrastructure configured (`tuning/tune_singular.sh`), ready to tune |
| 3 | Killers, history and counter moves are cleared at every iterative deepening iteration | 23.1 | A lifetime defect, not a heuristic change; nothing else on this list is cheaper |
| 4 | The Null Move Pruning static-eval gate, and `!is_pv` on NMP and RFP | 20.1, 20.2 | Five lines, against an evaluation the node already computed |
| 5 | Internal Iterative Reduction | 22 | Six lines; the engine has nothing in this family at all |
| 6 | The history table is not side-indexed, and cannot go negative | 23.2, 23.3 | Two independent defects in the statistic three other rules read |
| 7 | `improving`, the Late Move Pruning growth term, and the Reverse Futility depth bound | 21 | Needs the per-ply static-eval stack, so it lands after 4 |
| 8 | The Null Move reduction and its verification search | 20.3 | Parameters, not code — belongs in a tuning group once 20.1 has landed |
| 9 | The history bonus and malus curves | 23.4 | Only after 23.3, and only with its own SPSA group |
| 10 | Transposition Table: index, clusters, ageing, cached static eval | 25 | Large, and it touches the one structure every other item reads |
| 11 | Continuation History replacing killers and counter moves | 24 | Two runs minimum: the untuned rework is known to measure worse |
| 12 | ProbCut | 26 | The most speculative rule that still fires at the depth of play |
| 13 | NNUE incremental accumulator | 6 | Only worth it once `use_nnue` is the default path, and it is not |

Two proposals that used to have sections are dead and are not to be reopened. Damping the check
exemption, measured 2026-08-28: worth 4.5% of the tree and nothing in games. The staged
`MovePicker`, measured 2026-09-03: section 5.

### Everything still open, in one place

A new session can start from this table. The sections these rows once pointed at were deleted when
the document was trimmed on 2026-09-02; the write-ups are still in git, at revision `2a280c0`.

| Open | Kind |
| :--- | :--- |
| The Transposition Table stores an unproven bound at Black nodes on an empty window | defect, measured not to drift a warm table, unpriced |
| The root can hand a node an empty `alpha == beta` window | open question |
| Lazy Evaluation compares a `cheap_eval` that is missing the pawn structure on first visit | defect, measured not to drift a warm table, unpriced |
| `singular_margin`, `singular_tt_depth_margin` and `singular_depth_reduction` SPSA tuning | configured in `tuning/`, ready for tuning runs — section 8 |
| The Quiescence Search never generates en passant — shipped in v0.39.1 | resolved in v0.39.1, see section 7.3 |
| `tt_move` is captured at node entry, and Null Move Pruning and razoring each run a recursive search before generation probes the table again — so the two can disagree about this node's table move | property, not a defect in the eager search; it broke the staged picker, see 5.2 |
| The bad-capture pruning decision reads `alpha`, which moves during the node — harmless while every capture is evaluated once, latent for anything that evaluates one twice | latent, only reachable from a staged picker, see 5.2 |
| NNUE incremental accumulator, and making `use_nnue` the default — section 6, and not while work is HCE-only on `master` | large item, parked |
| Whether the 64% White score at 1s + 100ms is worth attacking — it is the engine's, not the pool's, and it inflates pair variance in every run | open question, cheap to test against another engine pairing |
| `scripts/measure_stage0.py` still drives the engine with a fixed `sleep` instead of `scripts/uci_driver.py` | unsafe measurement; it is kept because four other scripts import its 14-position corpus |
| `MoveRawList.moves` holds 128 from/to pairs against a legal maximum of 218, and `push` drops the rest silently | latent defect, needs a position with more than 128 moves to fire |
| `truncate_bad_moves = 99` truncates an unsorted list during search, so it drops moves in generation order rather than the worst ones | latent defect, same class |
| Whether mirror-invariant move generation is worth measuring at all | open question, no prior reason to gain |
| `mm.sh` takes its opening as `opening_lines[r % num_openings]`, so a run at `rounds = 50` only ever sees the **first 50 lines** of the pool, whatever its size | measurement mechanic, established 2026-09-02 |
| The negative extension, the other half of the singular rebate, is untried | proposal, unmeasured |
| `killer_moves`, `history_table` and `counter_moves` are allocated inside `get_moves`, which the iterative deepening loop in `game_handler.rs` calls once per depth — so all three are cleared at every iteration, not every move | defect, unmeasured, section 23.1 |
| The history table is `[from][to]` with no side-to-move index, so White and Black share every entry | defect, unmeasured, section 23.2 |
| History is `u32` and its malus saturates at zero, so a refuted quiet is indistinguishable from an unseen one and `lmr_history_bad_threshold` fires on the wrong moves | defect, unmeasured, section 23.3 |
| `enable_history_malus` ships `false`, and the bonus is `depth^2` with a global 4096-entry halving pass | property, unmeasured, section 23.4 |
| Null Move Pruning has no `static_eval >= beta` gate, although the evaluation is already computed at every node it runs at | proposal, unmeasured, section 20.1 |
| Null Move Pruning and Reverse Futility Pruning have no `!is_pv` guard, although razoring, Futility and LMP all do | proposal, unmeasured, section 20.2 |
| The Null Move reduction is `2 + depth / 6` and is verified above depth 6, against a published `3 + depth / 3` with no verification | proposal, unmeasured, section 20.3 |
| The engine has no `improving` flag, so no rule can scale on whether the side to move is doing better than two plies ago | proposal, unmeasured, section 21.1 |
| The Late Move Pruning growth term `2 * depth^2` makes every `lmp_max_depth` from 4 upwards search the same tree | defect, pinned by `test_lmp_max_depth_is_inert_above_four`, section 21.2 |
| `rfp_max_depth` is 3 against a published 6 to 9 | proposal, unmeasured, section 21.3 |
| There is no Internal Iterative Reduction and no Internal Iterative Deepening | proposal, unmeasured, section 22 |
| There is no continuation history; killers and the counter move occupy `BAND_KILLER` instead | proposal, unmeasured, section 24 |
| The Transposition Table indexes with a 64-bit modulo, holds one entry per slot, has no generation counter and caches no static evaluation | proposal, unmeasured, section 25 |
| There is no ProbCut | proposal, unmeasured, section 26 |
| There is no quiet-only move generator, which is what forced the staged picker's last stage to regenerate everything — see 5.4 | property, established 2026-09-04 |

Closed on 2026-09-01, and not to be reopened: what v0.36.0, v0.37.0 and v0.37.2 are each worth,
and the scoreboard configuration that could not price them.

Closed on 2026-09-03: the staged `MovePicker` (section 5) and raising en passant into the capture
band (section 7). Both were built and both were measured; neither cost a game run.

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


## 5. Staged `MovePicker` — built, priced and reversed, 2026-09-03

`[Negative result]` — it works. It is node-identical, it generates about **55% fewer moves**, and
it is **slower**. The reason is in 5.4 and it is not inside the picker, so do not rebuild this
expecting to tune your way out of it.

```
 Stage 0: TT Hash Move
 Stage 1: Good Captures & Queen Promotions (MVV-LVA / SEE >= 0)
 Stage 2: Killer Moves (Killer 1, Killer 2) & Counter Move
 Stage 3: Quiet Moves (ordered by History Heuristic)
 Stage 4: Bad Captures (SEE < 0)
```

`master` carries none of it: `enable_tt_move_first`, the stage loop and the five helper functions
that served it are gone, and the search is byte-for-byte v0.39.0's again, confirmed at 14 of 14
identical and inside the noise floor. The version that reached node identity is not kept anywhere:
`cfba697` holds the one with the four defects of 5.2 still in it, and 5.2 is what a rebuild would
need. Nothing here is worth rebuilding without 5.4 first.

### 5.1 What the stages are worth, and why that was not enough

Measured on v0.32.0 via `src/search_diag.rs` behind the `search-diag` Cargo feature, over 14
positions at fixed depth 10 and 764,055 interior nodes, on a node-identical tree.

| Stage | of interior nodes | cumulative |
| :--- | ---: | ---: |
| **0** PV/TT move | 19.1% | 19.1% |
| **1** capture | 25.1% | 44.2% |
| **1b** quiet giving check | 2.6% | 46.8% |
| **2** killer / counter move | 10.4% | **57.2%** |
| **3** ordinary quiet move | 0.9% | 58.1% |

The 57.2% is the share of interior nodes that cut before a quiet move is generated, and it is real:
the finished picker generates 55% fewer moves, which is the same statement measured from the other
end. **The mistake was reading that as the prize.** The other ~43% of nodes reach the last stage,
and there the picker does strictly *more* work than the eager path -- see 5.4.

The margin was already thinner than this table suggests. v0.38.1 hoisted the per-node buffers into
a per-search arena for +3.5%, so a picker can no longer claim the 3.1% of runtime the v0.31.0
profile put on buffer initialisation; and stage 0 alone had already measured **-9.1%** on v0.32.0.

### 5.2 The four defects between `cfba697` and node identity

`cfba697` was not one bug from working, it was four, and the document's own bisection of it
("stage 0 is identical, the capture stage is not") was wrong -- stage 0 alone diverged too. The
divergence is depth-dependent: the whole picker is 14 of 14 identical at fixed depth **8** and
breaks 5 of 14 at depth **11**, which is why it was mis-attributed.

The instrument that found all four is the one this section used to ask for: under an environment
switch, generate the eager list at every staged node and check that the move the picker is about
to hand over is the move the ranking loop would have selected next. It reports the divergence at
the node where it happens instead of as a score eleven plies later.

1. **The table-move candidate was the one probed at node entry.** `minimax` probes the table near
   the top of the node; generation probes it again where it generates. Between the two run Null
   Move Pruning and razoring, each a *recursive* search that writes to the shared table and can
   replace or evict this position's entry. The staged path carried the older move forward and
   searched a move the ranking loop had never given the table band to. Isolated by turning both
   rules off: zero divergences with them off, one with them on -- and in that one, *both* paths
   had a move at `BAND_TT`, they were just different moves.
2. **A staged node with no table move never ran its terminal test.** `candidate == None` still
   staged, with an *empty* list, and the terminal test before the loop was conditioned on the node
   not being staged. If the last stage then produced nothing either -- checkmate or stalemate --
   the loop fell through and the node returned `alpha` instead of a mate score. Rare, and it is
   why the diverging positions were the tactical ones.
3. **En passant does not rank in the capture band.** `get_en_passante_turns` builds its moves at
   rank 0 and `add_move` adds nothing but the check bonus, so an en passant capture sits at the
   *bottom of the quiet band*. The capture stage generated it, and searched it far too early. The
   premise written into `append_capture_stage` -- "en passant is a capture: it ranks in the
   capture band" -- was simply false. See section 7, including what happened when the rank was
   corrected instead.
4. **The bad-capture pruning decision was taken twice.** A capture that fails Static Exchange
   Evaluation is dropped by the staged path and regenerated by the last stage, where it is
   evaluated a second time -- and the pruning test reads `alpha`, which the killer moves searched
   in between can have moved. The eager path decides once. This one was **never fixed**: it
   accounted for 2 of the 5 divergences before the other three were repaired, and stops firing on
   the 14-position corpus afterwards, but it is a real latent difference. Anything that evaluates
   a capture twice has to carry the first verdict with it.

The constraint that governed the item held up and is worth keeping: **no staged picker in this
engine can be node-identical unless it ranks against a History Heuristic snapshotted at node
entry.** With the snapshot the tree is identical on 14/14; without it, 0/14, with searched-node
counts differing by up to a factor of four. Only the rows of the squares the side to move occupies
are ever read, so a node copies at most sixteen of the sixty-four.

### 5.3 What it measures

All against the `suprah-0.39.0` binary at fixed depth 11, `--repeats 3`, on host C. Every row is
**14 of 14 node-identical**; `nodes` is excluded from the identity criterion because it counts
generated moves, which is exactly what a staged picker legitimately reduces.

| | median | corpus total | faster |
| :--- | ---: | ---: | ---: |
| noise floor (v0.39.0 against itself) | -0.4% … +0.0% | -0.3% | 4-6 of 14 |
| picker off (`enable_tt_move_first=false`) | **-2.4%** | -1.9% | 1 of 14 |
| picker on | **-4.1%** | -3.4% | 0 of 14 |
| the gate this had to clear | +5.0% | | 11 of 14 |

### 5.4 Why it cannot reach the gate

The 4.1 points split in two, and only one half belongs to the picker.

**About 2 points are a toll for the staged control flow being inside `minimax` at all** -- the
`picker off` row above, which executes none of it. Three attempts to remove that toll each moved
the number by less than the noise floor or made it worse: putting the staging block behind
`#[inline(never)]`, removing the destructor that `NodeBuffers` acquired when it grew a boxed
field, and lifting the stage advance out of the per-move loop into an outer loop. The last of
those read **-3.2%**, worse than doing nothing. `minimax` is 3500 lines and sits on a codegen
cliff; without `perf`, which is not available on these hosts, and against a half-point noise
floor, this is not something to chase by guessing. Three tries was enough.

**The other 2 points are the picker's own overhead, and they are structural.** The last stage
re-generates the *entire* move list -- a full raw pass and a full ranking -- and then removes the
moves already searched with an O(n²) `contains` scan. Every node that reaches it therefore does
more work than the eager path did, on top of the stage-1 pass it already paid for. Only the 57.2%
that cut earlier win anything.

**Why the last stage had no choice, noted 2026-09-04.** It regenerates everything because there is
nothing else to call. `generate_moves_list_for_piece` takes an `only_captures` flag and there is no
`only_quiets` counterpart, so a quiet stage can only ask for the full list and subtract. A
generator that emits exactly the complement of the capture list -- verifiable against the existing
one by a consistency check over random games plus perft, which is cheap and total -- removes the
second raw pass, the second ranking and the O(n²) scan outright, and with them this whole half of
the deficit.

This does **not** reopen the item. The other 2 points, the toll for the staged control flow being
inside `minimax` at all, are unaffected: the `picker off` row read -2.4% while executing none of
the picker. A quiet-only generator moves the ceiling from about -2% to somewhere near zero, not to
+5%. It is recorded because the generator is independently useful and because 5.4 should not be
read as saying the second half of the overhead was irreducible.

So the ceiling is the point: a last stage that cost *nothing at all* still lands near -2%, not
+5%. Stage 0 alone cannot rescue it either -- it pays the same toll and can only save generation
at nodes that were going to cut anyway.

### 5.5 What survived it

* **A rule for anything that adds a per-node buffer.** The 16 KB history snapshot, inline in the
  arena level, grew a level from 6 KB to 22 KB and the arena from 1.5 MB to 5.6 MB, and cost the
  search 5.2% *without staging a single node* -- most of what v0.38.1 was measured +3.5% for.
  Behind a pointer it costs nothing. The arena is walked at every node; put anything that only
  some nodes read somewhere else.
* The five `#[allow(dead_code)]` helpers this item was built on -- `is_pseudo_legal`,
  `is_castling_shape`, `build_stage0_move`, `stage0_rank` and `white_to_move_pawns_on_seventh` --
  are **deleted from `master`**, 288 lines. They were dead in v0.39.0 too.
* `scripts/measure_throughput.py` takes `--base-options` and `--cand-options`, so one binary can
  be A/B'd against itself, and it reports tree identity with `nodes` excluded and says so.

## 6. NNUE: incremental accumulator and SIMD

`[Impact: High]` `[Complexity: High]` — only worth starting once `use_nnue` is the default path.

NNUE currently runs on **full recomputation per leaf** and is off by default on `master`.

* **Tasks**:
    - `[ ]` Add `AccumulatorStack` to `Board`, updated incrementally in `do_move` / `undo_move`.
    - `[ ]` AVX2 / SSE4.1 / NEON intrinsics for accumulator updates and the SCReLU forward pass.
    - `[ ]` Validate the incremental accumulator against full recomputation.
    - `[ ]` Set `use_nnue = true` as default in `src/config.rs` and `src/threads.rs`.

## 7. En passant

`[Impact: unknown]` `[Complexity: Low]` — one line ranks it, one flag hides it from the
Quiescence Search, and the two are separate questions with separate answers.

### 7.1 What it does today

`get_en_passante_turns` builds its moves with `rank = 0` and the en passant block calls `add_move`,
which adds nothing but the check bonus. An en passant capture therefore ranks at the **bottom of
the quiet band** — below every history-ranked quiet, below the killers, three bands below the
captures — even though its `capture` field is set.

Two things follow, and the second is the interesting one:

* It is searched last.
* **The lazy Static Exchange Evaluation never sees it.** That gate fires on a rank inside the
  capture band, and rank 0 is not in it. `see()` carries a careful en passant branch — the victim
  is not on the arrival square, so the captured pawn has to be removed from the occupancy by hand
  or "a defended en passant capture scores a full pawn too low and is pruned as a losing capture"
  — and that branch is unreachable from the move loop. It was written for a code path the rank
  keeps it out of.

### 7.2 Ranking it as a capture: measured, negative, 2026-09-03

`turn.rank = BAND_CAPTURE + 20000` — the pawn victim's Most Valuable Victim score with no
attacker penalty, which is exactly what an ordinary pawn-takes-pawn capture gets. Priced with
`scripts/measure_tree_size.py` at fixed depth 10 on two independent 300-position samples:

| sample | tree | median per-position ratio | faster |
| :--- | ---: | ---: | ---: |
| `openings/book_width.txt` | +2.0% | 0.994 | 119 of 300 |
| `openings/book_mixed.txt` | +3.4% | 0.995 | 99 of 300 |

Both samples agree in direction: **the tree gets bigger and the typical position gets slower.**
The first sample's total wall time read +1.7% and the second -2.0%, which is the total being
dominated by a handful of expensive positions — the median and the count are the readings to
believe here, and 600 positions agreeing is not a reshuffle.

Not shipped, and no game run spent on it. The plausible mechanism is that `BAND_CAPTURE` is above
`BAND_KILLER`, so this promotes a pawn-takes-pawn ahead of the killers and the counter move, and
a pawn capture is rarely the move that cuts. Whether the same is true of *ordinary* pawn captures
is a much larger question about the band layout and is not this item.

### 7.3 The Quiescence Search never generates it — shipped in v0.39.1, 2026-09-03

The en passant block was behind `!only_captures`, and `generate_valid_moves_list_capture` — the
Quiescence Search's generator — is the one caller that passes `true`. The Quiescence Search could
therefore neither play an en passant capture nor see one in its stand-pat, at any depth.

**Shipped in v0.39.1:**
1. Lifted en passant generation out of `!only_captures` in `src/move_gen_service.rs`.
2. When `only_captures == true`, assigned `turn.rank = BAND_CAPTURE + 20000` (MVV-LVA pawn-takes-pawn), ensuring proper ordering in QS.
3. Kept baseline rank (`0` + check bonus) for regular minimax search (`only_captures == false`), strictly preventing the tree inflation measured in Section 7.2.
4. Added dedicated TDD unit tests in `src/move_gen_service.rs` and `src/search_service.rs`.

**Measurements:**
- **Tree size & node identity** (`scripts/measure_tree_size.py` against `v0.39.0`, 300 positions from `book_width.txt`, depth 10, Hash=64, Threads=1):
  - Time: 27842 ms -> 28193 ms (-1.3%)
  - Nodes: 177,824,464 -> 188,298,154 (-5.9% generated moves)
  - Median per-position time ratio: **1.000** (neutral)
  - Faster: 134 of 300 positions
- **Mandatory cross-version smoke gauntlet** (200 games at 1s + 100ms, paired openings):
  - vs `Rust-In-Pieces V0.38.1`: 34 wins, 34 draws, 32 losses (**51.0%**)
  - vs `Rust-In-Pieces V0.39.0`: 40 wins, 43 draws, 17 losses (**61.5%**)
  - Total: 74 wins, 77 draws, 49 losses (**56.25%**, 112.5 / 200)
  - Both matchups exceed the $\ge 45\%$ smoke acceptance threshold.


## 8. Singular Extension parameter tuning (SPSA)

`[Impact: Medium]` `[Complexity: Low]` — The singular extension parameters shipped with untuned defaults.
The SPSA infrastructure is configured and verified to tune all three parameters.

### 8.1 Parameters and Search Role

| Parameter | Default | Range | Role in Search |
| :--- | ---: | :--- | :--- |
| `singular_margin` | 2 | [0, 64] | Verification search threshold: `tt_eval - singular_margin * depth`. |
| `singular_tt_depth_margin` | 3 | [0, 8] | TT depth requirement: entry qualifies at `tt_depth >= depth - singular_tt_depth_margin`. |
| `singular_depth_reduction` | 0 | [0, 8] | Verification search depth reduction: `((depth - 1) / 2 - singular_depth_reduction).max(0)`. |

### 8.2 SPSA Infrastructure Configuration

- **Tuning Definitions (`tuning/parameters.json`):** Registered with baseline values and legal ranges matching the UCI options.
- **Tuning Group (`tuning/groups.json`):** Dedicated group `"singular"` added, and included in `"search_and_ordering"` and `"all"`.
- **Runner (`tuning/tune_singular.sh`):** Configurable runner script invoking `tuning/spsa_tuner.py --group singular` via `<mm>`.
- **UCI Facade & Tests (`src/config.rs`):** Verified that `SingularMargin`, `SingularTtDepthMargin`, and `SingularDepthReduction` are parsed, clamped, and stored correctly across casing and separator styles.
- **Workflow:** Run tuning via `tuning/tune_singular.sh`, monitor progress via `skills/spsa_tuning_status.md`, and integrate converged parameters using `skills/spsa_harvest_results.md`.

## 20. Null Move Pruning: the missing static-eval gate, and the missing PV guard

`[Impact: unknown]` `[Complexity: Low]` `[unmeasured]` — five lines, against a static evaluation
this node has already paid for.

`search_service.rs:676`. The rule fires at every eligible node:

```rust
if config.enable_nmp
    && !skip_null_move
    && depth >= config.nmp_depth_threshold
    && !turn.gives_check
    && self.has_non_pawn_material(board, board.white_to_move)
```

Two guards that the published formulation carries are absent, and a third rule is present that
the published formulation drops.

### 20.1 There is no `static_eval >= beta` gate

The null move asks whether giving the opponent a free move still fails high. At a node whose
*static* evaluation is already below `beta`, that question has a predictable answer and the
reduced search that asks it is close to pure cost. The standard gate is one comparison.

The reason it is free here: `static_eval` is computed unconditionally at `search_service.rs:666`
for every node with `depth > 0 && !turn.gives_check` — which is a superset of the nodes Null Move
Pruning runs at. The value is already in a register when the rule is reached. Adding
`&& static_eval >= beta` cannot cost a single evaluation call.

The lazy-evaluation contract holds in the direction that matters. `calc_eval` returns the `cheap`
value early when `cheap - margin >= beta`, so a lazy return on the fail-high side is a value at or
above `beta + margin`; the gate accepts it, exactly as Reverse Futility Pruning at
`search_service.rs:752` already does with the same number.

### 20.2 There is no `!is_pv` guard, on this rule or on Reverse Futility Pruning

Razoring (`:779`), Futility Pruning (`:1186`) and Late Move Pruning (`:1153`) each carry `!is_pv`.
Null Move Pruning and Reverse Futility Pruning do not. Both therefore speculate on the principal
variation, where `beta - alpha > 1` and the score is the one that reaches the root.

`is_pv` is already tracked correctly through the recursion — the Principal Variation Search null
windows pass `false` at `:1431` and the full-window re-search passes `true` at `:1443` — so this
is a guard, not a plumbing change.

### 20.3 The reduction is shallower than the published one, and it is verified

`nmp_reduction: 2` with `nmp_dynamic_divisor: 6` gives `2 + depth / 6`: **R = 3 at the root depth
of 9 to 10 this engine reaches at the match time control.** The published adaptive form is
`3 + depth / 3`, plus a term in how far the static evaluation exceeds `beta`, capped:

```rust
let eval_term = ((static_eval - beta) / 200).min(3);
let reduction = 3 + depth / 3 + eval_term;
```

which is R = 6 to 7 at the same depths, before the margin term.

The engine additionally runs a **verification search** at `depth >= nmp_verification_threshold`
(6), re-searching this node at the reduced depth before the cut is taken. That doubles the cost of
every deep cut. The published pairing is the other way round: the static-eval gate of 20.1 is what
makes the verification unnecessary, because a node whose static evaluation is already at or above
`beta` is not the zugzwang case the verification exists to catch.

**Take 20.1 and 20.2 first and separately from 20.3.** The gate and the guards only remove searches;
the reduction and the verification change what a cut is allowed to conclude, and they are two more
parameters for a tuning group rather than a fixed choice.

**How to price it**: `scripts/measure_tree_size.py` over both 300-position samples for the
deterministic reading, then one fixed-N 6000-game run. Rule 7: decide the count before the run.
Note that 20.1 and 20.2 change the tree, so node identity is not available for any of this.

## 21. `improving`, and the two rules whose depth bounds keep them from firing

`[Impact: unknown]` `[Complexity: Medium]` `[unmeasured]`

### 21.1 The engine has no notion of `improving`

There is no per-ply record of the static evaluation, so no rule can ask whether the side to move
is doing better than it was two plies ago. `grep -in "improv" src/` returns nothing.

The standard construction is a stack written on node entry:

```rust
// STATIC_EVAL_UNAVAILABLE at every node entry, so a check node cannot leak a stale value
static_eval_stack[ply] = static_eval;
let improving = ply >= 2
    && static_eval_stack[ply - 2] != STATIC_EVAL_UNAVAILABLE
    && static_eval > static_eval_stack[ply - 2];
```

**The arena rule of 5.5 applies and is satisfied.** One `i16` per ply is 2 bytes against the 16 KB
history snapshot that cost the search 5.2% without staging a node. A `[i16; MAX_PLY + 1]` alongside
`killer_moves` is the right shape; it does not belong in `NodeBuffers`.

Note the reset discipline: the entry must be written at *node entry* on every node, including the
in-check nodes that have no static evaluation, or a check node inherits the value of whatever node
last occupied that ply.

### 21.2 The Late Move Pruning threshold outruns the position

Already established in `src/search_service.rs`, `test_lmp_max_depth_is_inert_above_four`, and
pinned by that test: `lmp_base_moves + 2 * depth^2` demands 53 quiet moves at a single node at
depth 5 and 75 at depth 6, so every `lmp_max_depth` from 4 upwards searches the same tree. The
test's own conclusion is the one to act on: *"The fix is the growth term, not the counter and not
the advertised bound."*

What the threshold admits today against the published form, which divides by two when the node is
not improving:

| depth | `3 + 2d^2` (today, capped at d = 4) | `(base + d^2) / (2 - improving)`, not improving | improving |
| ---: | ---: | ---: | ---: |
| 1 | 5 | 3 | 7 |
| 2 | 11 | 5 | 10 |
| 3 | 21 | 7 | 15 |
| 4 | 35 | 11 | 22 |
| 8 | inert | 35 | 70 |

At depth 4 the rule as written lets 35 quiet moves through before it prunes anything. A full move
list is rarely over 50 and most nodes cut long before that, which is precisely why the flat region
above 4 exists. The growth term wants to be `depth^2`, not `2 * depth^2`, and the cap wants to
move to 8 in the same change — the two are one edit and cannot be priced apart, because with the
old growth term the new cap is inert by construction.

`tuning/parameters.json` registers `lmp_max_depth` with `max: 8` and the UCI facade advertises
`max 10`, both over a region that is currently flat. Whatever the growth term becomes, the
advertised bound and the tuner's range have to be re-checked against it in the same change, or
SPSA keeps exploring nothing.

### 21.3 Reverse Futility Pruning stops at depth 3

`rfp_max_depth: 3` with `rfp_margin_per_depth: 80`. The published bound is 6 to 9 plies with a
margin near 90 to 100 per ply. At `depth = 3` today the rule demands a 240-centipawn surplus; the
same margin at depth 6 would demand 480, which is a wide enough gate that extending the depth
bound is not obviously the aggressive change it looks like.

`rfp_max_depth` and `rfp_margin_per_depth` are already SPSA-registered, so this is a range change
plus a tuning run rather than new code — but the range is only worth widening once 20.2 has given
the rule a `!is_pv` guard, because at depth 6 a PV-node static cut is a different proposition than
at depth 3.

### 21.4 What `improving` is worth beyond Late Move Pruning

The same flag conventionally scales the Reverse Futility margin and the Late Move Reduction table.
Those are separate changes with separate prices; 21.1 plus 21.2 is the smallest version that uses
the stack at all, and nothing else should be bundled into the run that prices it.

## 22. Internal Iterative Reduction

`[Impact: unknown]` `[Complexity: Low]` `[unmeasured]` — six lines. The engine has nothing in this
family: no Internal Iterative Deepening, no reduction on a missing table move.

A node at real depth with no Transposition Table move has no ordering guidance at all — the first
move it searches is whatever the capture band or the history table happens to rank first, and if
that move is wrong the node pays full depth to find out. The published rule spends one ply instead
of searching a badly ordered node at full depth:

```rust
// after the TT probe, before the depth <= 0 quiescence drop
if ply > 0 && depth >= iir_min_depth && tt_move.is_none() {
    depth -= 1;
}
```

`iir_min_depth` is 4 in the published form, applied at PV and non-PV nodes alike; the root is
exempt so that iterative deepening still completes the depth it was asked for.

Two placement constraints in this engine:

* It must come **after** the Transposition Table probe (`:600` onwards), which is what establishes
  `tt_move`, and **before** the `depth <= 0` quiescence branch at `:812`, so that a node reduced to
  zero drops into the Quiescence Search rather than searching at negative depth.
* `orig_alpha`/`orig_beta` are captured at `:657` and the entry is stored under the *reduced*
  depth. That is correct and intended — the node really was searched one ply shallower — but it
  means a later visit at the original depth will not accept the entry for a cutoff, which is the
  mechanism that makes the reduction self-repairing rather than permanent.

**How to price it**: `scripts/measure_tree_size.py` on both 300-position samples; the reading to
believe is the median per-position ratio and the count, per the lesson in *Start Here*. Then one
fixed-N run.

## 23. The History Heuristic has four defects, and the killers and counter moves share the worst one

`[Impact: unknown]` `[Complexity: Low to Medium]` `[unmeasured]` — four independent problems in
about thirty lines of code. They are listed cheapest first; each can be taken alone.

### 23.1 Every learned table is thrown away at every iterative deepening iteration

This is the one to fix first and it is not in `search_service.rs` at all.

The iterative deepening loop lives in `src/game_handler.rs:202` (and `:94` for `infinite`), and it
calls `SearchService::get_moves` once **per depth**. `get_moves` opens with

```rust
let mut killer_moves: [[Option<Turn>; 2]; 128] = [[None; 2]; 128];   // search_service.rs:90
let mut history_table = [[0u32; 64]; 64];                             // :91
let mut counter_moves: [[Option<Turn>; 64]; 64] = [[None; 64]; 64];   // :92
```

So the depth-8 search starts with empty killers, an empty history table and an empty counter-move
table. Everything the depth-7 search learned about this exact position is discarded, and the only
state that survives an iteration is the Transposition Table. The tables are re-learned from zero
at every depth, which is worst precisely at the deep iterations that matter most, and it means
the ordering quality the history heuristic is supposed to supply is never available early in an
iteration.

The published discipline is the other way round: the tables persist for the whole game, and each
new search **halves** the butterfly history so stale entries decay rather than staying saturated
at the cap. `ucinewgame` clears them.

The fix is to hoist the three tables out of `get_moves` into state that lives across the
iterative deepening loop and to halve the history on entry. That is a signature change to
`get_moves` and two call sites, and it interacts with nothing else in this list.

Note the interaction with 23.4: with `history_max_threshold` ageing as it is written today, a
persistent table changes how often the global halving pass runs, so 23.1 and 23.4 are cleaner
together than apart.

### 23.2 The history table is not indexed by side to move

```rust
let mut history_table = [[0u32; 64]; 64];                            // [from][to]
crate::model::BAND_QUIET + (*context.history_table)[from][to] as i32 // move_gen_service.rs:461
```

White and Black share every `[from][to]` entry. A quiet move that refutes for one side raises the
rank of the geometrically identical move for the other, in a position where it usually means
something else entirely. The published table is `[side][from][to]`, which is one extra dimension
and 8 KB.

`board.white_to_move` is available at both the write site (`:1489`) and the read site
(`move_gen_service.rs:461`), so this is an indexing change, not a plumbing change. **The one thing
to be careful about is which side's index is read at each site**: the write happens at the parent
node before `do_move`, the read happens during generation for the node whose moves are being
ranked, and the two must agree.

### 23.3 History can never go negative, so the LMR "bad" threshold cannot fire as intended

```rust
history_table[from][to] += (depth * depth) as u32;                        // :1492
history_table[b_from][b_to] = history_table[b_from][b_to].saturating_sub(penalty);  // :1502
```

The table is `u32` and the malus saturates at zero. A quiet move that has been actively refuted a
dozen times is therefore indistinguishable from a quiet move that has never been searched: both
read 0. `lmr_history_bad_threshold: 500` at `lmr_reduction` (`:1713`) consequently increases the
reduction for *unseen* moves, not for *refuted* ones — the opposite of what the parameter name
says and of what the reduction is for.

The published form is a signed table with a gravity update, which converges towards the cap
instead of clamping at it and never needs a rescaling pass:

```rust
// bonus may be negative; entry converges towards +/- MAX_HISTORY
*e += bonus - (*e) * bonus.abs() / MAX_HISTORY;
```

This is the change that makes `lmr_history_bad_threshold` meaningful, so it and 23.4 have to be
re-tuned together — the thresholds are calibrated to the magnitudes the update produces.

### 23.4 The malus is disabled, and the bonus curve is a rescaling pass

`enable_history_malus: false` at `config.rs:471`. The store loop at `:1495` exists and is off.

Separately, `depth * depth` as the bonus with a global halving of all 4096 entries whenever any
one of them passes `history_max_threshold: 9000` (`:1514`) is a different curve from the published
`min(mult * depth - sub, max)` with separate bonus and malus slopes. The published form makes the
malus steeper than the bonus so a refuted move is unlearned faster than a good one is learned, and
the gravity update of 12.3 removes the halving pass entirely.

Any change here moves four parameters at once (`hist_bonus_*`, `hist_malus_*`) plus the two LMR
thresholds, so it wants its own SPSA group rather than a hand-picked default. **It should not be
attempted before 23.3**, because a bonus curve tuned against a table that clamps at zero does not
transfer to one that goes negative.

## 24. Continuation History

`[Impact: unknown]` `[Complexity: High]` `[unmeasured]` — a rework, not an addition, and the one
item on this list that must not be measured without its tuning run.

Quiet moves are ordered today by three separate mechanisms occupying two bands: killer moves and
the counter move sit in `BAND_KILLER` with fixed bonuses (`killer_move_1_rank_bonus: 20000`,
`killer_move_2_rank_bonus: 10000`, `counter_move_rank_bonus: 15000`), and everything else sits in
`BAND_QUIET` ranked by the butterfly history. Late Move Reductions then consult all three
separately at `lmr_reduction` (`:1690`): one damping for a killer, one for a counter move, one
pair of history thresholds.

The published replacement is a pair of tables indexed by `[prev_piece][prev_to][piece][to]` — one
looking back one ply (which subsumes the counter move) and one looking back two — whose sum with
the butterfly history is *the* quiet ordering score, and is also the single statistic the reduction
consults. Killers and the counter-move table are then deleted rather than kept alongside.

What this engine would need:

* **Two `i16` tables of 12 x 64 x 12 x 64**, 2.36 MB each. Static or per-search-thread state — by
  the rule 5.5 established, emphatically *not* in `NodeBuffers` or the per-node arena, which is
  walked at every node.
* **The move that led to each ply, plus the piece that made it**, recorded at make time. The piece
  cannot be looked up from the board later, because it may have been captured in the meantime.
* **A null move must clear the previous-move slot** for its child, or the child ranks against a
  move that was never played.
* The band structure survives: the combined score replaces what `BAND_QUIET` holds and
  `BAND_KILLER` disappears. The total order of 5.2 and the `RANK_TIEBREAK_BITS` packing established
  in v0.39.0 both still apply and are what keeps the comparison a single `i32`.

**The measurement discipline this item needs is specific.** The published result for this rework
is that the *untuned* version measured worse than what it replaced, and that the gain appeared only
after the ordering constants and the reduction thresholds were tuned jointly. So:

* Do not run a game gauntlet on the untuned rework and conclude anything from it.
* The tuning group is the two history curves, the follow-up weight, and the four reduction
  thresholds, together — they are one calibrated system, exactly as 23.3 and 23.4 already are.
* This is therefore a two-run item at minimum, and it is correctly last among the search-rule
  items.

## 25. The Transposition Table: one slot, no ageing, no cached static evaluation, and a division in the probe

`[Impact: unknown]` `[Complexity: Medium to High]` `[unmeasured]` — `src/zobrist.rs`. Four separate
properties, of which two are throughput and two are search quality.

### 25.1 The index is a 64-bit modulo on the hottest path

```rust
let index = (*hash as usize) % self.table.len();   // zobrist.rs:204, get_entry
let index = (hash as usize) % self.table.len();    // zobrist.rs:224, insert_entry
```

`max_zobrist_hash_entries: 50_000_000` is not a power of two, so this is a real 64-bit integer
division, executed on every probe and every store — which is once or twice per node.

The standard alternative keeps arbitrary table sizes and costs a multiply:

```rust
let index = (((hash as u128) * (self.table.len() as u128)) >> 64) as usize;
```

This is **not** node-identical: it changes which positions collide, so the tree moves. It is
measurable with `scripts/measure_throughput.py` (which now takes `--base-options`/`--cand-options`
and reports identity with `nodes` excluded) and `scripts/measure_tree_size.py` together — the
throughput reading is the point and the tree reading is the control. Read the corpus total, not
the mean, and treat under half a point as no difference.

### 25.2 There is one entry per index and no ageing

`AtomicEntry` is a single `{key: u64, data: u64}` pair per slot: 16 bytes, one position, no
neighbours. The replacement policy at `:238` is depth-preferred with one exception for Quiescence
entries, and it has **no notion of when an entry was written**. A deep entry stored at move 12
occupies its slot for the rest of the game.

The published structure is a cluster of several entries sized to one 64-byte cache line, probed as
a group, with a generation counter bumped at the start of each search and an eviction score of
depth discounted by age. Entries shrink to fit — the full 64-bit key becomes a 16-bit verifier,
since the index already accounts for the rest — which is what pays for the extra entries.

Two things this engine would gain beyond hit rate: a `hashfull` figure that means something, and
somewhere to put a PV flag, which is what a replacement policy needs to protect principal
variation entries from ordinary ones.

The concurrency contract must survive. The current lockless scheme — invalidate the key, write the
data, restore the key — is what makes a torn read detectable, and a multi-entry cluster needs the
same property per entry, not per cluster.

### 25.3 The entry caches no static evaluation

`TranspositionEntry` holds `eval` — the *search score* — and no static evaluation. So every
revisit of a position recomputes `calc_eval` from scratch at `search_service.rs:666`, even though
the static evaluation of a position never changes.

The published entry carries the raw static evaluation next to the score, with a sentinel for "not
stored" (check nodes have none), and the node reuses it instead of calling the evaluation at all.

This is worth more here than the field size suggests, and it touches an item already on the open
list. *"Lazy Evaluation compares a `cheap_eval` that is missing the pawn structure on first visit"*
is a first-visit problem by construction; a cached raw evaluation means later visits do not have a
first visit to get wrong. The two should be looked at together.

One constraint: what is cached must be the **raw** evaluation, before any correction or clamping,
and the lazy-evaluation early return must not be cached as if it were a full evaluation — a lazy
return is a bound in one direction, not a value. Either store only full evaluations, or store the
lazy value with the margin that produced it. This is the detail that decides whether the item is
correct, and it should be settled before any code is written.

### 25.4 What order to take it in

25.1 alone is a self-contained throughput change with a clean measurement. 25.3 is a self-contained
search change. 25.2 is the one that rewrites the structure, and it subsumes the entry layout that
25.3 needs, so 25.3 is either done inside 25.2 or done first in the existing layout and re-done.
There is no version of this where all four land in one priced change.

## 26. ProbCut

`[Impact: unknown]` `[Complexity: Medium]` `[unmeasured]` — the most speculative item on this list,
and deliberately last.

Absent. The idea: before generating the node normally, ask whether some capture already beats a
*raised* `beta` at reduced depth. If one does, the node beats its real `beta` too, and the raised
bound is what makes that inference sound.

The published shape, capture-only:

* Gate on `!is_pv && !in_check && depth >= 5`, and on `beta` being far enough below the mate region
  that `beta + margin` does not run into it.
* `probcut_beta = beta + margin` with a margin near 180 centipawns.
* Generate captures only. For each, require `see_ge(move, probcut_beta - static_eval)` — the
  capture has to be plausibly large enough on its own before anything is searched.
* Confirm with a Quiescence Search at the raised null window, then with a **real** reduced-depth
  search at `depth - reduction` (reduction near 4, floored at 1). Both must clear `probcut_beta`.

Two constraints specific to this engine:

* **Rule 6 is satisfied**, and that is worth stating because it is the rule that killed the first
  attempt to price the Singular Extension. A minimum depth of 5 fires at plies 0 through 4 or 5 at
  the root depth of 9 to 10 this engine reaches at the match time control, not just at the root.
* **The store on a successful cut needs the 8.1 treatment.** The published version writes a lower
  bound under this position's hash at the confirmation depth. That is defensible — unlike the
  singular multicut, the result is backed by a legal capture and a real search of *this* position,
  with no move excluded — but this engine's history with speculative table writes is expensive
  enough that the first version should return without storing, and the store priced separately if
  at all.

`scripts/measure_tree_size.py` first: a rule that does not shrink the tree deterministically has
nothing to offer a game run.
