# Suprah Engine Strength Enhancement Roadmap (`task.md`)

What to build next in **Suprah**, and the record of what has already been tried and failed.
Read "Negative results" before proposing anything: eight of the ideas in this document were built,
measured and put back. Three of them looked excellent on every metric except games won: the staged
`MovePicker` was correct, cut the generated moves in half and was still slower, and Internal
Iterative Reduction removed two thirds of the tree and scored 35.5% against the release it was
built on.

---

## 🧭 Start Here

### Where the engine is

| | |
| :--- | :--- |
| Released | **v0.44.0** on `master` (HCE) since 2026-09-10 — the butterfly history is indexed by side to move, measured at +1.0 Elo [-6, +7] over 6000 games, i.e. a null that ships as the foundation for 23.3 and 23.4. Previously **v0.42.0** since 2026-09-08 — persistent killer, history and counter-move tables, worth +39.4 Elo [+29, +49]. Porting to `feature/nnue-evaluation` is governed by `skills/nnue_porting_and_release_procedure.md` |
| Throughput | **1.86x** over v0.30.3, from three measured changes on bit-identical search trees |
| Matchplay resolution | **+/-23 Elo at 500 games**, **+/-13 at 3000**, per pairing — measured on host A. On host C with paired openings: **+/-11 at 2000**, **+/-6.5 at 6000**, the last of these confirmed by v0.39.0's run, which returned [+19, +32] around +25.6 |
| Run cost | **the rate depends on the time control, so measure it before sizing a run.** At 1s + 100ms, concurrency 5: **2.3 s per game**, a 6000-game fixed-N run in under 4 hours. At 1s + 150ms, the control the singular campaign and 23.1 use: **3.75 s per game** measured 2026-09-07, so 6000 games is **6.25 hours** and a 240-game smoke gauntlet is 15 minutes. Pricing one change per run is affordable; bundling changes to save a run is not a saving worth having |
| Blocked on | nothing. The staged `MovePicker` was finished, priced and reversed on 2026-09-03: section 5 is a negative result, and `master`'s search is v0.39.0's again |
| Runs on | **host C (ARM, 8 cores)** since 2026-08-28 — resolve `<mm>` and rebuild the binaries there; nothing from host A or host B runs or transfers. Concurrency cap here is **5**, from `floor(nproc * 0.75) - 1` |


### What has shipped

See the Engines Changelog if needed.


### The next action

**The singular axis closed with v0.41.1 and the Quiescence Search en passant with v0.39.1**;
both write-ups are gone from this document, per `skills/task_management_procedure.md`. What is
in flight and what comes after it:

**Next up is what is left of backlog item 3: 23.3, the history that cannot go negative** — a
refuted quiet move and an unseen one both read 0, which is why `lmr_history_bad_threshold` fires
on the wrong moves. Item 1 closed 2026-09-09, item 2 the same day and negative, and 23.2 shipped
in v0.44.0 on 2026-09-10 as a null.

**Read 23.2 before starting 23.3.** It left the thresholds calibrated against a scale that has
already moved once, and 23.3 moves it again.

**23.3 now carries a written plan**, agreed 2026-09-10 and set out in that section: the table
becomes signed with a gravity update, the overflow rescale and `history_max_threshold` go,
`enable_history_malus` is switched on — without it nothing writes a negative entry and the change
would be a no-op — and the two Late Move Reduction thresholds are recalibrated against a
`search-diag` census of the pool before a single game is played. Nothing in it is built or
measured yet.

0. **Backlog 3 first half, section 23.2, shipped in v0.44.0 on 2026-09-10 as a measured null.**
   The butterfly history is `[side][from][to]`; White and Black no longer share an entry.
   **+1.0 Elo, 95% [-6, +7]** over 6000 games against v0.43.0, and the tree grew 1.8% and 5.8% at
   fixed depth 10 because splitting the table halved the magnitudes while the two LMR thresholds
   stayed put. It ships as the foundation 23.3 and 23.4 have to be priced on, not as a gain.
1. **Backlog 2, section 22, measured and refused 2026-09-09: Internal Iterative Reduction is a
   negative result on this engine.** It removes **70.4% and 64.9%** of the generated moves to
   fixed depth 10 and scores **35.5%** against v0.43.0 in its smoke gauntlet; restricted to
   `iir_min_depth = 8` it removes 39.3% and 42.8% and scores 42.0%. Four pairings, all below the
   45% gate, so no fixed-N run was spent and no number from it is an effect size. It ships behind
   `enable_iir`, default `false`. The cause is the table, not the rule: **41.4% of all nodes at
   depth 4 and above have no Transposition Table move** here. Section 22.3 says to ask again after
   backlog item 7, and to read the census share before booking a gauntlet.
2. **Backlog 1, section 20.1, measured 2026-09-09: the Null Move Pruning static-eval gate is
   Elo-neutral and makes the tree cheaper.** `-0.5 Elo, 95% [-7, +6]` over 6000 games against
   v0.42.0, against **+3.0% and +11.0% fewer generated moves** on the two 300-position pools and
   34.7% of all null searches removed. The gate ships enabled. Both `!is_pv` guards of 20.2 ship
   **disabled**: the same census reads the gate's whole saving back out again when they are on,
   because the root gives every root move `is_pv = true`. That root property is new, it is in the
   open list, and it is not to be changed inside a run pricing something else.

3. **Backlog 1, section 23.1, shipped in v0.42.0 — the largest single gain measured on this
   engine since the bands.** Killers, the history table and the counter moves moved from locals in
   `get_moves` into `EngineState`; they persist across the iterative deepening loop and the moves
   of a game, are halved on entry to each iteration and cleared on `ucinewgame`. **+39.4 Elo,
   95% [+29, +49]** over 2598 games, and 5.4% less wall time to fixed depth 10 over 300 pool
   positions. No heuristic changed — only how long three tables live. Section 23.1 carries the
   run's caveat: it was stopped by hand at 2598 of a planned 6000.

4. **The rest of the search audit is still unmeasured.** The audit of 2026-09-04 produced
   sections 20 to 26: seven rules either absent from this engine or present in a form that cannot
   fire at the depths it plays. Two of them have been through a run — 20 shipped Elo-neutral in
   v0.43.0, 22 was refused at the gate. **The remaining five are not measured** — each is a
   proposal with a mechanism and a measurement plan, and nothing in them may be quoted as an
   effect size. Sections 23.2 to 23.3 are between five and thirty lines each; 24, 25 and 26 are
   reworks. The order to take them in is the backlog table below.

5. **The negative extension**, the other half of the singular rebate, is still unmeasured, and
   section 8 leaves one open question on the same rule: whether `singular_margin = 0` wins through
   the extension or through the multicut it also maximises.

   The audit sections are numbered from 20 deliberately. Numbers 1 to 12 belong to the write-ups
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
| ~~1~~ | ~~The Null Move Pruning static-eval gate, and `!is_pv` on NMP and RFP~~ | 20.1, 20.2 | **Done 2026-09-09.** 20.1 shipped, Elo-neutral over 6000 games, 3.0%/11.0% cheaper tree. 20.2 ships disabled: it gives the whole saving back. See 20 |
| ~~2~~ | ~~Internal Iterative Reduction~~ | 22 | **Refused 2026-09-09.** -70.4% / -64.9% generated moves and 35.5% against v0.43.0; at `iir_min_depth = 8`, -39.3% / -42.8% and 42.0%. Ships `false`. Re-ask after item 7, not before — see 22.3 |
| 3 | History cannot go negative, so a refuted quiet move and an unseen one both read 0 | 23.3 | The half of item 3 that is left: 23.2 shipped in v0.44.0. This is what makes `lmr_history_bad_threshold` mean anything |
| 4 | `improving`, the Late Move Pruning growth term, and the Reverse Futility depth bound | 21 | Needs the per-ply static-eval stack, so it lands after 1 |
| 5 | The Null Move reduction and its verification search | 20.3 | Parameters, not code — belongs in a tuning group once 20.1 has landed |
| 6 | The history bonus and malus curves | 23.4 | Only after 23.3, and only with its own SPSA group |
| 7 | Transposition Table: index, clusters, ageing, cached static eval | 25 | Large, and it touches the one structure every other item reads |
| 8 | Continuation History replacing killers and counter moves | 24 | Two runs minimum: the untuned rework is known to measure worse |
| 9 | ProbCut | 26 | The most speculative rule that still fires at the depth of play |
| 10 | NNUE incremental accumulator | 6 | Only worth it once `use_nnue` is the default path, and it is not |

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
| `tt_move` is captured at node entry, and Null Move Pruning and razoring each run a recursive search before generation probes the table again — so the two can disagree about this node's table move | property, not a defect in the eager search; it broke the staged picker, see 5.2 |
| The bad-capture pruning decision reads `alpha`, which moves during the node — harmless while every capture is evaluated once, latent for anything that evaluates one twice | latent, only reachable from a staged picker, see 5.2 |
| NNUE incremental accumulator, and making `use_nnue` the default — section 6, and not while work is HCE-only on `master` | large item, parked |
| Whether the 64% White score at 1s + 100ms is worth attacking — it is the engine's, not the pool's, and it inflates pair variance in every run. **It is about four points smaller at the longer control**: 60.41% in v0.43.0's run and 59.86% in v0.44.0's, both 6000 games at 1s + 150ms on the same pool | open question, two independent readings at 1s + 150ms, established 2026-09-10 |
| `scripts/measure_stage0.py` still drives the engine with a fixed `sleep` instead of `scripts/uci_driver.py` | unsafe measurement; it is kept because four other scripts import its 14-position corpus |
| `MoveRawList.moves` holds 128 from/to pairs against a legal maximum of 218, and `push` drops the rest silently | latent defect, needs a position with more than 128 moves to fire |
| `truncate_bad_moves = 99` truncates an unsorted list during search, so it drops moves in generation order rather than the worst ones | latent defect, same class |
| Whether mirror-invariant move generation is worth measuring at all | open question, no prior reason to gain |
| `mm.sh` takes its opening as `opening_lines[r % num_openings]`, so a run at `rounds = 50` only ever sees the **first 50 lines** of the pool, whatever its size | measurement mechanic, established 2026-09-02 |
| The root searches **every** root move with `is_pv = true` — it runs no Principal Variation Search of its own, so this engine has one PV node per legal root move where the published formulation has one per iteration. It is why the `!is_pv` guards of 20.2 cost tree where they should be nearly free | defect or design choice, unmeasured, established 2026-09-09, section 20.2 |
| Whether the `!is_pv` guards of 20.2 are worth Elo despite costing tree — they ship disabled, and pricing them needs its own 6000-game run against v0.43.0, after the root question above | proposal, tree measured, Elo unmeasured, section 20.2 |
| The negative extension, the other half of the singular rebate, is untried | proposal, unmeasured |
| Splitting the history by side halved the magnitude an entry reaches, and `lmr_history_good_threshold` (4000) and `lmr_history_bad_threshold` (550) were not moved with it — the tree grew 1.8% and 5.8% at fixed depth 10 | property, measured 2026-09-10, section 23.2; the re-tuning belongs to 23.4 |
| History is `u32` and its malus saturates at zero, so a refuted quiet is indistinguishable from an unseen one and `lmr_history_bad_threshold` fires on the wrong moves | defect, unmeasured, section 23.3 |
| `enable_history_malus` ships `false`, and the bonus is `depth^2` with a global 4096-entry halving pass | property, unmeasured, section 23.4 |
| The Null Move reduction is `2 + depth / 6` and is verified above depth 6, against a published `3 + depth / 3` with no verification | proposal, unmeasured, section 20.3 |
| The engine has no `improving` flag, so no rule can scale on whether the side to move is doing better than two plies ago | proposal, unmeasured, section 21.1 |
| The Late Move Pruning growth term `2 * depth^2` makes every `lmp_max_depth` from 4 upwards search the same tree | defect, pinned by `test_lmp_max_depth_is_inert_above_four`, section 21.2 |
| `rfp_max_depth` is 3 against a published 6 to 9 | proposal, unmeasured, section 21.3 |
| Internal Iterative Reduction is built and ships `false`: it was refused at the gate on 2026-09-09 because **41.4% of the nodes at depth 4 and above have no table move** here, which is a property of the Transposition Table and not of the rule. Worth exactly one re-measurement after backlog item 7, starting with the census share | negative result, section 22.3 |
| There is still no Internal Iterative Deepening — the other member of the family searches the node at reduced depth first and uses its move, which is a second search and not a decrement | proposal, unmeasured |
| There is no continuation history; killers and the counter move occupy `BAND_KILLER` instead | proposal, unmeasured, section 24 |
| The Transposition Table indexes with a 64-bit modulo, holds one entry per slot, has no generation counter and caches no static evaluation | proposal, unmeasured, section 25 |
| There is no ProbCut | proposal, unmeasured, section 26 |
| There is no quiet-only move generator, which is what forced the staged picker's last stage to regenerate everything — see 5.4 | property, established 2026-09-04 |
| `SearchTables::age` halves the history once per iterative deepening *iteration*, because `get_moves` is one iteration; the published discipline halves once per `go` | untested variant, established 2026-09-07, section 23.1 |
| `time_check::run_time_check` drives its position list through one `EngineState`, so with persistent search tables its printed node counts are order-dependent | diagnostic only, no decision reads it, established 2026-09-07 |
| The startup benchmark `calculate_benchmark` runs a depth-3 search on the real `EngineState`, so the tables carry its history until the first `ucinewgame` | latent, harmless under any GUI that sends `ucinewgame`; the Transposition Table already had this property |

Closed on 2026-09-01, and not to be reopened: what v0.36.0, v0.37.0 and v0.37.2 are each worth,
and the scoreboard configuration that could not price them.

Closed on 2026-09-03: the staged `MovePicker` (section 5) and raising en passant into the capture
band (section 7). Both were built and both were measured; neither cost a game run.

Closed on 2026-09-08: the tables thrown away at every iterative deepening iteration, section
23.1, shipped in v0.42.0 at +39.4 Elo [+29, +49]. The section is kept rather than deleted because
it is the only record of the decay rate actually shipped, which is not the published one.

Closed on 2026-09-09, and reopenable only through backlog item 7: Internal Iterative Reduction,
section 22. Built, priced at two settings and refused at the smoke gauntlet in both; the section
is kept rather than deleted because it is the record of a rule that is correct, enormous in the
tree and worse in games, and of the table property that explains it.

Closed on 2026-09-07, and not to be reopened: en passant generation in the Quiescence Search
(shipped v0.39.1) and the singular parameter axis (shipped v0.41.1). Both write-ups were deleted
here once released and are recoverable from git at `792af14`; what survives of the singular
campaign is the compact record in section 8.

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

## 8. Singular Extension parameter tuning — shipped in v0.41.1, closed 2026-09-07

The three singular parameters shipped with untested defaults. All three were measured across
their useful range at 1s + 150ms, where the median root depth of 11 makes the rule actually fire.
The full write-up — ten SPRT screens over 32,698 games, three fixed-N price runs and the direct
`0`-against-`1` test — is in git at `792af14`.

**What shipped.** `singular_margin: 0`, down from an untested 2. Measured **+10.7 Elo, 95%
[+5, +17]** over 6739 games, pinned by
`test_the_shipped_singular_configuration_is_the_one_that_was_measured`. The ranking on the axis
is `0 > 1 > 2 > 3 > 6 > 4`, and 0 is the range boundary, so there is no room left downward.

**Two documented negatives, not to be retested.** `singular_tt_depth_margin` stays at 3 and
`singular_depth_reduction` stays at 0. The combination `margin = 1` with `tt_depth_margin = 2`
measured **+6.5**, *below* either parameter alone: the two pull against each other on the same
rule, so additivity does not hold on this axis.

**SPSA cannot decide this axis and must not be pointed at it again.** `tuning/spsa_tuner.py` uses
only the *sign* of a 2500-game batch and moves a parameter by `lr%` of the step. At the configured
`mutate=10`/`lr=5` the step is 1, so each iteration moves the value by ±0.05 — travelling from 2
to 7 would cost roughly 250,000 games, about 58 hours.

**Still open.** *Why* `margin = 0` wins. The `search-diag` counters measure extensions but not
multicuts, and `margin = 0` also maximises the multicut condition `threshold >= beta`. Separating
the two needs `margin = 0` with and without `enable_singular_multicut`.

## 20. Null Move Pruning: the missing static-eval gate, and the missing PV guard

`[Impact: measured]` `[Complexity: Low]` — five lines, against a static evaluation this node has
already paid for. **20.1 is built and measured: Elo-neutral, and it removes 3.0% and 11.0% of the
generated moves on the two 300-position pools. 20.2 is built and shipped disabled: the same census
reads the whole saving back out again when it is on.** Measured 2026-09-08 and 2026-09-09; the
numbers are under 20.1 and 20.2 below. 20.3 is untouched and still a proposal.

### What the run said

`suprah-0.43.0-rc` against `suprah-0.42.0`, **fixed N = 6000 games** decided before the start, no
early stopping, 1s + 150ms, `openings_wide.txt`, Hash=64 Threads=1 OwnBook=false, concurrency 5:

| | |
| :--- | :--- |
| Result | **-0.5 Elo for the candidate, 95% [-7, +6]** over 6000 games, 3000 pairs |
| Games | +1732 =2527 -1741 from the candidate's side, 50.1% to v0.42.0 |
| Health | no losses on time, 1 identical game in 6000, all 613 opening families seen |
| Design effect | 1.06, effective sample 2839 of 3000 pairs, intervals widen by 3% |

A null, at the resolution the count was chosen for. The point estimate crossed zero twice on the
way — +4.9 at 783 games, +0.5 at 1326, +1.9 at 3293, -2.1 at 4086 — which is what a null looks
like from the inside, and is the reason rule 7 exists.

The deterministic reading is not null, and it is the reason to keep the gate. See 20.1.

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
windows pass `false` and the full-window re-search passes `true` — so this is a guard, not a
plumbing change.

**Built, measured and shipped disabled, 2026-09-08.** The guard is not wrong. It is an order of
magnitude larger in this engine than in the formulation it comes from, and the reason is at the
root: `search_service.rs` searches **every** root move on the full aspiration window with `is_pv`
hardcoded `true`, because the root runs no Principal Variation Search of its own. The published
formulation has one PV node per iteration at the root; this engine has one per legal move. The
guard therefore switches Null Move Pruning off at every root child — the largest subtrees in the
search — and not on a narrow leftmost line.

The census says the same thing from the other end. The gate of 20.1 alone removes 3.0% and 11.0%
of the generated moves; with both PV guards added the same two pools read **-1.7% and -0.5%**, so
the guards give the entire saving back and a little more. Both halves of 20.2 ship as
`nmp_pv_guard` and `rfp_pv_guard`, both defaulting **false**, so the census can be reproduced from
one binary and the rule can be priced in games later without a variant build.

**What is still open on 20.2** is whether it is worth Elo despite costing tree. It removes
speculation from the nodes whose score reaches the root, which is the entire point of the guard,
and rule 1 says tree size does not price a search change — the Check Extension was the best of
four axes on every depth metric and -26.8 Elo in games. Pricing it needs its own 6000-game run
against v0.43.0, and it needs the root question below answered first, because the two interact:
if the root stopped calling every move a PV node, this guard would become the small rule the
literature describes.

**The root treats every root move as a PV node.** That is a separate item, and it is not to be
changed inside a run that is pricing something else — it moves the tree underneath the rule being
measured. Recorded in the open list.

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

**Measured, 2026-09-08.** The gate refuses **34.7%** of all Null Move Pruning candidate nodes,
over 40 positions at depth 10 with the `search-diag` counters: 387,272 candidates, 134,566 refused
by the gate, 36,335 by the PV guard, 60.7% surviving both, and 66.7% of the null searches that do
run produce a cutoff. Candidates by remaining depth are `3:212964, 4:94121, 5:45153, 6:19002,
7:10073, 8:4096, 9:1863`, so the rule is live at the root depth the time control reaches — rule 6
is satisfied, the gate is not inert at the depth of play.

The tree census, `scripts/measure_tree_size.py`, 300 positions per pool, fixed depth 10, one
binary against itself with the switch off on the base side:

| | `book_width.txt` | `book_mixed.txt` |
| :--- | ---: | ---: |
| generated moves, gate alone | **+3.0%** | **+11.0%** |
| generated moves, with both PV guards of 20.2 | -1.7% | -0.5% |

**Read the generated-move column, not the wall time.** The timing column of the same census reads
+2.1% and +7.1% for the gate, but every configuration — including the near-inert RFP guard —
sits about 5 percentage points higher on `book_mixed` than on `book_width`, which is an
instrument offset and not four independent pool effects. The generated-move count is a plain
count, no timing enters it, and move generation was not touched, so it is the honest reading here.
Both pools agree on it.

Shipped **enabled** in v0.43.0 as `nmp_static_eval_gate`. Elo-neutral at 6000 games, cheaper tree
on both pools: this is the Milestone-1 shape of change, except that it is not node-identical,
which is why it needed the run.

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

## 22. Internal Iterative Reduction — built, priced and stopped at the gate, 2026-09-09

`[Negative result]` — it works, it removes two thirds of the tree at fixed depth, and it loses
games. Do not rebuild this expecting to tune your way out of it: the reason is in 22.3 and it is
not inside the rule.

The published discipline spends one ply rather than searching a node with no Transposition Table
move at full depth. Six lines, after the table probe and before the `depth <= 0` quiescence drop:

```rust
if depth >= iir_min_depth && tt_move.is_none() {
    depth -= 1;
}
```

`master` carries it behind `enable_iir`, which ships **`false`**, with `iir_min_depth` (4) and
`iir_reduction` (1) as UCI options and a `search-diag` census. The switches are kept rather than
the code removed, the way `NmpPvGuard` and `RfpPvGuard` of 20.2 are kept: 22.3 says when this is
worth asking again, and the rule will be one flag away when it is.

### 22.1 What it does deterministically, and it is not small

`scripts/measure_tree_size.py`, one binary against itself, both 300-position pools, fixed depth 10:

| Setting | `book_width` | `book_mixed` | Median ratio | Nodes reduced |
| :--- | :--- | :--- | :--- | :--- |
| `iir_min_depth = 4` | **-70.4%** moves | **-64.9%** | 2.51x / 2.17x | 41.4% of nodes at depth >= 4 |
| `iir_min_depth = 8` | **-39.3%** moves | **-42.8%** | 1.17x / 1.26x | 28.2% of nodes at depth >= 8 |

Switched off, the build is bit-identical to v0.43.0 on 300 of 300 trees, to the digit — the rule 5
check that the plumbing is inert before anything measures what the rule does.

The second row is the one to understand. Restricting the rule to the top of the tree cuts the
population it fires on by a third, and the tree still shrinks by 40%: a ply given up near the root
deletes a whole layer beneath it. There is no setting of `iir_min_depth` at which this rule is
cheap, which is why the axis was not swept further.

### 22.2 What it costs in games

Smoke gauntlets per `skills/engine_release_procedure.md`, challenger first, 200 games each,
1s + 100ms, `openings_wide.txt`, concurrency 5:

| Setting | vs v0.43.0 | vs v0.42.0 |
| :--- | :--- | :--- |
| `iir_min_depth = 4`, the published form | 19/33/48, **35.5%** | 24/41/35, **44.5%** |
| `iir_min_depth = 8` | 27/30/43, **42.0%** | 23/39/38, **42.5%** |

Four pairings, two configurations, every one of them below the gate of roughly 45%, and the
second configuration is the more favourable half of the axis. **No fixed-N run was spent**, so
there is no interval and none of these percentages is an effect size — 100 games per pairing is
about ±80 Elo here. What they establish is the refusal, which is what a gate is for.

The engine gives up roughly a ply and a half of effective depth for the time it saves. Rule 1 in
one line: the tree got two thirds cheaper and the chess got worse.

### 22.3 Why it fails here, and when to ask again

**The rule assumes a table that answers.** It fires exactly where the probe returned no move, and
on this engine that is **41.4% of all nodes at depth 4 and above**. A published engine's rate is a
fraction of that, and the difference is not the rule — it is the table. `master`'s Transposition
Table indexes with a 64-bit modulo, holds one entry per slot, has no generation counter and caches
no static evaluation: **section 25, backlog item 7**. Every entry that survives longer is a move
handed out, and every move handed out is a node this rule stops reducing.

So this is a *mistimed* item rather than a wrong one. It is worth exactly one re-measurement after
item 7 lands, and the number to look at first is the census share, not the games: if the miss rate
at depth 4 has not fallen well below 41.4%, the gauntlet will read the same and does not need
running.

Two things this item did not try, and neither rescues the axis on its own: reducing only at
non-PV nodes — this engine gives every root move `is_pv = true`, so the guard is far larger here
than published, exactly as 20.2 found — and a second ply of reduction, which moves in the wrong
direction from a rule that is already too cheap by half.

## 23. The History Heuristic has four defects, and the killers and counter moves share the worst one

`[Impact: unknown]` `[Complexity: Low to Medium]` — four independent problems in about thirty
lines of code, listed cheapest first; each can be taken alone. **Two are closed**: 23.1 shipped in
v0.42.0 at +39.4 Elo, the largest gain measured on this engine since the bands, and 23.2 shipped
in v0.44.0 as a null. 23.3 and 23.4 remain, they change the *scale* of the statistic rather than
its indexing, and 23.4 must not be attempted before 23.3.

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

Note the interaction with 23.4: with `history_max_threshold` ageing as it is written today, a
persistent table changes how often the global halving pass runs, so 23.1 and 23.4 are cleaner
together than apart.

#### What was built, 2026-09-07

A signature change to `get_moves` was the obvious shape and is not the one taken: `get_moves`
has 35 call sites, about thirty of them tests. The three tables went into `EngineState` instead,
as `Mutex<SearchTables>` (`src/model.rs`), which no call site sees. The engine searches on one
thread — `threads.rs` rejects `setoption Threads` — so the lock is taken once per `get_moves` and
never on a search path.

* `search_service.rs:90` replaces the three local allocations with a guard, `age()` and a
  destructure; everything below already took `&mut` to these tables, so `minimax` is untouched.
* `game_handler.rs` calls `search_tables.reset()` in the `ucinewgame` block that already cleared
  the pawn and Zobrist tables. Persistent over a game, cleared between games.
* `age()` halves the history on entry to `get_moves` and deliberately leaves killers and counter
  moves alone: both are overwritten wholesale by the next cutoff at the same ply, so a stale entry
  there costs one ordering slot rather than a lasting bias in the statistic three other rules read.

**Read the decay rate carefully, because it is not the published one.** `get_moves` is one
iterative deepening *iteration*, not one search, so `age()` runs once per depth: a depth-10
search halves nine times and an early iteration's contribution is worth 2^-8 of a late one by
the end. The table still carries ordering from iteration to iteration, which is what 23.1 is
about, but "halve once per `go`" — the discipline as it is usually written — is a **different,
untested variant**. What the run below prices is the per-iteration version.

**Two tests were paired comparisons sharing one `EngineState`** — `test_dynamic_nmp_verification_search`
and `test_futility_pruning_node_reduction`, which compare node counts with a rule on and off.
Persistent tables would have let the first search order the second one's moves. Both now take one
state per search. The Transposition Table was always shared in those two, which is the same
hazard and was already latent.

**Deterministic reading.** `scripts/measure_tree_size.py`, 300 positions from `book_width.txt`,
fixed depth 10: **61816 ms -> 58488 ms, 5.4% less wall time**, median per-position ratio 1.035,
faster on 159 of 300, and 0 of 300 trees identical. Generated moves went the other way, +3.3%.
The census isolates positions with `ucinewgame`, which now also resets these tables, so this is
persistence *within* one `go depth 10` and not leakage between positions.

#### What it is worth — shipped in v0.42.0, 2026-09-08

**+39.4 Elo, 95% interval [+29, +49]**, against `suprah-0.41.1` at 1s + 150ms over 2598 games.

```
/root/mattmagie/hist_ab.pgn: 2598 games, 1 pairings

Rust-In-Pieces V0.41.1  vs  Rust-In-Pieces V0.41.1-hist
  2598 games   +642 =1021 -935   score 44.4%
  unpaired    -39.4  95% CI [-50, -29]
  paired      -39.4  95% CI [-49, -29]   (1298 pairs)
```

`pairing_elo.py` prints the baseline first, so the 44.4% and the minus sign belong to v0.41.1;
the candidate is the one ahead. Health: no losses on time, 0 identical games, 1112 distinct
openings over 613 families at 2.1 pairs each, ICC 0.025, **design effect 1.03**, effective sample
1263 of 1298 pairs. White scored 60.91%.

**Read the caveat with the number.** The run was planned as fixed N = 6000 and was **stopped by
hand at 2598** because the interval had separated from zero by more than four times its own
width. The interval is therefore not from a completed fixed-N design, exactly as the
`singular_margin = 0` run that shipped v0.41.1 was stopped at 6739 of 10000. At this count the
resolution is about +/-10 Elo, so the *direction and rough magnitude* are solid and the third
digit is not. Nothing here was gated on an interim reading — the stop was a decision to spend the
remaining four hours elsewhere, not a stopping rule.

The two measurements agree, which is the reason to believe this one: 5.4% less work to a fixed
depth deterministically, and a large matchplay gain, from a change that alters no heuristic at
all — only how long three tables live.

### 23.2 The history table was not indexed by side to move — repaired in v0.44.0, measured null

`[Measured]` `[+1.0 Elo, 95% [-6, +7] over 6000 games]` — shipped because the statistic is now
correct, not because it won anything. Read the last paragraph before building on this.

```rust
history_table[side][from][to]          // model.rs, `[2][64][64]`, White is side 0
crate::model::history_side(white)      // the one place the convention is written down
```

White and Black shared every `[from][to]` entry. A quiet move that refuted for one side raised the
rank of the geometrically identical move for the other, in a position where it usually means
something else. The repair is an indexing change and not a plumbing one: `board.white_to_move` was
already available at the write site and at the read site, and the two agree by construction — a
node credits the side to move at the cutoff, and the child that reads the entry during generation
is ranking that same side's moves one ply later. `test_history_credits_the_side_that_played_the_move`
pins it, using the property that a depth-2 search from the start position can only write for
Black; the test was verified to fail against a deliberately swapped index.

**What it measured.** Against v0.43.0, 6000 fixed-N games at 1s + 150ms, no early stopping, the
count fixed before the run: **+1.0 Elo, 95% paired interval [-6, +7]** over 3000 pairs, 50.1%.
No losses on time, design effect 1.00, effective sample 3000 of 3000. The smoke gauntlet read
47.0% against v0.43.0 and 61.0% against v0.42.0.

**The deterministic reading went the other way**, and this is the part to carry forward: the tree
grows by **+1.8% and +5.8%** generated moves to fixed depth 10 on the two pools, with 18 and 24 of
300 trees identical. Splitting one table into two roughly halves the magnitude an entry reaches
while `lmr_history_good_threshold` (4000) and `lmr_history_bad_threshold` (550) stay where they
are, so the Late Move Reduction became less generous about sparing well-scoring quiet moves. The
indexing is right; the thresholds are now calibrated against the wrong scale.

**So 23.2 is a foundation, not a gain.** It ships ahead of 23.3 and 23.4 for one reason: those two
change the scale of the same statistic, and pricing them on top of a table that averages the two
sides would confound them with this. The next run gets a clean baseline. Whether the repair
eventually pays depends on the re-tuning in 23.4, and a session that re-reads this section after
23.3 should expect the thresholds to move.

### 23.3 History can never go negative, so the LMR "bad" threshold cannot fire as intended

`[Planned]` — the plan below is the agreed one, written 2026-09-10 against `v0.44.0`. Nothing in
it has been built or measured, and no number in it is a result.

```rust
history_table[side][from][to] += (depth * depth) as u32;                            // :1617
history_table[side][b_from][b_to] = ....saturating_sub(penalty);                    // :1628
if history_table[side][from][to] > config.history_max_threshold { ... }             // :1644
```

The table is `u32` and the malus saturates at zero. A quiet move that has been actively refuted a
dozen times is therefore indistinguishable from a quiet move that has never been searched: both
read 0. `lmr_history_bad_threshold: 500` at `lmr_reduction` (`:1843`) consequently increases the
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

**The malus is off, so this is not a type change.** `enable_history_malus` is `false`
(`config.rs:506`) and the store loop at `search_service.rs:1620` never runs. Nothing writes a
decrement today, so a signed table on its own would be a no-op: switching the malus on is part of
this item and not of 23.4, which changes only the *curves*.

#### The five steps

1. **`u32` to `i32`** in `model.rs:63`, the `SearchContext` pointer at `model.rs:136`, the
   signatures in `search_service.rs` (`minimax`, `singular_verification`, `lmr_reduction`'s
   `hist_val`), the two thresholds in `config.rs:242` with their UCI parsers, and the eight test
   literals in `move_gen_service.rs`. The `as i32` cast at `move_gen_service.rs:466` disappears.
2. **The gravity update**, as one function beside the `BAND_*` constants so both call sites share
   it:

   ```rust
   pub const MAX_HISTORY: i32 = 16_384;
   pub fn history_gravity(entry: &mut i32, bonus: i32) {
       let b = bonus.clamp(-MAX_HISTORY, MAX_HISTORY);
       *entry += b - (*entry) * b.abs() / MAX_HISTORY;
   }
   ```

   The bonus site and the malus site both call it, the malus with a negative bonus. The overflow
   rescale at `:1644` goes: gravity cannot drive `|e|` past the cap, so `history_max_threshold`
   becomes dead and its field, its UCI setter (`config.rs:1093`) and its assertion go with it.
   `tuning/parameters.json` does not carry it. **`SearchTables::age` stays** — that is 23.1's decay
   across the iterative deepening iterations, not the overflow pass.
3. **`enable_history_malus` to `true`.** Bonus and malus stay symmetric at `depth * depth`; making
   the malus steeper is 23.4.
4. **The two LMR thresholds onto the new scale**, which is the one real decision — below.
5. **Tests.** Gravity converges and never passes the cap, at both signs; a refuted quiet ends below
   zero, which must fail against today's code; `age` with negative entries;
   `test_history_credits_the_side_that_played_the_move` and
   `test_the_two_history_planes_are_independent` stay green unchanged; and the ordering check
   below.

#### Negative ranks are safe, but one comment becomes false

`MoveList::push` (`model.rs:370`) computes `rank = (rank << RANK_TIEBREAK_BITS) | (u8::MAX - order)`.
With a negative rank the low eight bits after the shift are zero, so the `|` is still an addition
and `rank * 256 + tiebreak` stays monotone. A quiet at `-MAX_HISTORY` reaches -4,194,304, while a
capture demoted by `SEE_DEMOTION` (1,024,000,000) sits near -256,000,000: negative ranks already
exist in this search, and the quiet band stays above the demoted captures. The ceiling is unmoved
at `BAND_TT << 8` = 1.28e9, inside `i32`. But `push` carries the comment *"The generator clamps its
ranks at zero, so the shift cannot lose a sign"*, and that stops being true — it has to be
rewritten with the argument above, and pinned by a test.

#### Step 4: what the thresholds become

Today the table tops out near 9000, `lmr_history_good_threshold` is 4000 (44% of that) and
`lmr_history_bad_threshold` is 500 — 5.6%, and **positive**, which is the defect. After the change
the range is [-16384, +16384] and there are two ways to go:

* **Rescale mechanically**: 4000/9000 and 500/9000 of the new cap, so roughly 7300 and 900. The
  firing rates stay close to today's, but `bad` stays positive and the defect survives in effect;
  the run would price the gravity curve and little else.
* **Set `bad` negative** — 0 or about -1000 — so the rule fires on moves that were actually
  refuted. That is what 23.3 is for, but it couples the mechanism to a guessed parameter.

**The plan took the second, and measured rather than guessed.** `scripts/measure_history_census.py`
and the `SEARCHDIAGHIST` counters were built for it, and both readings below are 300 positions from
`book_width.txt` at fixed depth 10, one process per binary, counters cumulative.

| | decisions | `good` fires | `bad` fires | entry = 0 | entry < 0 |
| :--- | ---: | ---: | ---: | ---: | ---: |
| v0.44.0, unsigned, malus off | 12,846,749 | 0.08% | 97.07% | 64.63% | impossible |
| 23.3, thresholds still 4000/500 | 12,214,681 | 0.04% | 98.99% | 11.88% | 78.68% |
| 23.3, `bad = 0` | 14,504,994 | 0.06% | 78.69% | 11.07% | 78.69% |

Three things fall out of it, and only the first was expected.

**The defect is real and it is large.** On v0.44.0 the "bad" branch fired on 97.07% of all
decisions, and 64.63 of those points were entries reading exactly zero: **two thirds of every
penalty the rule handed out went to a move the search had never seen.** At `bad = 0` the branch and
the negative population coincide to the digit — 78.69% and 78.69% — so the rule now says what its
name says.

**`lmr_history_good_threshold` is inert and always was.** It fires on 0.08% of decisions on
v0.44.0 and 0.04% here, because almost no entry reaches 4096 under either update. The "reduce
promising quiets less" half of this rule has not been running. That is a finding about the old
engine, not about 23.3, and it belongs to 23.4 — moving it would put a second behaviour change in
the run that prices this one.

**The rule is not a three-way split.** With `good` inert, `lmr_reduction` is "+1 unless the entry
clears `bad`", so the threshold sets one number: how much of the tree gets an extra ply of
reduction.

#### What the tree said, and why it cannot settle the threshold

`scripts/measure_tree_size.py` against `suprah-0.44.0`, 300 positions, depth 10, generated moves:

| Candidate | `bad` fires on | `book_width` | `book_mixed` |
| :--- | ---: | ---: | ---: |
| `bad = 0`, refuted only | 78.7% | **+12.1%** | **+11.7%** |
| `bad = 64`, the rate v0.44.0 fired at | 97.2% | +9.3% | — |
| `bad = 512` | 99.6% | **-8.9%** | — |

No tree in any run is identical to the baseline's. The ordering is the same binary in all three
rows and only the threshold moves, so the column is the price of *not* penalising a move the
search knows nothing about — and it is steep. Matching v0.44.0's firing rate does not recover its
tree either: the population behind the rate is a different one.

**Read what that instrument is, before reading the sign.** A search that reduces more searches a
smaller tree, near enough by definition, so "generated moves to fixed depth" ranks these three by
how aggressive the reduction is and not by how well aimed it is. It says the penalty on unseen
moves was buying tree; it cannot say whether the moves it was buying were worth searching.
`task.md` rule 1 exists for exactly this and sends the axis to matchplay.

The shipped value is `bad = 0`, the one that states what the item claims.

#### The gate disagreed with the tree, 2026-09-10

Both calibrations were built as their own binaries — `V0.44.1-bad0` and `V0.44.1-bad512`, the
threshold compiled in rather than set through `engine_options`, which is one global list for every
engine in a `.trn` — and played a `round_robin` against v0.44.0 and v0.43.0. 600 games, 100 per
pairing, 1s + 100ms, `openings_wide.txt`, concurrency 5. `round_robin` and not `gauntlet` because
the pairing that matters is the two calibrations against each other, and rule 2's note says a
configuration a run exists to qualify has to be the challenger or the mode has to be this one. All
four `id name` strings were checked distinct before the start.

| Pairing | score | paired Elo | 95% |
| :--- | ---: | ---: | :--- |
| `bad = 0` vs v0.44.0 | 54.0% | +27.9 | [-28, +86] |
| `bad = 512` vs v0.44.0 | 50.5% | +3.5 | [-47, +54] |
| `bad = 0` vs `bad = 512` | 50.5% | +3.5 | [-53, +60] |
| `bad = 0` vs v0.43.0 | 55.0% | +34.9 | [-9, +80] |
| `bad = 512` vs v0.43.0 | 51.0% | +6.9 | [-40, +54] |
| v0.44.0 vs v0.43.0 | 53.0% | +20.9 | [-37, +79] |

No losses on time, no duplicate games, White 60.0%, 50 openings at 12 games each.

**Every interval includes zero and none of these is an effect size** — the last row is the proof:
the same pairing was measured over 6000 fixed-N games at +1.0 Elo, and 100 games read it at +20.9.
That is what +/-110 Elo of resolution looks like, and it is why this run decides only whether
something is grossly broken.

What it does say is that nothing is: both calibrations clear the 45% gate against both
predecessors. And the variant carrying a 12% larger tree is the one that is not behind, which is
the second reading in a row telling the same story about the instrument — generated moves to fixed
depth ranks these by how aggressive the reduction is, and the games do not. `bad = 0` goes into
the 6000-game run.

The fine values and the curves stay 23.4, with its own SPSA group. Before that group runs, someone
has to check whether `spsa_tuner.py` accepts a negative range at all, because `parameters.json` has
never held one and `lmr_history_bad_threshold` is now the first parameter that wants one.

#### How it gets priced

`cargo test` green and the two deterministic readings above are done; what they said is one
section up, and the expectation written here before the run — that repairing the calibration would
move the tree back towards v0.43.0's — was wrong in sign and is left standing as the record of it.
What remains is the smoke gauntlet as challenger against v0.44.0
and v0.43.0, 100 games per pairing at 1s + 100ms with `openings_wide.txt` and the 45% gate — and
the release-candidate version collision of `skills/engine_release_procedure.md` verified *before*
the run, not after. Finally a fixed-N run of **6000 games against v0.44.0 at 1s + 150ms**,
concurrency 5, the count fixed in advance and no early stopping, about 6.25 hours, read with
`scripts/pairing_elo.py` and `scripts/match_health.py`.

#### What could go wrong

The gravity changes the distribution and not only the sign: an entry converges instead of growing
linearly, and `age` halves on top of that, which is two decays stacked. If the census shows entries
rarely approaching the cap, that is the first thing to look at — dropping `age` under gravity is a
separate run, not this one. `MAX_HISTORY = 16_384` is a stipulation and a pure scale factor as long
as the thresholds are set relative to it; it belongs in 23.4's group. And enabling the malus is a
behaviour change in its own right that cannot be separated out here, so the release commit has to
say so, or a later reader will take the run for a type refactor.

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
