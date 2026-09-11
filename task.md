# Suprah Engine Strength Enhancement Roadmap (`task.md`)

What to build next in **Suprah**, and the record of what has already been tried and failed. Read
section 19 before proposing anything: eight ideas here were built, measured and put back, and three
of them looked excellent on every metric except games won — the staged `MovePicker` was correct,
halved the generated moves and was still slower, and Internal Iterative Reduction removed two
thirds of the tree and scored 35.5% against the release it was built on. Trimmed to 500 lines on 2026-09-11; what was deleted is in git at `3d61a85`.

---

## 🧭 Start Here

### Where the engine is

| | |
| :--- | :--- |
| Released | **v0.45.0** on `master` (HCE) since 2026-09-11 — the butterfly history is signed and updated by gravity, the malus is on, and `lmr_history_bad_threshold` moved to 0 so the Late Move Reduction penalty lands on refuted quiet moves instead of unseen ones. Measured **+9.6 Elo, 95% [+3, +16]** over 6000 fixed-N games, against a tree that is 12% *larger*. Previously **v0.44.0** since 2026-09-10, the side-indexed table, a null at +1.0 [-6, +7] that shipped as this one's foundation. Porting to `feature/nnue-evaluation` is governed by `skills/nnue_porting_and_release_procedure.md` |
| Throughput | **1.86x** over v0.30.3, from three measured changes on bit-identical search trees |
| Matchplay resolution | **+/-23 Elo at 500 games**, **+/-13 at 3000**, per pairing — measured on host A. On host C with paired openings: **+/-11 at 2000**, **+/-6.5 at 6000**, the last of these confirmed by v0.39.0's run, which returned [+19, +32] around +25.6 |
| Run cost | **the rate depends on the time control, so measure it before sizing a run.** At 1s + 100ms, concurrency 5: **2.3 s per game**, a 6000-game fixed-N run in under 4 hours. At 1s + 150ms, the control the singular campaign and 23.1 use: **3.75 s per game** measured 2026-09-07, so 6000 games is **6.25 hours** and a 240-game smoke gauntlet is 15 minutes. Pricing one change per run is affordable; bundling changes to save a run is not a saving worth having |
| Blocked on | nothing. The host is busy until 23.4's run finishes, which is what section 21 waits for |
| Runs on | **host C (ARM, 8 cores)** since 2026-08-28 — resolve `<mm>` and rebuild the binaries there; nothing from host A or host B runs or transfers. Concurrency cap here is **5**, from `floor(nproc * 0.75) - 1` |


### The next action

Two items are in flight at once, 2026-09-11.

**23.4, the `lmr_history_good_threshold`, is being priced right now.** The census found the
rebate branch firing on **0.06%** of all Late Move Reduction decisions — the half of the rule that
spares promising quiet moves has never run — and nothing in the table above `+2^13` to justify the
4000 it was set to. It is 0 since `3d61a85`, which makes the rule the sign of the history entry
and fires it on 10.19%. The smoke gauntlet cleared the gate (49.5% against v0.45.0, 53.0% against
v0.44.0, a gate and not a measurement) and the 6000-game fixed-N run against v0.45.0 at 1s + 150ms
is the number that decides it. Against it: the tree is **21.1% and 18.3%** larger and **12.8% and
17.5%** slower to fixed depth 10, and the wall-time column is new — 23.3 won with a bigger tree,
but not with a slower one.

**Backlog item 4, section 21, is built and unmeasured** on branch `feature/improving-stack`
(`4586d2c`): the per-ply static-eval stack, and the Late Move Pruning growth term replaced by
`depth^2` halved when the side to move is not improving, with `lmp_max_depth` opened to 8. Its
tree measurement, its census and its gauntlet all need the host to itself and are waiting for
23.4's run to finish.

**What the rest of the audit is.** The audit of 2026-09-04 produced sections 20 to 26: seven rules
either absent here or present in a form that cannot fire at the depths this engine plays. Three
have now been through a run — 20.1 shipped Elo-neutral, 22 was refused at the gate, 23.1 to 23.3
shipped. **The remaining ones are not measured**, and nothing in them may be quoted as an effect
size. The order is the backlog table below.

**The negative extension**, the other half of the singular rebate, is still unmeasured.

The audit sections are numbered from 20 deliberately. Numbers 1 to 12 belong to write-ups deleted
on 2026-09-02, and `src/threads.rs`, `src/search_service.rs` and `src/config.rs` still carry
back-references to `task.md` 10.5, 10.6, 10.10, 10.12 and section 11 that resolve against revision
`2a280c0`. Reusing those numbers would silently redirect them.

**The bands are what paid.** Measured **+25.6 Elo** over 6000 games, 95% **[+19, +32]**, and
deterministically **21.4% less work** to fixed depth 10 over 300 pool positions. None of the three
inversions it repaired was a missing heuristic; all three were the rank scale contradicting
itself. Three lessons from that measurement, all of which still change how the next one is run:

* **The 14-position corpus cannot rank an ordering change.** Re-permuting a tie class moves single
  positions by factors in *both* directions — one variant read -1.5% on the corpus and +2.7% over
  300 pool positions. The corpus is sized for node identity, which is binary. Use
  `scripts/measure_tree_size.py` for anything that re-shapes the tree.
* **Measure the instrument before believing it.** The same ordering measured -0.5%, +1.4% and
  +2.7% depending only on whether the comparison was two branches, a packed i64 or a single i32 —
  identical trees, on the digit, all three times.
* **The throughput instrument has a noise floor wider than most differences worth arguing about.**
  `measure_throughput.py` with one binary on both sides reads a median between -0.4% and +0.0% at
  `--repeats 3`, and the candidate runs second and pays about a third of a point for it. Read the
  **corpus total**, not the mean, and treat anything under half a point as no difference.

**`openings_wide.txt` is qualified.** All 613 opening families, design effect **1.01**, effective
sample 2978 of 3000 pairs against the old pool's 1.74. White scores **64.24%** against the old
pool's 64.03%, so the colour bias is this engine's and not the pool's — the 69% and 72% seen in
two gauntlets were the first 50 lines of the file, because **`mm.sh` takes its opening as
`opening_lines[r % n]`** and a run at `rounds = 50` never reaches line 51.

### The backlog, in order

| # | Item | Where | Why this order |
| ---: | :--- | :--- | :--- |
| ~~1~~ | ~~The Null Move Pruning static-eval gate, and `!is_pv` on NMP and RFP~~ | 19 | **Done 2026-09-09.** 20.1 shipped, Elo-neutral over 6000 games, 3.0%/11.0% cheaper tree. 20.2 ships disabled: it gives the whole saving back. See 20 |
| ~~2~~ | ~~Internal Iterative Reduction~~ | 19 | **Refused 2026-09-09.** -70.4% / -64.9% generated moves and 35.5% against v0.43.0; at `iir_min_depth = 8`, -39.3% / -42.8% and 42.0%. Ships `false`. Re-ask after item 7, not before — see 22.3 |
| ~~3~~ | ~~History cannot go negative, so a refuted quiet move and an unseen one both read 0~~ | 19 | **Done 2026-09-11**, shipped in v0.45.0 at **+9.6 Elo, 95% [+3, +16]** over 6000 games. The table is signed and updated by gravity, the malus is on, and `lmr_history_bad_threshold` is 0. The tree grew 12% and the games went the other way. See 23.3 |
| 4 | `improving`, the Late Move Pruning growth term, and the Reverse Futility depth bound | 21 | Needs the per-ply static-eval stack, so it lands after 1 |
| 5 | The Null Move reduction and its verification search | 20 | Parameters, not code — a tuning group, and the gate it pairs with has landed |
| 6 | The history bonus and malus curves | 23.4 | Only after the signed table of v0.45.0, and only with its own SPSA group — which needs the tuner fixed first |
| 7 | Transposition Table: index, clusters, ageing, cached static eval | 25 | Large, and it touches the one structure every other item reads |
| 8 | Continuation History replacing killers and counter moves | 24 | Two runs minimum: the untuned rework is known to measure worse |
| 9 | ProbCut | 26 | The most speculative rule that still fires at the depth of play |
| 10 | NNUE incremental accumulator | 6 | Only worth it once `use_nnue` is the default path, and it is not |

Two proposals that used to have sections are dead and are not to be reopened. Damping the check
exemption, measured 2026-08-28: worth 4.5% of the tree and nothing in games. The staged
`MovePicker`, measured 2026-09-03: section 19.

### Everything still open, in one place

A new session can start from this table. The sections these rows once pointed at were deleted when
the document was trimmed on 2026-09-02; the write-ups are still in git, at revision `2a280c0`.

| Open | Kind |
| :--- | :--- |
| The Transposition Table stores an unproven bound at Black nodes on an empty window | defect, measured not to drift a warm table, unpriced |
| The root can hand a node an empty `alpha == beta` window | open question |
| Lazy Evaluation compares a `cheap_eval` that is missing the pawn structure on first visit | defect, measured not to drift a warm table, unpriced |
| `tt_move` is captured at node entry, and Null Move Pruning and razoring each run a recursive search before generation probes the table again — so the two can disagree about this node's table move | property, not a defect in the eager search; it broke the staged picker, see 19 |
| The bad-capture pruning decision reads `alpha`, which moves during the node — harmless while every capture is evaluated once, latent for anything that evaluates one twice | latent, only reachable from a staged picker, see 19 |
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
| ~~`lmr_history_good_threshold` fires on 0.06% of Late Move Reduction decisions~~ — the threshold is 0 since 2026-09-11 and the branch fires on 10.19%, the census having found nothing above `+2^13` to justify 4000 | defect, **in flight**: built and gated, the 6000-game run is what closes it, section 23.4 |
| `spsa_tuner.py:194` takes its step as `max(1, round(abs(value) * mutate_pct / 100))`, so a parameter sitting at 0 can never leave the neighbourhood of 0 whatever its range — both LMR history thresholds are now exactly there, and it is the same mechanism that made SPSA useless for the singular parameters | defect in the tuner, **measured by reading it** 2026-09-11, blocks 23.4's tuning group |
| The history bonus and malus are both `depth * depth` through `history_gravity`, against a published form with a steeper malus than bonus, and `MAX_HISTORY` is a stipulation | proposal, unmeasured, section 23.4, blocked on the tuner row above |
| The Null Move reduction is `2 + depth / 6` and is verified above depth 6, against a published `3 + depth / 3` with no verification | proposal, unmeasured, section 20 |
| The `improving` flag and the Late Move Pruning growth term are **built and unmeasured** on `feature/improving-stack`: the stack exists, the term is `depth^2` halved when not improving, and `lmp_max_depth` is open to 8 | built 2026-09-11, needs a tree reading, a gauntlet and a run, section 21 |
| `rfp_max_depth` is 3 against a published 6 to 9 | proposal, unmeasured, section 21.3 |
| Internal Iterative Reduction ships `false`: it was refused at the gate on 2026-09-09 because **41.4% of the nodes at depth 4 and above have no table move** here, which is a property of the Transposition Table and not of the rule. Worth exactly one re-measurement after backlog item 7, starting with the census share | negative result, section 22.3 |
| There is still no Internal Iterative Deepening — the other member of the family searches the node at reduced depth first and uses its move, which is a second search and not a decrement | proposal, unmeasured |
| There is no quiet-only move generator, which is what forced the staged picker's last stage to regenerate everything — see 19 | property, established 2026-09-04 |
| `SearchTables::age` halves the history once per iterative deepening *iteration*, because `get_moves` is one iteration; the published discipline halves once per `go` | untested variant, established 2026-09-07, section 23.1 |
| `time_check::run_time_check` drives its position list through one `EngineState`, so with persistent search tables its printed node counts are order-dependent | diagnostic only, no decision reads it, established 2026-09-07 |
| The startup benchmark `calculate_benchmark` runs a depth-3 search on the real `EngineState`, so the tables carry its history until the first `ucinewgame` | latent, harmless under any GUI that sends `ucinewgame`; the Transposition Table already had this property |

Also closed and not to be reopened: what v0.36.0, v0.37.0 and v0.37.2 are each worth, and the
scoreboard configuration that could not price them (2026-09-01). Everything else that is finished
is in section 19.

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

8. **Anything that adds a per-node buffer goes behind a pointer, or stays out of the arena.**
   A 16 KB history snapshot placed inline in the arena level grew a level from 6 KB to 22 KB and
   the arena from 1.5 MB to 5.6 MB, and cost the search **5.2% without staging a single node** —
   most of what v0.38.1 had been measured +3.5% for. Behind a pointer it costs nothing. The arena
   is walked at every node: put anything only some nodes read somewhere else. Two bytes per ply
   beside the killers, as the `improving` stack of 21.1 uses, is the shape this permits. Measured
   2026-09-03; the write-up it comes from is in git at `3d61a85`, section 5.5.

## 6. NNUE: incremental accumulator and SIMD

`[Impact: High]` `[Complexity: High]` — only worth starting once `use_nnue` is the default path.

NNUE currently runs on **full recomputation per leaf** and is off by default on `master`.

* **Tasks**:
    - `[ ]` Add `AccumulatorStack` to `Board`, updated incrementally in `do_move` / `undo_move`.
    - `[ ]` AVX2 / SSE4.1 / NEON intrinsics for accumulator updates and the SCReLU forward pass.
    - `[ ]` Validate the incremental accumulator against full recomputation.
    - `[ ]` Set `use_nnue = true` as default in `src/config.rs` and `src/threads.rs`.

## 19. Closed, deleted, and where to read them

Every write-up below was built or measured, is finished, and is **not to be reopened**. The full
text of each is in git at revision `3d61a85`, which is the last revision of this file that carried
them; the ones deleted earlier resolve against `2a280c0` (sections 1 to 12) and `792af14` (the
singular campaign and the Quiescence Search en passant). Source comments still point at some of
these numbers.

| item | what it was | why it is closed |
| :--- | :--- | :--- |
| **5. Staged `MovePicker`**, 2026-09-03 | Correct, halved the generated moves, still slower. About 2 points of the 4.1 are a toll for the staged control flow being in `minimax` at all, and 2 are the last stage regenerating the entire move list and removing the searched moves with an O(n²) scan | Ceiling, not a bug. A last stage costing *nothing* still lands near -2% against a +5% gate. `master`'s search is v0.39.0's again |
| **7. En passant** | Ranking it in the capture band made the tree 2.0% and 3.4% bigger on 600 positions and was never played. Generating it in the Quiescence Search shipped in v0.39.1 | One measured negative, one shipped |
| **8. Singular parameters**, v0.41.1 | `singular_margin` 2 → 0, **+10.7 Elo [+5, +17]**; the axis orders 0 > 1 > 2 > 3 > 6 > 4. `singular_tt_depth_margin` stays 3 and `singular_depth_reduction` stays 0, both measured negative. `margin 1 + tt_depth 2` together read +6.5, worse than either alone — the two act on the same firing rate and do not add | Closed 2026-09-07. Open only: whether 0 wins through the extension or through the multicut, whose `threshold >= beta` is also maximal at 0 |
| **20.1 / 20.2 NMP**, v0.43.0 | The `static_eval >= beta` gate ships enabled: **-0.5 Elo [-7, +6]** over 6000 games at 3.0%/11.0% fewer generated moves and 34.7% of null searches removed. Both `!is_pv` guards ship **disabled** — they give the whole saving back, because the root marks every root move `is_pv` | Shipped. The root property is in the open list and is its own question |
| **22. Internal Iterative Reduction**, 2026-09-09 | Removes 70.4% / 64.9% of the generated moves and scores **35.5%** against v0.43.0; at `iir_min_depth = 8`, 39.3% / 42.8% and 42.0%. Ships behind `enable_iir`, default `false` | Refused at the gate in four pairings. The cause is the table, not the rule: **41.4% of nodes at depth 4+ have no table move** here. Re-ask only after backlog item 7, starting with that census |
| **23.1 Persistent tables**, v0.42.0 | Killers, history and counter moves moved into `EngineState`: **+39.4 Elo [+29, +49]** over 2598 games, 5.4% less wall time. The run was stopped by hand at 2598 of 6000 | Shipped. The decay actually shipped is `age` once per *iteration*, not once per `go` — that variant is in the open list |
| **23.2 Side-indexed history**, v0.44.0 | `[side][from][to]`: **+1.0 Elo [-6, +7]** over 6000 games, tree 1.8% and 5.8% bigger | Shipped as the foundation 23.3 and 23.4 are priced on, not as a gain |
| **23.3 Signed history with gravity**, v0.45.0 | The table goes negative, `history_gravity` is the only writer, the malus is on, `lmr_history_bad_threshold` is 0: **+9.6 Elo [+3, +16]** over 6000 games against a tree **12% larger**. The census: the old "bad" branch fired on 97.07% of decisions and two thirds of those were entries reading exactly zero | Shipped. Its clearest lesson is kept in rule 1: generated moves to fixed depth rank a Late Move Reduction change by how aggressive it is, not by how well it is aimed |

## 20. Null Move Pruning: the reduction and its verification search

`[Impact: unknown]` `[Complexity: Low]` `[unmeasured]` — backlog item 5. The gate of 20.1 shipped
in v0.43.0 and the `!is_pv` guards of 20.2 ship disabled; both are in the closed record above.
This is what is left of the section.

`nmp_reduction: 2` with `nmp_dynamic_divisor: 6` gives `2 + depth / 6`: **R = 3 at the root depth
of 9 to 10 this engine reaches at the match time control.** The published adaptive form is
`3 + depth / 3` plus a capped term in how far the static evaluation exceeds `beta`:

```rust
let eval_term = ((static_eval - beta) / 200).min(3);
let reduction = 3 + depth / 3 + eval_term;
```

which is R = 6 to 7 at the same depths, before the margin term.

The engine additionally runs a **verification search** at `depth >= nmp_verification_threshold`
(6), re-searching the node at the reduced depth before taking the cut, which doubles the cost of
every deep cut. The published pairing is the other way round: the static-eval gate is what makes
the verification unnecessary, because a node already at or above `beta` is not the zugzwang case
the verification exists to catch.

The rule is live at the depth of play — rule 6 is satisfied. The census of 2026-09-08 over 40
positions at depth 10 counted 387,272 candidates by remaining depth
`3:212964, 4:94121, 5:45153, 6:19002, 7:10073, 8:4096, 9:1863`, with 66.7% of the null searches
that run producing a cutoff.

**These are parameters, not code**, so they belong in a tuning group rather than a hand-picked
default — and they are a different kind of change from the gate: the gate only removes searches,
while the reduction and the verification change what a cut is allowed to conclude.

## 21. `improving`, and the two rules whose depth bounds keep them from firing

`[Impact: unknown]` `[Complexity: Medium]` — **21.1 and 21.2 are built on branch
`feature/improving-stack` (2026-09-11, `4586d2c`), unit-tested and unmeasured.** 21.3 is not in
that build and 21.4 is not to be bundled into the run that prices it.

### 21.1 The engine had no notion of `improving`

There was no per-ply record of the static evaluation, so no rule could ask whether the side to
move is doing better than it was two plies ago. The build adds `STATIC_EVAL_UNAVAILABLE`
(`i16::MIN`) and a `[i16; MAX_PLY]` threaded through `minimax` as a per-search local, written at
every node **before any recursion**, so the slot a node reads at `ply - 2` always belongs to its
own grandparent on the current path.

The sentinel is the load-bearing part. `static_eval` itself is `0` outside
`depth > 0 && !gives_check`, and three pruning guards are pinned against never reading that `0`
(`test_the_guards_never_read_the_sentinel_static_eval`). A stack slot needs a *different* hole
value, or a node that evaluated to a dead draw is indistinguishable from one that never evaluated,
and `improving` reads "better than a dead draw" where it means "no comparison exists".

Rule 8 applies and is satisfied: two bytes per ply, next to the
killers, not in `NodeBuffers`.

**The flag is live.** Forcing `is_improving` to `false` and re-searching two positions at fixed
depth 8 shrinks the tree by **2.5%** and **3.6%** — so the stack is written and read on the path
it is supposed to be. That is not a census: what share of nodes are improving is still unknown and
wants a `search-diag` counter before the run.

### 21.2 The Late Move Pruning threshold outran the position

`lmp_base_moves + 2 * depth^2` demanded 53 quiet moves at a single node at depth 5 and 75 at depth
6. A full move list is rarely over 50 and most nodes cut long before that, so **every
`lmp_max_depth` from 4 upwards searched the same tree** — pinned, until this build, by
`test_lmp_max_depth_is_inert_above_four`, whose own conclusion was the one acted on: *"The fix is
the growth term, not the counter and not the advertised bound."*

The build replaces it with the published form, `depth^2` halved when the side to move is not
improving, and opens the cap to 8 in the same edit — the two cannot be priced apart, because with
the old growth term the wider cap is inert by construction. What the two forms admit, **computed
with the shipped `lmp_base_moves = 3`**:

| depth | `3 + 2d^2` (old, capped at d = 4) | `(3 + d^2) / (2 - improving)`, not improving | improving |
| ---: | ---: | ---: | ---: |
| 1 | 5 | 2 | 4 |
| 2 | 11 | 3 | 7 |
| 3 | 21 | 6 | 12 |
| 4 | 35 | 9 | 19 |
| 8 | inert | 33 | 67 |

An earlier revision of this table printed 3/7, 5/10, 7/15, 11/22 and 35/70. Those numbers solve to
a base of **6**, not the 3 this engine ships, and they are wrong for the formula the header gives
them. Corrected 2026-09-11. `lmp_base_moves` is SPSA-registered over `[0, 20]`, so the base is a
tuning question; the growth term is not.

**The headline of this change is the tighter budget at depths 1 to 4, not the wider cap.** At
depth 4 it admits 9 quiet moves where the old rule admitted 35. The first evidence of how much
that is came from an unrelated test: `test_check_extension_max_depth_restricts_the_tree` asserted
that frontier check extensions enlarge the tree, and under the new term they no longer do at that
position — 97,576 nodes against 97,966 with extensions off. The cause is the growth term and not
the cap: with the cap still at 4 the same inversion reads 97,737 against 98,104. With Late Move
Pruning switched off the property holds unchanged (105,372 against 101,607), so the extension is
untouched and that test now measures it in isolation.

The UCI facade, `tuning/parameters.json` and `test_lmp_max_depth_advertises_only_its_live_range`
move to 8 with the cap, and the inertness test is replaced by its opposite,
`test_lmp_max_depth_is_live_to_eight`. That is what the old test existed for.

**Still to do before it can be released:** the tree measurement on both pools, a census of how
often `improving` is true, the smoke gauntlet, and a 6000-game fixed-N run. None of them could run
while 23.4's run held the host.

### 21.3 Reverse Futility Pruning stops at depth 3

`rfp_max_depth: 3` with `rfp_margin_per_depth: 80` against a published bound of 6 to 9 plies and a
margin near 90 to 100. At depth 3 the rule demands a 240-centipawn surplus; the same margin at
depth 6 would demand 480, so widening the bound is less aggressive than it looks. Both parameters
are SPSA-registered, so this is a range change plus a tuning run rather than new code — but it
waits for 20.2's `!is_pv` guard, because at depth 6 a PV-node static cut is a different
proposition than at depth 3. **Not in the 21.1/21.2 build.**

### 21.4 What `improving` is worth beyond Late Move Pruning

The same flag conventionally scales the Reverse Futility margin and the Late Move Reduction table.
Those are separate changes with separate prices, and nothing else may be bundled into the run that
prices 21.1 plus 21.2.

## 23. The History Heuristic

Four defects were found on 2026-09-04. Three are repaired and shipped — the tables no longer die
at every iterative deepening iteration (v0.42.0), the table is indexed by side (v0.44.0), and it
is signed and updated by gravity (v0.45.0); all three are in the closed record of section 19. The
fourth is below.

### 23.4 `lmr_history_good_threshold` has never fired, and the curves are still untuned

`[In flight, 2026-09-11]` — the threshold is being priced now; the curves are not.

**The threshold was 4000 on a scale that ends at 8192.** 23.3 left this as a measured defect, not
a proposal: the "reduce promising quiet moves less" half of the Late Move Reduction fired on
**0.06%** of decisions, and 0.08% before the signed table. The census
(`scripts/measure_history_census.py`, 300 positions from `book_width.txt`, depth 10) says why —
the positive population ends at `+2^13`, and cumulatively from the top: 0.06% at or above 4096,
0.22% above 2048, 1.24% above 512, 3.39% above 128, 6.15% above 32, **10.24% above 0**.

**The value is 0, and it is not a tuning choice.** The positive side has no knee — the buckets
rise to a broad maximum around `+2^5` and fall away smoothly — so anything between 32 and 1024 is
a hand-picked point on a smooth curve, which is what 23.3 refused to do for `bad` and what the
curves below reserve for a tuning group. At `good = 0` and `bad = 0` the rule is the **sign** of
the entry and nothing else: positive reduces one ply less, zero is untouched, negative reduces one
ply more, one ply whatever the magnitude. Nothing new becomes reachable — the call site takes
`if reduction > 0`, so a decision falling to zero skips the reduced search and goes to the
full-depth Principal Variation Search, as a killer or counter move already can.

The candidate's own census confirms the threshold is all that moved: `good` reads **10.19%**
against the 10.24% predicted, while `bad` / `zero` / `negative` stay at 79.69% / 10.12% / 79.69%
against 78.69% / 11.07% / 78.69%.

**The tree hates it, more than it hated 23.3.** Against `suprah-0.45.0`, 300 positions per pool at
depth 10: `book_width` **+21.1%** generated moves and 12.8% more wall time, `book_mixed`
**+18.3%** and 17.5%. 23.3 went into its run at +12.1% and +11.7% and returned +9.6 Elo, and this
instrument has now twice ranked a Late Move Reduction change by aggressiveness rather than by aim.
But record what is different before the games are played: 23.3's cost was in generated moves, and
**this one is also in wall time at fixed depth**. At a fixed time control that is depth the engine
does not reach, and it is the first reading on this axis the games cannot simply overrule by being
better aimed.

**How it is priced.** Smoke gauntlet as challenger, 100 games per pairing at 1s + 100ms: 49.5%
against v0.45.0 and 53.0% against v0.44.0, no losses on time, no duplicates — a gate, not a
measurement. Then 6000 fixed-N games against v0.45.0 at 1s + 150ms, concurrency 5, the count fixed
in advance, read with `pairing_elo.py` and `match_health.py`.

**What could go wrong.** An entry goes positive after a single bonus — `depth * depth` at depth 4
is 16, inside the bucket carrying most of the positive mass — so "positive" means "rewarded once
and not since refuted", not "reliably good". The symmetric argument applied to `bad = 0` and the
games came out in favour, which is why the symmetric value is tried before a magnitude. And a
killer that is also historically positive now gets two rebates; that overlap was never priced and
the census cannot see it. If the run reads negative the axis is not closed — the next question is
a magnitude on the positive side, and it belongs to the group below.

**The curves, and why they cannot be tuned yet.** Bonus and malus are both `depth * depth` through
`model::history_gravity`, against a published form with a **steeper malus than bonus** so a
refuted move is unlearned faster than a good one is learned; `MAX_HISTORY = 16_384` is a
stipulation and a pure scale factor as long as the thresholds are set relative to it. Four
parameters plus the two thresholds is a tuning group. **That group cannot run today, and the
reason is not the negative range.** `spsa_tuner.py:194` takes its perturbation as
`max(1.0, round(abs(theta) * mutate_pct / 100))` and then moves the parameter by `lr_pct` percent
of it: the bounds handle negatives correctly, but the **step is proportional to the magnitude of
the value**, so a parameter sitting at 0 gets a step of 1 and an update of a fraction of it and
can never leave the neighbourhood of zero. Both LMR history thresholds are now exactly there. The
tuner needs an absolute per-parameter step first — its own item, and the same mechanism that made
SPSA useless for the singular parameters.

`tuning/parameters.json` is brought in line with the shipped defaults meanwhile (`good` 0 over
`[0, 1024]`, `bad` 0 over `[-1024, 0]`); it had registered `good` over `[2000, 8000]`, entirely
above the live population, and still carried `bad: 500`. **This changes no play** — the tuner
reads that file, the engine does not.

## 24. Continuation History

`[Impact: unknown]` `[Complexity: High]` `[unmeasured]` — backlog item 8. A rework, not an
addition, and the one item here that must not be measured without its tuning run.

Quiet moves are ordered today by three mechanisms in two bands: killers and the counter move sit
in `BAND_KILLER` with fixed bonuses, everything else in `BAND_QUIET` ranked by the butterfly
history, and `lmr_reduction` consults all three separately. The published replacement is a pair of
tables indexed by `[prev_piece][prev_to][piece][to]` — one looking back one ply, which subsumes
the counter move, and one looking back two — whose sum with the butterfly history is *the* quiet
ordering score and the single statistic the reduction reads. Killers and the counter-move table
are then deleted rather than kept alongside.

What it would need here: two `i16` tables of 12 x 64 x 12 x 64, **2.36 MB each**, as per-search
state and emphatically not in `NodeBuffers` or the per-node arena, which is walked at every node;
the move that led to each ply **plus the piece that made it**, recorded at make time because the
piece cannot be looked up later once captured; a null move clearing the previous-move slot for its
child, or the child ranks against a move that was never played. The band structure survives — the
combined score replaces what `BAND_QUIET` holds, `BAND_KILLER` disappears, and the total order and
`RANK_TIEBREAK_BITS` packing of v0.39.0 still keep the comparison a single `i32`.

**The measurement discipline is specific, and it is why this is last.** The published result is
that the *untuned* rework measured worse than what it replaced, and the gain appeared only after
the ordering constants and the reduction thresholds were tuned jointly. So: no gauntlet on the
untuned version, and a tuning group of the two curves, the follow-up weight and the four reduction
thresholds together. A two-run item at minimum — and it inherits 23.4's blocker, because that
group cannot run until `spsa_tuner.py` has an absolute step.

## 25. The Transposition Table: one slot, no ageing, no cached static evaluation, and a division in the probe

`[Impact: unknown]` `[Complexity: Medium to High]` `[unmeasured]` — `src/zobrist.rs`, backlog item
7. Four properties, two of throughput and two of search quality. **There is no version of this
where all four land in one priced change.**

**25.1 The index is a 64-bit modulo on the hottest path.** `(hash as usize) % self.table.len()` at
`zobrist.rs:204` and `:224`, and `max_zobrist_hash_entries: 50_000_000` is not a power of two, so
this is a real 64-bit division once or twice per node. The standard alternative keeps arbitrary
sizes and costs a multiply: `(((hash as u128) * (len as u128)) >> 64) as usize`. It is **not**
node-identical — it changes which positions collide — so it needs `measure_throughput.py` for the
point and `measure_tree_size.py` as the control. Self-contained, and the cleanest of the four.

**25.2 One entry per index, and no ageing.** `AtomicEntry` is a single 16-byte `{key, data}` pair
with a depth-preferred replacement policy that has **no notion of when an entry was written**: a
deep entry stored at move 12 occupies its slot for the rest of the game. The published structure
is a cluster sized to one 64-byte cache line, probed as a group, with a generation counter bumped
each search and eviction by depth discounted by age; the full 64-bit key becomes a 16-bit verifier
because the index already accounts for the rest, and that is what pays for the extra entries.
Beyond hit rate it would buy a `hashfull` that means something and somewhere to put a PV flag. The
lockless contract must survive per entry, not per cluster: invalidate the key, write the data,
restore the key is what makes a torn read detectable. This is the rewrite, and it subsumes the
layout 25.3 needs.

**25.3 The entry caches no static evaluation.** It holds the *search score* only, so every revisit
recomputes `calc_eval` from scratch (`search_service.rs:666`) although a position's static
evaluation never changes. The published entry carries the raw static evaluation beside the score
with a sentinel for "not stored". It touches an open-list item: *"Lazy Evaluation compares a
`cheap_eval` that is missing the pawn structure on first visit"* is a first-visit problem, and a
cached evaluation means later visits have no first visit to get wrong. **The constraint that
decides whether this is correct**: what is cached must be the raw evaluation before any correction
or clamping, and a lazy early return is a bound in one direction and not a value — either store
only full evaluations, or store the lazy value with the margin that produced it. Settle that
before writing code.

## 26. ProbCut

`[Impact: unknown]` `[Complexity: Medium]` `[unmeasured]` — backlog item 9, the most speculative
item here and deliberately last.

Absent. The idea: before generating the node normally, ask whether some capture already beats a
*raised* `beta` at reduced depth. If one does, the node beats its real `beta` too, and the raised
bound is what makes that inference sound. The published shape is capture-only: gate on
`!is_pv && !in_check && depth >= 5` and on `beta + margin` not running into the mate region; take
`probcut_beta = beta + margin` with a margin near 180 centipawns; generate captures only and
require `see_ge(move, probcut_beta - static_eval)` before searching anything; confirm with a
Quiescence Search at the raised null window and then a **real** search at `depth - reduction`
(reduction near 4, floored at 1), both clearing `probcut_beta`.

Two constraints specific to this engine:

* **Rule 6 is satisfied**, which is worth stating because it is the rule that killed the first
  attempt to price the Singular Extension: a minimum depth of 5 fires at plies 0 through 4 or 5 at
  the root depth of 9 to 10 this engine reaches at the match time control, not only at the root.
* **The store on a successful cut is the risk.** The published version writes a lower bound under
  this position's hash at the confirmation depth. That is defensible — the result is backed by a
  legal capture and a real search of *this* position, with no move excluded — but this engine's
  history with speculative table writes is expensive enough that the first version should return
  without storing, and the store be priced separately if at all.

`scripts/measure_tree_size.py` first: a rule that does not shrink the tree deterministically has
nothing to offer a game run.
