# Suprah Engine Strength Enhancement Roadmap (`task.md`)

What to build next in **Suprah**, and the record of what has already been tried and failed.
Read "Negative results" before proposing anything: ten of the ideas in this document were built,
measured and put back. Three of them looked excellent on every metric except games won: the staged
`MovePicker` was correct, cut the generated moves in half and was still slower, and Internal
Iterative Reduction removed two thirds of the tree and scored 35.5% against the release it was
built on.

---

## 🧭 Start Here

### Where the engine is

| | |
| :--- | :--- |
| Released | **v0.45.0-NNUE** on this branch since 2026-09-11 — the butterfly history is signed and updated by gravity, ported from `master` v0.45.0 where it measured +9.6 Elo [+3, +16]; **that number does not transfer here**, because this branch keeps the protected `lmr_history_bad_threshold = 550` where `master` measured 0, and because it was measured on HCE. Previously **v0.44.0-NNUE** since 2026-09-10 — the butterfly history is indexed by side to move, ported from `master` v0.44.0 where it measured a null; unpriced here. Previously **v0.43.0-NNUE** since 2026-09-09 — the Null Move Pruning static-eval gate, ported from `master` v0.43.0, with both `!is_pv` guards shipped disabled. This branch **is** maintained again; the "HCE on `master` only" rule of 2026-09-02 was lifted on 2026-09-07 |
| Authority | **`task.md` on `master` is the roadmap.** This copy exists so the branch is readable on its own and drifts if it is not synced; when the two disagree, `master` is right. Branch-specific work lives in `task/search_task.md` and `task/eval_task.md` |
| Throughput | **1.86x** over v0.30.3, from three measured changes on bit-identical search trees |
| Matchplay resolution | **+/-23 Elo at 500 games**, **+/-13 at 3000**, per pairing — measured on host A. On host C with paired openings: **+/-11 at 2000**, **+/-6.5 at 6000**, the last of these confirmed by v0.39.0's run, which returned [+19, +32] around +25.6 |
| Run cost | **the rate depends on the time control, so measure it before sizing a run.** At 1s + 100ms, concurrency 5: **2.3 s per game**, a 6000-game fixed-N run in under 4 hours. At 1s + 150ms, the control the singular campaign and 23.1 use: **3.75 s per game** measured 2026-09-07, so 6000 games is **6.25 hours** and a 240-game smoke gauntlet is 15 minutes. Pricing one change per run is affordable; bundling changes to save a run is not a saving worth having |
| Blocked on | nothing. The staged `MovePicker` was finished, priced and reversed on 2026-09-03: section 5 is a negative result, and `master`'s search is v0.39.0's again |
| Runs on | **host C (ARM, 8 cores)** since 2026-08-28 — resolve `<mm>` and rebuild the binaries there; nothing from host A or host B runs or transfers. Concurrency cap here is **5**, from `floor(nproc * 0.75) - 1` |


### What has shipped

See the Engines Changelog if needed.


### The next action

**Read `master`'s `task.md` for the live backlog; this section carries only what is specific to
this branch.** Kept current as of 2026-09-11.

1. **Ported in v0.43.0-NNUE: the Null Move Pruning static-eval gate** (`master` `task.md` 20.1).
   The rule now runs only where the static evaluation is already at or above `beta`. On `master`
   it measured **-0.5 Elo, 95% [-7, +6]** over 6000 games — a null at the resolution the count was
   chosen for — against 34.7% of all null searches removed, and it ships as a throughput change.
   **On this branch the gate reads a full network score**: `calc_eval` returns before the
   lazy-evaluation block is reached, so the lazy contract that makes the gate safe on `master` is
   not needed here. `NmpPvGuard` and `RfpPvGuard` are ported and ship **disabled**, as on
   `master`. Unpriced in games on this branch.
2. **The cross-version gauntlet passed, 2026-09-10.** Challenger `suprah-0.44.0-nnue` first,
   900 games, 1s + 100ms, `openings_wide.txt`, concurrency 5: 362 wins, 338 draws, 200 losses
   overall, 59.0%. Ahead of every NNUE predecessor except the one it replaces — v0.43.0-NNUE
   24/45/31 (46.5%), v0.41.1-NNUE 36/39/25, v0.40.0-NNUE 35/47/18, v0.39.1-NNUE 46/38/16,
   v0.38.0-NNUE 43/38/19, v0.37.2-NNUE 53/31/16, v0.33.1-NNUE 42/45/13 — plus 80/14/6 against HCE
   `suprah-0.39.1` and 3/41/56 against SleepMind. Health: no losses on time, no duplicate games,
   White 64.7%, 50 distinct openings at 18 games each, because `rounds = 50` never reaches line 51
   of the pool. **This is rule 2's regression gate and nothing more** — the one pairing that is
   below 50% reads **-24.4 Elo, 95% paired [-71, +21]** over 50 pairs, which at 100 games is the
   resolution of the instrument and not a finding. The release procedure's 45% floor is held.
   v0.43.0-NNUE's own gauntlet passed the same way on 2026-09-09, at 800 games and 335/285/180.
3. **Ported in v0.42.0-NNUE: killers, history and counter moves persist across the iterative
   deepening loop** (`master` `task.md` 23.1). On `master` in HCE mode this measured **+39.4 Elo,
   95% [+29, +49]** over 2598 games. **That number does not transfer to this branch** — it was
   measured against an HCE evaluation, and move ordering interacts with the evaluation that scores
   the moves. Treat it as a reason to expect a gain here, not as this branch's effect size.
4. **`singular_margin` was retuned to 0** in v0.41.1-NNUE, ported from `master`. The other two
   singular parameters are documented negatives. The axis is closed.
5. **The negative extension**, the other half of the singular rebate, is still unmeasured.
6. **Ported in v0.44.0-NNUE: the butterfly history is `[side][from][to]`** (`master` `task.md`
   23.2). White and Black no longer share an entry. On `master` it measured **+1.0 Elo, 95%
   [-6, +7]** over 6000 games — a null, shipped there as the foundation 23.3 and 23.4 have to be
   priced on. **That number does not transfer here** and this branch has not priced it. Two gates
   were run and they disagree by seven points on the same pairing: the release-candidate smoke
   gauntlet read 53.5% against v0.43.0-NNUE and 52.0% against v0.42.0-NNUE, the cross-version
   gauntlet of item 2 read 46.5% against v0.43.0-NNUE. Two 100-game readings that straddle 50% are
   what a null looks like at this resolution; neither is an effect size. The same known limitation
   applies: the split halved the magnitudes while `lmr_history_good_threshold` and
   `lmr_history_bad_threshold` stayed put.
7. **Ported in v0.45.0-NNUE: the butterfly history is signed, updated by gravity, and the malus
   is on** (`master` `task.md` 23.3). `history_max_threshold` and the rescaling pass are gone; an
   entry converges towards `model::MAX_HISTORY` and a refuted quiet move now ends below zero,
   where before it was indistinguishable from one that had never been searched. On `master` this
   measured **+9.6 Elo, 95% [+3, +16]** over 6000 fixed-N games — the largest measured gain since
   v0.42.0, and won against a tree that is 12% *larger*.

   **Two reasons that number is not this branch's.** It was measured on HCE, and move ordering
   interacts with the evaluation that scores the moves. And `master` measured it with
   `lmr_history_bad_threshold = 0`, while this branch keeps **550**, one of the protected SPSA
   values: on the new scale 550 fires on essentially every decision, which is approximately where
   this branch already sat, so what is ported is the mechanism and not the calibration. Re-tuning
   it belongs to 23.4's SPSA group. `master`'s census also found `lmr_history_good_threshold`
   inert at 0.06% of decisions, which applies here just as much and is unmeasured on either branch.
8. **Nothing else is coming in from `master` right now.** Internal Iterative Reduction, its backlog
   item 2, was built and refused at the gate on 2026-09-09 — 35.5% against v0.43.0 in the
   published form, 42.0% at `iir_min_depth = 8`, both below 45% — and ships there behind
   `enable_iir`, default `false`. **It is not ported here, and it should not be ported here as a
   throughput idea either**: the rule fires where the Transposition Table has no move, and this
   branch scores with a network, so its own miss rate and its own trade-off would have to be
   measured from scratch. `master` moves on to what is left of backlog item 3 — 23.3, the history that
   cannot go negative; that is the next candidate for a port.

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

**The backlog lives on `master` and is not duplicated here** — the copy that used to stand in this
place listed three items that had all shipped (Quiescence Search en passant in v0.39.1-NNUE, the
singular tuning in v0.41.1-NNUE, the NNUE incremental accumulator in v0.40.0-NNUE), which is
exactly the drift the Authority row above warns about. `master`'s order as of 2026-09-09, after
Internal Iterative Reduction (22) was struck through as a negative result: the two history-table
defects (23.2, 23.3), `improving` and the Reverse Futility depth bound (21), the Null Move
reduction as a tuning group (20.3), the history curves (23.4), the Transposition Table rework
(25), Continuation History (24), ProbCut (26).

Branch-specific work that is *not* on `master`'s list lives in `task/search_task.md` and
`task/eval_task.md`. The order in which the network architecture grows is section 9.

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
| Whether the 64% White score at 1s + 100ms is worth attacking — it is the engine's, not the pool's, and it inflates pair variance in every run | open question, cheap to test against another engine pairing |
| `scripts/measure_stage0.py` still drives the engine with a fixed `sleep` instead of `scripts/uci_driver.py` | unsafe measurement; it is kept because four other scripts import its 14-position corpus |
| `MoveRawList.moves` holds 128 from/to pairs against a legal maximum of 218, and `push` drops the rest silently | latent defect, needs a position with more than 128 moves to fire |
| `truncate_bad_moves = 99` truncates an unsorted list during search, so it drops moves in generation order rather than the worst ones | latent defect, same class |
| Whether mirror-invariant move generation is worth measuring at all | open question, no prior reason to gain |
| `mm.sh` takes its opening as `opening_lines[r % num_openings]`, so a run at `rounds = 50` only ever sees the **first 50 lines** of the pool, whatever its size | measurement mechanic, established 2026-09-02 |
| The root searches **every** root move with `is_pv = true` — it runs no Principal Variation Search of its own, so this engine has one PV node per legal root move where the published formulation has one per iteration. It is why the `!is_pv` guards of 20.2 cost tree where they should be nearly free | defect or design choice, unmeasured, established 2026-09-09, section 20.2 |
| Whether the `!is_pv` guards of 20.2 are worth Elo despite costing tree — they ship disabled, and pricing them needs its own 6000-game run against v0.43.0, after the root question above | proposal, tree measured, Elo unmeasured, section 20.2 |
| The negative extension, the other half of the singular rebate, is untried | proposal, unmeasured |
| The history table is `[from][to]` with no side-to-move index, so White and Black share every entry | defect, unmeasured, section 23.2 |
| History is `u32` and its malus saturates at zero, so a refuted quiet is indistinguishable from an unseen one and `lmr_history_bad_threshold` fires on the wrong moves | defect, unmeasured, section 23.3 |
| `enable_history_malus` ships `false`, and the bonus is `depth^2` with a global 4096-entry halving pass | property, unmeasured, section 23.4 |
| The Null Move reduction is `2 + depth / 6` and is verified above depth 6, against a published `3 + depth / 3` with no verification | proposal, unmeasured, section 20.3 |
| The engine has no `improving` flag, so no rule can scale on whether the side to move is doing better than two plies ago | proposal, unmeasured, section 21.1 |
| The Late Move Pruning growth term `2 * depth^2` makes every `lmp_max_depth` from 4 upwards search the same tree | defect, pinned by `test_lmp_max_depth_is_inert_above_four`, section 21.2 |
| `rfp_max_depth` is 3 against a published 6 to 9 | proposal, unmeasured, section 21.3 |
| Internal Iterative Reduction is built on `master` and ships `false` there: refused at the gate on 2026-09-09 because **41.4% of the nodes at depth 4 and above have no table move** — measured on `master` in HCE mode, and a property of the Transposition Table rather than of the rule. Not ported here, and the miss rate under a network evaluation is unmeasured | negative result on `master`, its section 22.3 |
| There is still no Internal Iterative Deepening — the other member of the family searches the node at reduced depth first and uses its move, which is a second search and not a decrement | proposal, unmeasured |
| `Board` updates the PST sums on every move although NNUE mode never reads them | released in v0.54.2-NNUE (+0.7% / +0.9%); ported to `master` 2026-11-20 (tree unchanged there), section 6.1 |
| Local builds used no `target-cpu`, about 11% slower on x86-64 than the `x86-64-v3` binary CI ships | fixed in v0.54.2-NNUE (`scripts/build_flags.sh`, `scripts/rebuild_engines.sh`); v0.54.0-NNUE and v0.54.1-NNUE rebuilt on the x86 match host; ported to `master` 2026-11-20, section 6.2 |
| The NNUE inference speedups of branch `perf/nnue-inference` (+7.7% default, +4.6% v3, tree-identical) | released in v0.54.2-NNUE: +13.7 Elo [+8.2, +19.1] against v0.54.1-NNUE built alike; NEON check on ARM open, section 6.2 |
| There is no continuation history; killers and the counter move occupy `BAND_KILLER` instead | proposal, unmeasured, section 24 |
| The Transposition Table indexes with a 64-bit modulo, holds one entry per slot, has no generation counter and caches no static evaluation | proposal, unmeasured, section 25 |
| There is no ProbCut | proposal, unmeasured, section 26 |
| The network architecture after 512 neurons: queen-axis output buckets, 1024 neurons, a second layer, queen-axis input buckets | plan, ordered, not started, section 9 |
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

Closed on 2026-09-07, and not to be reopened: en passant generation in the Quiescence Search
(shipped v0.39.1) and the singular parameter axis (shipped v0.41.1). Both write-ups were deleted
here once released and are recoverable from git at `792af14`; what survives of the singular
campaign is the compact record in section 8.

Closed on this branch, and not on `master`: the NNUE incremental accumulator, shipped in
v0.40.0-NNUE. `master`'s copy still carries it as parked because `use_nnue` is not its
default path; here it is.

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
4. **Up to v0.54.6-NNUE (`master` v0.52.4) `nodes` and `nps` in UCI output reported *generated*
   moves, not searched nodes**, about six times the positions visited (6.0 over five positions to
   depth 11, 2026-11-21). Any change to move generation moved that number for reasons unrelated to
   speed, and `scripts/benchmark_nps.py` reads exactly that field. Since v0.54.7-NNUE (`master`
   v0.52.5) both count **visited positions**, every move played in the main and the Quiescence
   Search, as other engines count; the two units cannot be converted, and `scripts/uci_driver.py`
   (`node_unit`) makes the scripts flag a comparison across the change. Measure throughput as wall
   time to a fixed depth all the same. Since v0.54.5-NNUE (`master` v0.52.3) `nodes` counts from `go`, as UCI defines it,
   where it counted each iteration on its own before; `scripts/uci_driver.py` (`nodes_from_go`)
   puts old and new builds on the same count, and tree identity is compared without `nodes`.
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

`[Impact: High]` `[Complexity: High]`

NNUE previously ran on full recomputation per leaf (`compute_accumulator`). In `feature/nnue-evaluation` (v0.40.0-NNUE):

* **Tasks**:
    - `[x]` Thread `[NNUEAccumulator; MAX_PLY]` down `SearchService::minimax` and update incrementally in move loops (quiet, capture, castling, en passant, promotions).
    - `[x]` Validate incremental accumulator against full recomputation across move types, king bucket transitions, and deep search trees (100% bit-identical).
    - `[ ]` AVX2 / SSE4.1 / NEON intrinsics for accumulator updates and the SCReLU forward pass.
      **On x86 the autovectorizer already emits what intrinsics would** (disassembly of the
      `x86-64-v3` build on branch `perf/nnue-inference`, 2026-11-18): `apply_delta` runs each
      perspective fully unrolled in 16 ymm registers, aligned `vmovdqa` loads, `vpsubw` / `vpaddw`,
      one store into the child slot; the SCReLU loop uses `vpmaddwd` since the 16-bit product of
      6.2. Do not hand-write AVX2 for these two loops. **Open on ARM only:** check the NEON code the
      compiler emits for the same loops on the ARM host, ideally with an `nnue-prof` profile there
      first, before writing any NEON intrinsics.
    - `[ ]` Merge to `master` and set `use_nnue = true` as default in `src/config.rs` and `src/threads.rs`.

### 6.1 Skip the incremental PST in NNUE mode

`[Impact: Low]` `[Complexity: Low]` — a throughput patch; established 2026-11-17. **Built and
measured 2026-11-18 on branch `perf/nnue-inference`, released in v0.54.2-NNUE** — see "As built" below. **Port
to `master` afterwards** (constraint 3): `src/model.rs` and the two read sites in
`src/eval_service.rs` go over unchanged; on `master` `get_moves` never switches tracking off
(it sets `!network`, which is `true` there while `use_nnue` is off), so the behaviour there is the
old one and the files stay identical on both branches.

**The finding.** `Board` keeps `pst_mg` / `pst_eg` incrementally: summed in `Board::new`, saved
and updated on every `make_move` (moved piece, capture, promotion, all four castlings), stored
in `MoveInformation::old_pst_mg` / `old_pst_eg` and restored in `undo_move`. The only readers
are `EvalService::cheap_eval` and the classical evaluation in `EvalService::evaluate`, and both
sit behind the NNUE return (`if config.use_nnue && self.nnue_net.loaded`); `cheap_eval` has no
other caller. With the network active, which is the default here, the work is dead on every
move of every search. Search, move ordering, SEE and `datagen` do not read the PST.
`pawn_key` and `non_pawn_key` are **not** dead: the Correction History indexes by them.

**What it is worth.** Little: a handful of L1 lookups and adds per move, and `MoveInformation`
shrinks from 56 to 48 bytes by arithmetic. The NNUE accumulator update per move is many times
that. Expect under 1-2% wall time; the Elo is below what matchplay can resolve. The reason to do
it is that dead work leaves the hot path at no risk to the tree.

**Constraints.**
1. `UseNNUE` switches at runtime (`setoption`), so the classical evaluation must see correct
   PST sums immediately after `UseNNUE false`. A compile-time switch is ruled out.
2. `Board` does not know the `Config`; the decision has to be handed in.
3. `src/model.rs` is shared with `master` (port, never merge). Divergence in `make_move` costs
   effort on every future port.

**The design (Option A, chosen).**
- `Board` gets `track_pst: bool`. The PST lines of `make_move` and `undo_move` run only when it
  is set; the branch is constant for a whole search and predicts perfectly.
- `Board::set_pst_tracking(on)` recomputes `pst_mg` / `pst_eg` from the bitboards when tracking
  is switched on, so a stale sum can never be read.
- The search sets it at its start to `!(config.use_nnue && eval.nnue_net.loaded)`, the same
  condition `EvalService::evaluate` branches on.
- `debug_assert!(board.track_pst)` at both classical read sites.
- The change ports to `master` unchanged with the flag always `true`, keeping `src/model.rs`
  identical on both branches.

**Rejected.** Option B, removing the incremental PST on this branch and computing it on demand
in the classical evaluation: no branch and no stale state, but `src/model.rs` and
`src/eval_service.rs` would diverge from `master` in the most frequently edited function, for a
gain this small. Option C, a generic `make_move<const PST: bool>`: saves one predictable branch
at the cost of threading the parameter through the whole search.

**As built (two departures from the design above).**
- The classical read sites call `Board::pst_sums()`, which returns the incremental sums while
  tracking and recomputes them from the bitboards otherwise, **instead of a `debug_assert`**. The
  runtime switch to HCE is then correct unconditionally, also for an evaluation outside a search
  (`eval`, adjudication) on a board a network search left untracked, at no cost in NNUE mode.
- The scattered PST lines of `make_move` are gathered into one `Board::update_pst`, called under
  a single `if self.track_pst`. `undo_move` still restores the saved sums unconditionally (a copy),
  so `MoveInformation` keeps its size.

**Measured** (x86-64, 20 cores, depth 16, best of 5, against the previous branch head): **+0.7%
default build, +0.9% `x86-64-v3`**, control +0.4%, 14/14 identical trees. HCE mode: 14/14
identical trees at depth 12, -0.1%. As expected, the gain sits at the edge of what the corpus
resolves.

* **Tasks**:
    - `[x]` Test: with tracking on, after move sequences covering captures, all four castlings,
      en passant and promotions, the PST sums equal a full recomputation, also after `undo_move`.
    - `[x]` Test: with tracking off, a do/undo sequence leaves bitboards, `cached_hash`,
      `pawn_key` and `non_pawn_key` identical to the same sequence with tracking on.
    - `[x]` Test: moves made with tracking off, then tracking switched on, yield correct sums.
    - `[x]` Test: `UseNNUE` flipped from `true` to `false` mid-session gives the same classical
      evaluation as a freshly built board.
    - `[x]` Implement Option A in `src/model.rs`, `src/search_service.rs` and the classical read
      sites in `src/eval_service.rs`.
    - `[x]` Acceptance: node identity and wall time, see "Measured".
    - `[x]` Release with the rest of branch `perf/nnue-inference` (6.2): v0.54.2-NNUE.
    - `[x]` Port to `master` (2026-11-20): `src/model.rs`, `src/eval_service.rs` read sites and the
      `set_pst_tracking` line of `get_moves`; the HCE tree is unchanged there, since `master` loads
      no network and always tracks.

### 6.2 Inference micro-optimisations without intrinsics (branch `perf/nnue-inference`)

`[Impact: Medium]` `[Complexity: Low]` — built and measured 2026-11-18 on an x86-64 host with 20
cores, on top of v0.54.0-NNUE; **released in v0.54.2-NNUE (2026-11-20)**.

**Release measurement** (x86-64, 16 threads, 1s + 150ms, fixed 6,000 games each, 6 games at a time per
run): against v0.54.1-NNUE rebuilt with `x86-64-v3` **+13.7 Elo, paired 95% [+8.2, +19.1]**; against the
v0.54.1-NNUE binary built without `target-cpu` **+46.5 Elo, [+41.0, +52.0]**. Throughput on the same host at
depth 16: +4.9% / +19.8% quiet, +7.2% / +22.3% with 11 engines searching alongside (8 physical cores, so a
12-game match shares cores). The Elo per percent of speed is high at this time control; the ARM gain is
unmeasured.

**How it was measured.** `scripts/measure_throughput.py` at **depth 16**, best of 5 (7 for the
total), the default build (as `build_and_release.sh` builds) and a `-C target-cpu=x86-64-v3` build
(as CI ships) side by side, each run paired with an A/A control of the baseline against itself.
Depth 10 is useless on this host: the whole corpus takes about 460 ms and the noise swamps any
effect. The controls read -1.1% to +0.9%. Every step kept **14/14 identical trees**. The aarch64
numbers in the CHANGELOG do not compare with these.

| Step | Default | v3 | Outcome |
| :--- | ---: | ---: | :--- |
| SCReLU as `(clamp(v) * w)` in i16 times `clamp(v)` in i32 (the `pmaddwd` / `smlal` shape); the loader refuses any output weight beyond +-128, the largest for which `255 * w` fits i16 | **+3.3%** | **+1.7%** | shipped on the branch, `7b2dbec` |
| Output weights as `[[[i16; 256]; 2]; 8]`, fixed rows instead of slices; feature transformer, biases, output rows and `NNUEAccumulator` 64-byte aligned | -0.6% | +0.7% | null, kept on the branch (`134ed63`) as the precondition for the intrinsics of section 6 |
| Output bucket from `occupied.count_ones()`, piece decoding by a 32-entry table instead of `% 10` | +1.0% | +0.5% | shipped on the branch, `ee3ce06` |
| Feature row lookup of `apply_delta` forced inline: as a closure it compiled to 2-4 out-of-line calls per perspective ahead of the vector loop (0 after) | +0.4% | +0.2% | null, kept on the branch: no cost, cleaner code generation |
| **Total, v0.54.0-NNUE -> `ee3ce06`** | **+7.7%** | **+4.6%** | 13/14 positions faster in both builds |

The total exceeds the sum of the steps; the steps were each measured against their own
predecessor in separate sessions, so the direct comparison is the number to quote.

**Negative results, not to be reopened without a new idea:**

- **Release profile** `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`: **+-0.0% default,
  -0.7% v3**, inside the control. The engine is one crate and its hot paths are already
  `#[inline(always)]`; the profile only quadruples the build time. Not committed.
- **Finny refresh in one pass** — gather every changed row, then fold them into the entry in
  64-lane register tiles written to the entry and the output at once: **-0.3% default, -1.7% v3**,
  1/14 positions faster on v3. The refreshes are about 4% of the search and average 6.2 features,
  so the gather and the variable-count inner loops cost more than the saved load/store pairs.
  Reverted; its test of the largest differences was kept (`14e88a6`).

**Found on the way, done on the branch:** the `x86-64-v3` build is **about 11% faster** than the
default build on this host (6694 ms against 7645 ms for the same corpus), and `build_and_release.sh`
built without `target-cpu`, so local x86 runs measured a binary CI does not ship and understated
every SIMD gain. Now `scripts/build_flags.sh` holds the flags - `x86-64-v3` on x86-64, none on ARM
(NEON is baseline), `SUPRAH_TARGET_CPU` overrides, empty switches it off, a CPU without AVX2 stops
the build - and `build_and_release.sh` tests and builds with them. **Every engine in one run must
be built the same way**, so the opponents released before this change are rebuilt once on the
x86 match host: `MM_DIR=<mm> scripts/rebuild_engines.sh <tag> ...` builds each tag in a temporary
worktree into `<mm>/engines` under its usual name and keeps the old binary once in
`<mm>/engines/baseline-x86-64/`. A v0.54.0-NNUE rebuild searched v0.54.0-NNUE's tree (14/14). The
measurement SOP, section 5, carries the rule. **Ported to `master` 2026-11-20:** `scripts/build_flags.sh`,
its test, `scripts/rebuild_engines.sh` and the SOP paragraph went over unchanged; `build_and_release.sh`
differs between the branches, so only its `build_flags` block was ported. The HCE opponents in
`<mm>/engines` were rebuilt the same way.

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


## 8. Analysis output: principal variation and MultiPV

Two UCI options for analysis front ends, built 2026-11-20. Both ship off, and with both off the
search is v0.54.2-NNUE's tree node for node (14/14 positions of `scripts/measure_throughput.py`).

### 8.1 `TtCutPvGuard`: table cutoffs stay off the principal variation

A Transposition Table entry holds a score and one move, not the line behind it. A cutoff on a
PV node therefore wrote `[move, None]`, and the reported line ended there: at depth 16 the line
averaged **5.9 moves** over the 14 positions of `scripts/measure_pv_length.py`, with a minimum of
one. With the guard a PV node keeps the entry's move for ordering and its static evaluation but
neither returns on the entry nor narrows its window with the entry's bound, in the main search and
in the Quiescence Search; every other node cuts as before. The line then averages **20.1 moves** at
depth 16 (minimum 8). Cost: the 14-position corpus took 26.3 s instead of 17.5 s to depth 16,
measured with the cores busy, so indicative only. Unmeasured in games; Stockfish never cuts a PV
node from the table, so whether it may ship on is a 6000-game question of its own.

### 8.2 `MultiPV`: the best `n` root moves, each with its score and line

`MultiPV` 1 to 10, default 1. Line 1 is the iteration's normal search and decides `bestmove`.
Line `k` then searches the root again without the first moves of lines 1 to `k - 1`
(`SearchService::get_moves_excluding`), seeded with line `k`'s score of the previous depth, and is
written as `info depth <d> multipv <k> ...`. Since v0.54.5-NNUE all lines of a depth are sorted by
score, line 1 included, and the top one is the move played, the ponder reply and the line the next
depth starts from; before, line 1 kept rank 1 even where a further line scored a few centipawns
above it. With `MultiPV` 1 nothing changes. A line the clock interrupts ends the iteration's
further lines, so every reported line is complete; since v0.54.4-NNUE the ranks such a cut depth
leaves empty are filled with the lines of the last depth that reported, with their own depth and
without a move the cut depth already shows, so no move appears twice. Every line's `nodes` and
`time` count from `go`, so neither falls from one line to the next (both reported by the first
front end that used it; a line's own search counted only its own nodes, and its `time` was when
its best move last changed). Since v0.54.5-NNUE that holds with one line too, so `nps` is right
there as well, and the full ranking is written as soon as line 1 of a depth completes and again
after every further line, the ranks not yet searched filled from the last depth: no snapshot of
the output ever shows a move twice, and the alternatives appear as they finish. Since v0.54.6-NNUE the last
`info` before `bestmove` always names the move played: when an interrupted iteration proved a move
better than the last completed one and that move is played (`proven_better_while_interrupted`), it
is written at rank 1 with its interrupted depth and its score marked `lowerbound`, before, it went
to the log only and `bestmove` could name a move no `info` had shown. No more lines than legal moves. Each further
line costs a root search, so play is weaker above 1. Combine it with `TtCutPvGuard` for full
lines.

### 8.3 `go nodes`, and `nodes` in visited positions

Since v0.54.7-NNUE `go nodes <n>` is honoured: the search stops once the whole `go` has visited
`n` positions (`Stats::node_limit`, checked every 1024 positions where the clock is checked), and
`bestmove` is the last completed depth's move, as under a clock. With `MultiPV` the budget covers
all lines together. `nodes` and `nps` count visited positions since the same release (rule 4),
about a sixth of the generated moves counted before, so `nps` reads about 1.2M instead of 7M on
the x86 match host. `datagen --nodes` counts visited positions as well and keeps its default of
5000 (owner's decision, 2026-11-21): the same number now searches about six times as much per
move, so a data run generates about six times fewer games per hour than r5 did. r5/r5b run on their
own binary copy and are unaffected.

## 9. Network architecture roadmap (planned 2026-11-24, not started)

The order in which the network grows after the first 512-neuron net (v0.60.0-NNUE). Nothing here is
built yet. Each step is trained on the same data and hyperparameters as its baseline, with only the
named change, and measured according to `skills/matchplay_measurement_procedure.md`: first at fixed
nodes (the evaluation gain alone), then on time (the net effect after the cost in nodes per second).

**Why this order.** Width has diminishing returns at roughly linear cost: the accumulator update and
the output layer both grow with the hidden size, and so do the feature-transformer weights the
search reads at random (about 7.9 MB at 512, 15.7 MB at 1024). 256 -> 512 measured a cost of
-28.5 Elo at an unchanged tree (padded net) and a net gain of +29.5 at 1s + 150ms; each further
doubling is expected to buy less evaluation for the same cost, and needs more data. Changes that
leave the nodes per second unchanged therefore come first.

| Step | Change | Precondition | Expected | Inference cost |
| :--- | :--- | :--- | :--- | :--- |
| **9.1** | Analyse the training data: positions per piece count x queens on the board (none / any, and none / side to move only / opponent only / both) | none | sets the bucket boundaries of 9.2 instead of guessing them | none |
| **9.2** | Output buckets with a queen axis: `phase_bucket(pieces) * 2 + (queens != 0)`, split only in the middle phases (about 6-21 pieces) where both cases occur often enough; about 11-12 buckets | 9.1 | a few Elo, possibly at the resolution limit | none (one index) |
| **9.3** | 1024 hidden neurons (not 768: a +256 step is too small to resolve in games) | about 1.5-2 B positions; measure the cost first with v0.60.0's net padded to 1024 (`pad --to 1024`), as for 512 | smaller than 256 -> 512 | about 2x accumulator and output, twice the weights in cache |
| **9.4** | A second layer with one layer stack per output bucket (for example H -> 16 -> 32 -> 1, Stockfish-style), the queen axis of 9.2 selecting the stack | separate plan; enough data for the extra parameters | the largest of these, because a bucket then learns its own non-linear features instead of only re-weighting shared ones | new inference code, small dense layers |
| **9.5** | Queen axis in the input (king) buckets: 10 x 2 feature-transformer weight sets | only if 9.2 gains measurably | more than 9.2 | twice the weights and Finny table; a full accumulator rebuild at every queen exchange in the tree |

**Not planned:** 16 plain piece-count output buckets (two pieces per bucket instead of four). The
step from one bucket to eight was the large one; eight to sixteen is expected to be near zero and
thins the rare endgame buckets further. Recorded so it is not proposed again without a reason.

**Implementation notes for 9.2.** The trainer replaces `MaterialCount<8>` with its own bucket
function; the engine uses the identical function, the bucket count goes into the network format and
the size check on load. Tests first: a parity test (trainer and engine give the same bucket for a
list of FENs), and the existing mirror and colour-symmetry tests must still hold.
