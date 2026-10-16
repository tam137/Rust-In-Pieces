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
| Released | **v0.46.4** on `master` (HCE) since 2026-10-14 — Reverse Futility Pruning to depth 8 and ahead of Null Move Pruning (21.3), SPRT [0, 10] H1 after 1876 games against v0.46.3. Before it **v0.46.3** (2026-10-13), Null Move Pruning at the published depth and without the verification search (section 20), SPRT [0, 10] H1 after 3308 games against v0.46.2. Before that **v0.46.2** (2026-10-12), Principal Variation Search at the root (section 28), SPRT [0, 10] H1 after 386 games against v0.46.1. Before that **v0.46.1** (2026-10-11), `improving` in the Late Move Reduction (21.4), SPRT [0, 5] and [0, 10] H1 after 9846 games against v0.46.0. Before that **v0.46.0** (2026-10-10), `improving` and the Late Move Pruning threshold of section 21, SPRT [0, 10] H1 after 4580 games and [0, 5] H1 after 6234 against v0.45.9. Before that, **v0.45.9** (2026-10-09), the end of a full source review, section 27: nine releases from v0.45.1, three of them tree-identical speed and robustness work (**22% less time to fixed depth** on identical trees) and five priced in games or positions — R17, R20, R19 and R24 by SPRT [0, 10], R21 by a targeted conversion test. Against v0.45.0, fixed-N 6000 games at 1s + 150ms: **+98.6 Elo, 95% [+92, +106]**. Porting to `feature/nnue-evaluation` is governed by `skills/nnue_porting_and_release_procedure.md` |
| Throughput | **1.86x** over v0.30.3, from three measured changes on bit-identical search trees, and about **1.24x** more from the review's tree-identical releases v0.45.2 to v0.45.4 (+12.6%, +9.5%, +0.7% on the throughput corpus) |
| Matchplay resolution | **+/-23 Elo at 500 games**, **+/-13 at 3000**, per pairing — measured on host A. On host C with paired openings: **+/-11 at 2000**, **+/-6.5 at 6000**, the last of these confirmed by v0.39.0's run, which returned [+19, +32] around +25.6 |
| Run cost | **the rate depends on the time control, so measure it before sizing a run.** At 1s + 100ms, concurrency 5: **2.3 s per game**, a 6000-game fixed-N run in under 4 hours. At 1s + 150ms, the control the singular campaign and 23.1 use: **3.75 s per game** measured 2026-09-07, so 6000 games is **6.25 hours** and a 240-game smoke gauntlet is 15 minutes. Pricing one change per run is affordable; bundling changes to save a run is not a saving worth having |
| Blocked on | nothing. The source review of 2026-10-07 finished on 2026-10-09 (section 27); its leftovers are in the table below |
| Runs on | **host C (ARM, 8 cores)** since 2026-08-28 — resolve `<mm>` and rebuild the binaries there; nothing from host A or host B runs or transfers. Concurrency cap here is **5**, from `floor(nproc * 0.75) - 1` |


### The next action

**The evaluation has never been priced, and it is audited now: section 29** (2026-10-16, census and
Texel error, no code changed). Every term dates from before the rules of this file; king danger
from piece attacks contributes 0.03 cp on average; `adjust_eval` jumps by up to 200 cp at +/-200;
the evaluation is a fifth of the search time. Its recommended order, ahead of backlog items 6 to
10: **E1** the `adjust_eval` step (small, one SPRT), then **E2** a Texel tuner - re-fitted
material and piece-square tables predict results six times better than any single change, on two
independent game sets. The rest is in 29.5.

**The source review of 2026-10-07 is finished: section 27.** It shipped v0.45.1 to v0.45.9,
each shown tree-identical, a gain in games, or a gain on a targeted position set, and it left a
short list of findings that were deferred or could not be priced — the review rows of the table
below. What it changes for the next work: the owner accepts **+5 Elo** for a change (2026-10-08),
which an SPRT [0, 10] cannot see; 27 has the [0, 5] extension procedure used for R18.

**23.4, the `lmr_history_good_threshold`, is closed: kept, null** — +2.7 Elo, 95% [-4, +9] over
6000 games, shipped in v0.45.1. See 23.4.

**Backlog item 4, 21.1 and 21.2, shipped in v0.46.0** (2026-10-10): the per-ply static-eval
stack and the Late Move Pruning threshold `(base + depth^2) / (2 - improving)` with
`lmp_max_depth` 8. SPRT [0, 10] H1 after 4580 games, [0, 5] H1 after 6234; over the 6234 games
+10.3 Elo, paired 95% [+3, +17], a stopped run and not an effect size. The Late Move Reduction
half of 21.4 followed in **v0.46.1** (2026-10-11): one ply more at a node that is not improving,
SPRT [0, 5] and [0, 10] H1 after 9846 games, +7.4 Elo [+2, +13] over 9847, a stopped run. The
Reverse Futility half of 21.4 was refused twice. 21.3 shipped in v0.46.4, see below.

**The root runs its own Principal Variation Search since v0.46.2** (2026-10-12, section 28): SPRT
[0, 10] H1 after 386 games against v0.46.1, +69 Elo [+41, +97] over 399 games, a stopped run and
not an effect size. It answers the open-items row on one PV node per root move, and it is what
20.2 and then 21.3 were waiting for. **20.2 was re-measured with it and refused on 2026-10-13**:
the `!is_pv` guards fire on the pools but cost +2.8% / +3.6% time to depth 10 and SPRT [0, 10]
H0 after 2839 games against v0.46.2, -3.4 Elo [-14, +7]; they stay off.

**Null Move Pruning searches the null child at the published depth since v0.46.3** (2026-10-13,
section 20): `depth - (3 + depth / 3)` and no verification search. SPRT [0, 10] H1 after 3308 games
against v0.46.2, +11.9 Elo [+2, +22] over 3316 games, a stopped run and not an effect size. What is
left of backlog item 5, the published reduction's term in the static evaluation, was **refused on
2026-10-15**: SPRT [0, 10] H0 after 2834 games against v0.46.4, -3.7 Elo [-14, +7] over 2847.
The null child as a non-PV node, as published, was **refused on 2026-10-16**: SPRT [0, 10] H0
after 1984 games against v0.46.4, -7.1 Elo [-19, +5] over 1998. The term at divisor 100 and a cap
of 1 was **refused on 2026-10-16** too: SPRT [0, 10] H0 after 2190 games against v0.46.4, -5.8 Elo
[-18, +6] over 2204.

**Reverse Futility Pruning cuts to depth 8 and ahead of Null Move Pruning since v0.46.4**
(2026-10-14, 21.3). SPRT [0, 10] H1 after 1876 games against v0.46.3, +17.3 Elo [+5, +30] over
1885 games, a stopped run and not an effect size. The order was the finding: behind the null move,
the wider bound only ever cut nodes whose null search had just failed. Backlog item 4 is done.
The margin, 80 against a published 90 to 100, was **refused at 90 on 2026-10-15**: SPRT [0, 10] H0
after 3490 games against v0.46.4, -2.3 Elo [-12, +7] over 3503; 100 costs 6 to 8% more time to depth 10.
The cheaper side was **refused at 70 on 2026-10-16**: SPRT [0, 10] H0 after 1424 games, -12.6 Elo [-27, +2] over 1438.

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
| ~~4~~ | ~~`improving` and the Late Move Pruning growth term~~ **done in v0.46.0**, ~~`improving` in the Late Move Reduction~~ **done in v0.46.1**, ~~the Reverse Futility depth bound~~ **done in v0.46.4** (21.3); `improving` in the RFP margin was refused twice on 2026-10-10, whole step and half step | 21 | **Done 2026-10-14.** The margin itself, 80 against a published 90 to 100: 90 refused on 2026-10-15, 70 on 2026-10-16, see 21.3 |
| ~~5~~ | ~~The Null Move reduction and its verification search~~ **done in v0.46.3**; ~~the term in `static_eval - beta` of the published reduction~~ **refused 2026-10-15**; ~~the null child as a non-PV node~~ **refused 2026-10-16**; ~~the term at divisor 100 and a cap of 1~~ **refused 2026-10-16** | 20 | **Done.** The term: SPRT [0, 10] H0 after 2834 games against v0.46.4, -3.7 Elo [-14, +7]; code on the local branch `nmp-eval-term`. At 100 / 1: SPRT [0, 10] H0 after 2190 games, -5.8 Elo [-18, +6]; code on the local branch `nmp-eval-term-100`. The non-PV null child: SPRT [0, 10] H0 after 1984 games, -7.1 Elo [-19, +5]; code on the local branch `nmp-child-pv`, see 20 |
| 6 | The history bonus and malus curves | 23.4 | Only after the signed table of v0.45.0, and only with its own SPSA group — which needs the tuner fixed first |
| 7 | Transposition Table: index, clusters, ageing, cached static eval | 25 | Large, and it touches the one structure every other item reads |
| 8 | Continuation History replacing killers and counter moves | 24 | Two runs minimum: the untuned rework is known to measure worse |
| 9 | ProbCut | 26 | The most speculative rule that still fires at the depth of play |
| 10 | NNUE incremental accumulator | 6 | Only worth it once `use_nnue` is the default path, and it is not |
| 11 | `adjust_eval`: a step of up to 200 cp at +/-200 | 29.5 E1 | The strongest Texel signal that holds on both sets, and a defect besides; small, one SPRT. Recommended first |
| 12 | A Texel tuner for material, piece-square tables and the evaluation weights | 29.5 E2 | The largest lever any instrument here has shown; after 11, which removes a non-linearity it would have to model |
| 13 | The king-centre step below phase 60, a no-pawn scale factor, R26, the lazy first visit | 29.5 E3, E4, E6 | Small, each priced on its own; E4 acts in a corner and is priced like R21 |
| 14 | King safety from piece attacks, with attack maps shared by the evaluation | 29.5 E5, E7 | Absent today; only with 12, or it is one more hand-set term |

Two proposals that used to have sections are dead and are not to be reopened. Damping the check
exemption, measured 2026-08-28: worth 4.5% of the tree and nothing in games. The staged
`MovePicker`, measured 2026-09-03: section 19.

### Everything still open, in one place

A new session can start from this table. The sections these rows once pointed at were deleted when
the document was trimmed on 2026-09-02; the write-ups are still in git, at revision `2a280c0`.

| Open | Kind |
| :--- | :--- |
| The Transposition Table stores an unproven bound at Black nodes on an empty window | defect, measured not to drift a warm table, unpriced |
| ~~The root can hand a node an empty `alpha == beta` window~~ | **answered 2026-10-12**: it happened once a pass had failed high against the aspiration window; since v0.46.2 such a pass ends there (`RootPvs` on). With `RootPvs` off the old behaviour is kept, section 28 |
| Lazy Evaluation compares a `cheap_eval` that is missing the pawn structure on first visit: **16 to 17% of all lazy returns**, and 15 to 18% of the 0.75 to 0.90% of lazy returns that land on the wrong side of the bound (29.3) | defect, measured not to drift a warm table, unpriced; 29.5 E6 |
| `tt_move` is captured at node entry, and Null Move Pruning and razoring each run a recursive search before generation probes the table again — so the two can disagree about this node's table move | property, not a defect in the eager search; it broke the staged picker, see 19 |
| The bad-capture pruning decision reads `alpha`, which moves during the node — harmless while every capture is evaluated once, latent for anything that evaluates one twice | latent, only reachable from a staged picker, see 19 |
| NNUE incremental accumulator, and making `use_nnue` the default — section 6, and not while work is HCE-only on `master` | large item, parked |
| Whether the 64% White score at 1s + 100ms is worth attacking — it is the engine's, not the pool's, and it inflates pair variance in every run. **It is about four points smaller at the longer control**: 60.41% in v0.43.0's run and 59.86% in v0.44.0's, both 6000 games at 1s + 150ms on the same pool | open question, two independent readings at 1s + 150ms, established 2026-09-10 |
| `scripts/measure_stage0.py` still drives the engine with a fixed `sleep` instead of `scripts/uci_driver.py` | unsafe measurement; it is kept because four other scripts import its 14-position corpus |
| `MoveRawList.moves` holds 128 from/to pairs against a legal maximum of 218, and `push` drops the rest silently | latent defect, needs a position with more than 128 moves to fire |
| `truncate_bad_moves = 99` truncates an unsorted list during search, so it drops moves in generation order rather than the worst ones | latent defect, same class |
| Whether mirror-invariant move generation is worth measuring at all | open question, no prior reason to gain |
| `mm.sh` takes its opening as `opening_lines[r % num_openings]`, so a run at `rounds = 50` only ever sees the **first 50 lines** of the pool, whatever its size | measurement mechanic, established 2026-09-02 |
| ~~The root searches **every** root move with `is_pv = true`~~ — one PV node per legal root move where the published formulation has one per iteration | **closed 2026-10-12**: shipped in v0.46.2, SPRT [0, 10] H1 after 386 games against v0.46.1, section 28 |
| The `!is_pv` guards of 20.2 with root PVS on: **refused 2026-10-13** against v0.46.2. Census at depth 10, 300 positions per pool (`openings_wide` / `openings_mixed`): NMP guard 0 and 1 of 300 trees unchanged, +1.0% / +5.3% generated moves, +2.1% / +4.3% time; RFP guard 57 and 40 unchanged, -0.7% / +3.9%, +1.6% / +1.6%; both +0.7% / +5.3%, +2.8% / +3.6%. Both on: SPRT [0, 10] H0 after 2839 games, -3.4 Elo [-14, +7], [0, 5] undecided at -1.05. They stay off; code on the local branch `pv-guards` | negative result, sections 20 and 28 |
| The negative extension, the other half of the singular rebate, is untried | proposal, unmeasured |
| Splitting the history by side halved the magnitude an entry reaches, and `lmr_history_good_threshold` (4000) and `lmr_history_bad_threshold` (550) were not moved with it — the tree grew 1.8% and 5.8% at fixed depth 10 | property, measured 2026-09-10, section 23.2; the re-tuning belongs to 23.4 |
| ~~`lmr_history_good_threshold` fires on 0.06% of Late Move Reduction decisions~~ — the threshold is 0 since 2026-09-11 and the branch fires on 10.19%, the census having found nothing above `+2^13` to justify 4000 | defect, **closed 2026-10-07**: kept at 0, +2.7 Elo [-4, +9] over 6000 games, section 23.4 |
| `spsa_tuner.py:194` takes its step as `max(1, round(abs(value) * mutate_pct / 100))`, so a parameter sitting at 0 can never leave the neighbourhood of 0 whatever its range — both LMR history thresholds are now exactly there, and it is the same mechanism that made SPSA useless for the singular parameters | defect in the tuner, **measured by reading it** 2026-09-11, blocks 23.4's tuning group |
| The history bonus and malus are both `depth * depth` through `history_gravity`, against a published form with a steeper malus than bonus, and `MAX_HISTORY` is a stipulation | proposal, unmeasured, section 23.4, blocked on the tuner row above |
| ~~The Null Move reduction is `2 + depth / 6` and is verified above depth 6, against a published `3 + depth / 3` with no verification~~ | **closed 2026-10-13**: shipped in v0.46.3, the null child at `depth - (3 + depth / 3)` and no verification, SPRT [0, 10] H1 after 3308 games against v0.46.2, section 20 |
| The published Null Move reduction's term `min((static_eval - beta) / 200, 3)`: **refused 2026-10-15** against v0.46.4. With Reverse Futility Pruning ahead of the null move it is at most 1 at depths 3 to 5 and only lowers a child from depth 5 on: 4.6% / 5.0% of null searches, -0.9% / -0.7% time to depth 10. SPRT [0, 10] H0 after 2834 games, -3.7 Elo [-14, +7] over 2847, [0, 5] undecided at -1.09. Code on the local branch `nmp-eval-term`. **Divisor 100 and a cap of 1 refused 2026-10-16** against v0.46.4: a shallower child on 9.3% / 9.8% of null searches, +2.6% / -1.3% time to depth 10 (`openings_wide` / `openings_mixed`); SPRT [0, 10] H0 after 2190 games, -5.8 Elo [-18, +6] over 2204, [0, 5] undecided at -1.17. Code on the local branch `nmp-eval-term-100` | negative result, section 20 |
| The null child inherits the parent's `is_pv` although it always searches a null window: the published non-PV null child was **refused 2026-10-16** against v0.46.4. Only 0.7% of null searches start at a PV node (depth 10, both pools); census +2.3% / -0.7% time to depth 10 (`openings_wide` / `openings_mixed`). SPRT [0, 10] H0 after 1984 games, -7.1 Elo [-19, +5] over 1998, [0, 5] undecided at -1.22. Code on the local branch `nmp-child-pv` | negative result, section 20 |
| ~~The `improving` flag and the Late Move Pruning growth term~~ | **closed 2026-10-10**: shipped in v0.46.0, SPRT [0, 10] H1 after 4580 games, [0, 5] H1 after 6234, section 21 |
| ~~`improving` is not used by the Late Move Reduction table~~ | **closed 2026-10-11**: shipped in v0.46.1, SPRT [0, 5] and [0, 10] H1 after 9846 games against v0.46.0, section 21.4 |
| `improving` in the Reverse Futility margin: **refused twice on 2026-10-10** against v0.46.0 - the published whole step (`margin * (depth - improving)`, 0 at depth 1) SPRT [0, 10] H0 after 1036 games, -21.8 Elo [-40, -4]; half a step (40/120/200) [0, 10] H0 after 4058 games, -0.9 Elo [-10, +8]. Code on the local branches `rfp-improving-margin` and `rfp-improving-half-step`. The step from depth 2 only and a tuned margin are untried | negative result, section 21.4 |
| ~~`rfp_max_depth` is 3 against a published 6 to 9~~ | **closed 2026-10-14**: shipped in v0.46.4 with the rule ahead of Null Move Pruning, SPRT [0, 10] H1 after 1876 games against v0.46.3, section 21.3 |
| `rfp_margin_per_depth` 80 against a published 90 to 100: **refused 2026-10-15** at 90 against v0.46.4. Census at depth 10 against 80, `openings_wide` / `openings_mixed`: 70 -6.3% / -7.2% time, 90 +0.4% / +1.0%, 100 +7.8% / +6.0%, 120 +17.3% / +12.3%; 100 was not played (more than 5% more time on both pools). 90: SPRT [0, 10] H0 after 3490 games, -2.3 Elo [-12, +7] over 3503, [0, 5] undecided at -1.07. Code on the local branch `rfp-margin`. **70 refused 2026-10-16**: census -5.4% / -6.5% time, SPRT [0, 10] H0 after 1424 games, -12.6 Elo [-27, +2] over 1438, code on the local branch `rfp-margin-70`. 100 in games is untried | negative result, section 21.3 |
| Internal Iterative Reduction ships `false`: it was refused at the gate on 2026-09-09 because **41.4% of the nodes at depth 4 and above have no table move** here, which is a property of the Transposition Table and not of the rule. Worth exactly one re-measurement after backlog item 7, starting with the census share | negative result, section 22.3 |
| There is still no Internal Iterative Deepening — the other member of the family searches the node at reduced depth first and uses its move, which is a second search and not a decrement | proposal, unmeasured |
| There is no quiet-only move generator, which is what forced the staged picker's last stage to regenerate everything — see 19 | property, established 2026-09-04 |
| `SearchTables::age` halves the history once per iterative deepening *iteration*, because `get_moves` is one iteration; the published discipline halves once per `go` | untested variant, established 2026-09-07, section 23.1 |
| `time_check::run_time_check` drives its position list through one `EngineState`, so with persistent search tables its printed node counts are order-dependent | diagnostic only, no decision reads it, established 2026-09-07 |
| The startup benchmark `calculate_benchmark` runs a depth-3 search on the real `EngineState`, so the tables carry its history until the first `ucinewgame` | latent, harmless under any GUI that sends `ucinewgame`; the Transposition Table already had this property |
| Review leftovers, deferred: R4 `pv_nodes` behind a mutex and a SipHash map at every interior node (0.3% to 2%, 18 call sites), R7 a table move that is a promotion or en passant never gets the table band, R16c a `stop` that arrives before its `go` is lost | deferred, section 27 |
| Review leftovers, unpriced: R22 the lazy margin does not bound the positional terms (a tuning question; measured 2026-10-16, 29.3), R23 table cutoffs at the root's children can hide a repetition, R25 no quiet queen promotion in the Quiescence Search, R26 the pawn hash caches a blockade by pieces (measured: 0.23% of hits, 22 cp, 29.3), R27 the null child's counter-move entry and an abort sentinel that looks like a cutoff, R28 two steps in the evaluation (see the end of 29.5), R29 no fifty-move rule and the start position missing from the repetition history, R30 rook and bishop promotions off by default | listed, section 27; R22 and R26 measured in 29 |
| The evaluation, audited 2026-10-16: king danger 0.03 cp on average, `adjust_eval` a step of up to 200 cp, the positional cap load-bearing (+3716 to +4210 Texel error without it), `king_in_check_malus` and `king_in_double_check_malus` unreachable in play, material and piece-square tables far from the data | audit, section 29; candidates E1 to E8 |
| R18, the time allocation — a soft limit at 60% of the budget and the budget bounded below the clock — is **not shipped**: SPRT [0, 10] H0 after 9050 games, [0, 5] undecided at 9244, +2.3 Elo [-3, +8]. The code is on the local branch `review/r18-time-management`; the clock bound alone, the 10 ms cliff, is untested | negative result, section 27 |

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
| **20.1 / 20.2 NMP**, v0.43.0 | The `static_eval >= beta` gate ships enabled: **-0.5 Elo [-7, +6]** over 6000 games at 3.0%/11.0% fewer generated moves and 34.7% of null searches removed. Both `!is_pv` guards ship **disabled** — they gave the whole saving back, because the root marked every root move `is_pv`; re-measured with root PVS (v0.46.2) and refused again on 2026-10-13, see the open list | Shipped. The root property was answered in v0.46.2, section 28 |
| **22. Internal Iterative Reduction**, 2026-09-09 | Removes 70.4% / 64.9% of the generated moves and scores **35.5%** against v0.43.0; at `iir_min_depth = 8`, 39.3% / 42.8% and 42.0%. Ships behind `enable_iir`, default `false` | Refused at the gate in four pairings. The cause is the table, not the rule: **41.4% of nodes at depth 4+ have no table move** here. Re-ask only after backlog item 7, starting with that census |
| **23.1 Persistent tables**, v0.42.0 | Killers, history and counter moves moved into `EngineState`: **+39.4 Elo [+29, +49]** over 2598 games, 5.4% less wall time. The run was stopped by hand at 2598 of 6000 | Shipped. The decay actually shipped is `age` once per *iteration*, not once per `go` — that variant is in the open list |
| **23.2 Side-indexed history**, v0.44.0 | `[side][from][to]`: **+1.0 Elo [-6, +7]** over 6000 games, tree 1.8% and 5.8% bigger | Shipped as the foundation 23.3 and 23.4 are priced on, not as a gain |
| **23.3 Signed history with gravity**, v0.45.0 | The table goes negative, `history_gravity` is the only writer, the malus is on, `lmr_history_bad_threshold` is 0: **+9.6 Elo [+3, +16]** over 6000 games against a tree **12% larger**. The census: the old "bad" branch fired on 97.07% of decisions and two thirds of those were entries reading exactly zero | Shipped. Its clearest lesson is kept in rule 1: generated moves to fixed depth rank a Late Move Reduction change by how aggressive it is, not by how well it is aimed |

## 20. Null Move Pruning: the reduction and its verification search

**Shipped in v0.46.3 (2026-10-13)**, backlog item 5: the null child at the published depth and no
verification search. The gate of 20.1 shipped in v0.43.0 and the `!is_pv` guards of 20.2 ship
disabled; both are in the closed record above. The term in `static_eval - beta` of the published
reduction was refused on 2026-10-15, at divisor 100 and a cap of 1 on 2026-10-16, and the null
child as a non-PV node on 2026-10-16 (end of this section).

**This section used to compare two conventions.** It read "R = 3 at the root depth of 9 to 10
against a published 6 to 7". This engine searches the null child at
`depth - 1 - (nmp_reduction + depth / nmp_dynamic_divisor)`, so its `reduction` excludes the
move's own ply; the published `depth - R` includes it. Until v0.46.2 the effective reduction was
`3 + depth / 6` against the published `3 + depth / 3`, which is `nmp_reduction` 2 with divisor 3
here, not 3 with 3. Child depth by remaining depth (`search_service::nmp_child_depth`, pinned by
`test_nmp_child_depth_follows_the_published_form`):

| depth | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| until v0.46.2 (`2`, `/6`) | 0 | 1 | 2 | 2 | 3 | 4 | 5 | 6 |
| v0.46.3, published (`2`, `/3`) | 0 | 0 | 1 | 1 | 2 | 3 | 3 | 4 |
| literal `3`, `/3` (not built) | 0 | 0 | 0 | 0 | 1 | 2 | 2 | 3 |

Depth 3, 57% of the candidates, is unchanged: its child was already the Quiescence Search.

Until v0.46.2 the engine also ran a **verification search** at `depth >= nmp_verification_threshold`
(6), re-searching the node at the null child's depth before taking the cut. The published pairing
runs none: a node already at or above `beta`, which is all the gate of 20.1 lets through, is not
the zugzwang case the verification exists to catch, and a side without non-pawn material never
reaches the rule. Since v0.46.3 the verification sits behind `Config::nmp_verification` (UCI
`NmpVerification`), off; the threshold keeps its 6 for when it is on.

**What was measured** (2026-10-13, host C):
* **Census** (`search-diag`, 300 pool positions per pool, depth 10, v0.46.2's settings): null-move
  candidates by remaining depth 3: 57%, 4: 22%, 5: 12%, 6 to 9: 9%. The verification ran 67,193 /
  61,452 times on `openings_wide` / `openings_mixed` and refused the cut **40 / 34 times (0.06%)**.
  Null searches that cut: 60.2% / 58.7% before, 61.7% / 59.7% after.
* **Tree** against v0.46.2, depth 10, 0 of 300 identical everywhere (protocol only, rule 1): the
  divisor alone **-14.9% / -14.1%** time, the verification off alone -2.6% / **+3.9%**, both
  **-16.4% / -13.2%** (`openings_wide` / `openings_mixed`). Dropping the verification alone grew the tree on one
  pool; only together with the deeper reduction does it save work.
* **Identity:** the release with `NmpDynamicDivisor=6 NmpVerification=true` is v0.46.2, and its
  defaults are v0.46.2 with `NmpDynamicDivisor=3 NmpVerificationThreshold=99` - 300/300 identical
  trees on both pools, both checks.
* **Gate:** SPRT at 1s + 150ms on `openings_wide.txt`, concurrency 4, run under [0, 5] with the
  [0, 10] monitor. **[0, 10] H1 after 3308 games** (LLR +2.95, +12.3 Elo at the stop); [0, 5]
  undecided at +1.9. Over the 3316 games that finished **+11.9 Elo, paired 95% [+2, +22]** - a
  stopped run, not an effect size (rule 7). No losses on time, White 53.1%, design effect 1.00.
* **Smoke gauntlet**, 1s + 100ms, 100 games per pairing: 46.5% against v0.46.2, 68.0% against
  v0.46.1, 92.5% against v0.34.0, no losses on time. A gate, not a measurement.

**The published reduction's term in `static_eval - beta`: refused on 2026-10-15.** The published
reduction also grows with `min((static_eval - beta) / 200, 3)`; under the gate of 20.1 that term is
never negative here, and the pawn is 100, so 200 is two pawns. The candidate took it behind two UCI
spin options, `NmpEvalDivisor` (200) and `NmpEvalMaxReduction` (3); a cap of 0 is v0.46.4.
**Reverse Futility Pruning caps it.** Since v0.46.4 that rule runs ahead of the null move on a
superset of its conditions and cuts every surplus of `80 * depth` up to depth 8, so the term is at
most 1 at depths 3 to 5, 2 at 6 and 7, 3 from 8, and at depths 3 and 4 the null child is the
Quiescence Search with or without it:

| depth | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| v0.46.4 | 0 | 0 | 1 | 1 | 2 | 3 | 3 | 4 |
| largest term after RFP | 1 | 1 | 1 | 2 | 2 | 3 | 3 | 3 |
| shallowest child with the term | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1 |

* **Identity:** the candidate with `NmpEvalMaxReduction=0` is v0.46.4, 300/300 identical trees on
  both pools.
* **Tree** against that, depth 10, 300 positions (protocol only, rule 1): **-0.9% / -0.7%** time,
  -1.9% / -1.0% generated moves (`openings_wide` / `openings_mixed`), 0 of 300 identical.
* **`search-diag`**: a term above 0 on 15.0% / 16.6% of null searches, almost all of them 1 (2 and
  3 on about 1,200 and 760); a shallower child on **4.6% / 5.0%**, all from depth 5 on, about a
  fifth of the null searches there. The share of null searches that cut is unchanged.
* **Gate:** SPRT at 1s + 150ms on `openings_wide.txt`, concurrency 4, run under [0, 5] with the
  [0, 10] monitor. **[0, 10] H0 after 2834 games** (LLR -3.01, -3.6 Elo at the stop); [0, 5]
  undecided at -1.09. Over the 2847 games that finished **-3.7 Elo, paired 95% [-14, +7]**. The
  reading went from -10 after 314 games to +5.4 after 1678 and back down. No losses on time, White
  52.8%, design effect 1.01.

The code is on the local branch `nmp-eval-term`.

**The term at divisor 100 and a cap of 1: refused on 2026-10-16.** One pawn of surplus takes one
ply off the null child, and never more than one. With Reverse Futility Pruning ahead, the shallowest
child by remaining depth 3 to 10 is 0, 0, 0, 0, 1, 2, 2, 3 (200 / 3: 0 everywhere up to depth 9).
The candidate is the code of `nmp-eval-term` with the defaults moved to 100 and 1.

* **Identity:** the candidate with `NmpEvalMaxReduction=0` is v0.46.4, 300/300 identical trees on
  both pools.
* **Tree** against that, depth 10, 300 positions (protocol only, rule 1): **+2.6% / -1.3%** time,
  +2.8% / -2.9% generated moves (`openings_wide` / `openings_mixed`; plus is less work), 0 of 300
  identical. Neither pre-registered stop fired: more time on both pools, or a shallower child on no
  more than 5.0% of null searches on both.
* **`search-diag`**: a term of 1 on **41.1% / 43.5%** of null searches (200 / 3: 15.0% / 16.6%), a
  shallower child on **9.3% / 9.8%** (200 / 3: 4.6% / 5.0%), only from depth 5; the null cut rate
  unchanged (56.1 to 56.3%, 53.1 to 53.2%).
* **Gate:** SPRT at 1s + 150ms on `openings_wide.txt`, concurrency 4, run under [0, 5] with the
  [0, 10] monitor. **[0, 10] H0 after 2190 games** (LLR -3.03, -5.9 Elo at the stop); [0, 5]
  undecided at -1.17. Over the 2204 games that finished **-5.8 Elo, paired 95% [-18, +6]**; the
  reading went from +7 after 900 to 1100 games down to -5 after 2000. No losses on time, White
  52.3%.

Twice the lever of 200 / 3 bought nothing either: the term is refused in both readings. The code
is on the local branch `nmp-eval-term-100`.

**The null child as a non-PV node: refused on 2026-10-16.** The null search hands the parent's
`is_pv` to its child although it always searches a null window at `-beta` and its score never
becomes a principal variation. Started at a PV node (the guard of 20.2 ships off), the child and
its first move at every ply run without razoring, bad-capture pruning, Late Move Pruning and
Futility Pruning, and with one ply less Late Move Reduction. The published null child is a non-PV
node. The candidate took it behind a UCI check option, `NmpChildPv` (default `false`); `true` is
v0.46.4. On Kiwipete no null search starts at a PV node up to depth 9, so the tests run on a quiet
Queen's Gambit Declined middlegame.

* **Identity:** the candidate with `NmpChildPv=true` is v0.46.4, 300/300 identical trees on both
  pools.
* **Tree** against that, depth 10, 300 positions (protocol only, rule 1): **+2.3% / -0.7%** time,
  +3.3% / -0.7% generated moves (`openings_wide` / `openings_mixed`), 50 and 53 of 300 identical.
  The pre-registered stop (more time on both pools) did not fire.
* **`search-diag`**: null searches at a PV node are **0.7%** of all null searches (about 9,400 per
  pool, 4,600 to 4,800 of them from depth 5 up, where the child is not the Quiescence Search); 3 to
  4% of them cut, and from depth 5 up the cuts rise from 46 to 70 and from 52 to 64.
* **Gate:** SPRT at 1s + 150ms on `openings_wide.txt`, concurrency 4, run under [0, 5] with the
  [0, 10] monitor. **[0, 10] H0 after 1984 games** (LLR -2.98, -6.8 Elo at the stop); [0, 5]
  undecided at -1.22. Over the 1998 games that finished **-7.1 Elo, paired 95% [-19, +5]**; the
  reading stayed between -4 and -15 from 600 games on. No losses on time, White 55.0%, design
  effect 1.00.

The null child keeps the parent's `is_pv`. The code is on the local branch `nmp-child-pv`. The rule
touches under 1% of null searches; what moved the tree is reordering through the Transposition
Table and the history, not the pruning inside those few searches.

## 21. `improving`, and the two rules whose depth bounds keep them from firing

`[Impact: Medium]` `[Complexity: Medium]` — **21.1 and 21.2 shipped in v0.46.0 (2026-10-10), the
LMR half of 21.4 in v0.46.1 (2026-10-11), 21.3 in v0.46.4 (2026-10-14).** The RFP half of 21.4 was refused at the
gate twice on 2026-10-10, in its published form and with half the step, see 21.4.

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

### 21.5 What 21.1 and 21.2 measured, v0.46.0

Built on `3d61a85` and cherry-picked onto v0.45.9 without conflicts. One defect was found on the
way and fixed with a test that failed first: razoring, the Null Move verification and the singular
verification re-enter a node at its own ply, so they overwrote the node's slot - razoring with the
sentinel, the singular verification with an evaluation taken under its own narrow window, which
the lazy evaluation can answer differently. Each re-entry now restores the slot.

* **Census** (`search-diag`, 300 pool positions, depth 10): the comparison exists at 93.7% of the
  nodes with a static evaluation, and `improving` is true at **66.1%** of those, 48% to 70% by
  depth. The old threshold would have made **0.1%** of the deletions the new one makes: the rule
  this replaced was all but dead, so v0.46.0 is in effect a new pruning rule, not a retuned one.
* **Tree** against v0.45.9, depth 10: 0 of 300 identical on both pools, **12.8%** and **10.2%**
  less time. Recorded, not a gate (rule 1).
* **Gate:** SPRT at 1s + 150ms on `openings_wide.txt`, run under [0, 5] with [0, 10] watched on the
  same games. [0, 10] H1 after 4580 games (stopping score 51.44%), [0, 5] H1 after 6234 (51.48%).
  Over the 6234 games **+10.3 Elo, paired 95% [+3, +17]** - a stopped run, so an upper reading and
  not an effect size (rule 7). No losses on time, White 60.37%, design effect 1.00.
* **Smoke gauntlet**, 1s + 100ms, 100 games per pairing: 47.5% against v0.45.9, 55.5% against
  v0.45.8, 81.5% against v0.34.0.

### 21.3 Reverse Futility Pruning stops at depth 3 — shipped in v0.46.4

`rfp_max_depth: 3` with `rfp_margin_per_depth: 80` against a published bound of 6 to 9 plies and a
margin near 90 to 100. At depth 3 the rule demands a 240-centipawn surplus; the same margin at
depth 6 would demand 480, so widening the bound is less aggressive than it looks. Both parameters
are SPSA-registered, so this is a range change plus a tuning run rather than new code. It waited
for 20.2's `!is_pv` guard, because at depth 6 a PV-node static cut is a different proposition than
at depth 3; that guard was refused on 2026-10-13 (open list), so whether a 21.3 candidate carries
it is a decision for 21.3's own run. **Not in the 21.1/21.2 build.**

**What reading the code found: the order.** Null Move Pruning ran *ahead* of this rule, where the
published formulation has it behind. From `depth >= nmp_depth_threshold` (3) the null move tries
every node whose static evaluation is at or above `beta`, and this rule asks the same plus a
surplus of its margin. So from depth 3 on it only ever saw a node whose null search had failed, or
one that never runs one (the null child, the singular verification): widening the bound alone cut
exactly the nodes where a null search had just found a threat, and paid for every other null
search. The order decides no cut - a node the rule may cut is cut in either order, by the null
search at `beta` or by this rule behind it at `static_eval - margin` - so ahead of the null move
the rule only spares that null search. The candidate therefore took both: `rfp_max_depth` 3 -> 8
and the rule ahead of Null Move Pruning behind `RfpBeforeNmp` (default on; off is v0.46.3's
order). No SPSA run: the tuner cannot resolve a few Elo at 1s + 150ms (section 19, item 8).

* **Census** (300 pool positions, depth 10, against v0.46.3; less time / fewer generated moves,
  `openings_wide` and `openings_mixed`): the bound alone -2.8% / -3.1% and -0.3% / -0.2%, 19 and
  16 of 300 trees unchanged; the order alone (bound 3) +0.1% / +0.2% and -1.5% / -1.6%; both
  **-2.6% / -3.6%** and **-1.5% / -1.8%**, 0 of 300 unchanged. Recorded, not a gate (rule 1).
* **`search-diag`** (`openings_wide`): behind the null move the bound of 8 cuts 7,030 nodes at
  depths 4 to 8, **every one of them after a failed null search**; ahead of it, 88,000 (depth 4:
  60,588, 5: 21,934, 6: 3,882, 7: 1,499, 8: 97), and depth 3 goes from 50,367 cuts to 268,876.
  Null searches 1,811,091 -> 1,423,853 (-21%). Cuts at PV nodes 748 -> 783; the PV guard stays off.
* **Identity:** the release with `RfpBeforeNmp=false` is v0.46.3 with `RfpMaxDepth=8`, and with
  `RfpMaxDepth=3` as well v0.46.3 itself: 300/300 identical trees on both pools, all four.
* **Gate:** SPRT at 1s + 150ms on `openings_wide.txt`, concurrency 4, run under [0, 5] with
  [0, 10] watched on the same games. [0, 10] **H1 after 1876 games** (LLR +2.95). Over the 1885
  games **+17.3 Elo, paired 95% [+5, +30]** - a stopped run, so an upper reading and not an
  effect size (rule 7); [0, 5] undecided at +1.74. No losses on time, White 52.7%, design effect
  1.02. The reading wandered from -10 after 384 games to +19 after 1280.
* **Smoke gauntlet**, 1s + 100ms, 100 games per pairing: 53.5% against v0.46.3, 52.5% against
  v0.46.2, 86.0% against v0.34.0.

`test_a_null_move_into_stalemate_does_not_leave_the_node_drawn` runs with this rule off: at depth
7 it now cuts that node on its extra queen before the null move is tried.

**The margin: 90 refused on 2026-10-15.** `rfp_margin_per_depth` has been 80 since v0.28.4, when
the rule stopped at depth 3, and was never measured; the published margin is 90 to 100 per ply,
and the pawn is 100 here. Over 98% of the rule's cuts are at depths 1 to 3, so the margin acts on
the bulk of the tree rather than on a few deep nodes. `RfpMarginPerDepth` was already parsed, so
the census ran on v0.46.4 itself.

* **Tree** against 80, depth 10, 300 positions (protocol only, rule 1), `openings_wide` /
  `openings_mixed`: 70 **-6.3% / -7.2%** time, 90 **+0.4% / +1.0%** (-1.0% / -2.6% generated
  moves), 100 **+7.8% / +6.0%**, 120 +17.3% / +12.3%; 0 of 300 identical everywhere. 100, the
  value first chosen, exceeded the pre-registered +5% on both pools and was not played; 90 was.
* **`search-diag`**, 80 -> 90: Reverse Futility cuts -9.2% / -8.4%, about half of that at depth 1;
  null searches +1.8% / +4.3%, null cuts +3.3% / +6.9% - part of what the rule no longer cuts
  statically, the null move cuts instead.
* **Identity:** the candidate with `RfpMarginPerDepth=80` is v0.46.4, 300/300 on both pools.
* **Gate:** SPRT at 1s + 150ms on `openings_wide.txt`, concurrency 4, run under [0, 5] with the
  [0, 10] monitor. **[0, 10] H0 after 3490 games** (LLR -3.16, -2.1 Elo at the stop); [0, 5]
  undecided at -1.07. Over the 3503 games that finished **-2.3 Elo, paired 95% [-12, +7]**; the
  reading stayed between -4 and +1.3 from 1100 games on. No losses on time, White 54.4%, design
  effect 1.00.

The margin stays 80. The code is on the local branch `rfp-margin` (default 90, `RfpMarginPerDepth`
advertised, `tuning/parameters.json` with it).

**The margin: 70 refused on 2026-10-16.** The cheaper side, against the published direction: 90
measured flat, which is no evidence that less accuracy pays, and the run says it does not.

* **Tree** against 80, depth 10, 300 positions, `openings_wide` / `openings_mixed`: 70 **-5.4% /
  -6.5%** time (-8.7% / -11.8% generated moves); 75 +0.2% / -0.8%, 60 -9.2% / -14.4%.
* **`search-diag`**, 80 -> 70: Reverse Futility cuts +4.3% / +2.2%, most of the growth at depths
  3 to 5; null searches -8.0% / -8.2%, null cuts -10.0% / -10.2%.
* **Identity:** the candidate with `RfpMarginPerDepth=80` is v0.46.4, and v0.46.4 with 70 is the
  candidate - 300/300 on both pools each.
* **Gate:** as for 90. **[0, 10] H0 after 1424 games** (LLR -3.02, -12.0 Elo at the stop); [0, 5]
  undecided at -1.36. Over the 1438 games that finished **-12.6 Elo, paired 95% [-27, +2]**; the
  reading was below -10 throughout, at worst -32 after 550 games. No losses on time, White 53.7%,
  design effect 1.00.

The margin stays 80, between a flat 90 and a losing 70. The code is on the local branch
`rfp-margin-70` (default 70, `RfpMarginPerDepth` advertised). Untried in games: 100, over the +5%
time rule of 2026-10-15.

**On the NNUE branch: v0.46.4-NNUE** (2026-10-14). Ported onto v0.46.2-NNUE, where
`rfp_max_depth` was a protected parameter at 3 - the start value of the one SPSA run over it,
never left - and moved to 8 with the owner's approval; the margin stays 80. With `RfpBeforeNmp`
off and the bound at 3 the port is v0.46.2-NNUE node for node (300/300 on both pools). Tree to
depth 10: 3.7% / 5.1% less time, 29% fewer null searches. SPRT [0, 10] **H1 after 2190 games**
against v0.46.2-NNUE; over 2202 games +13.3 Elo, paired 95% [+3, +23] - a stopped run, not an
effect size. Smoke gauntlet 62.0% / 64.5% / 88.5% against v0.46.2- / v0.45.9- / v0.34.0-NNUE.
The NNUE test helper `search_node` handed the node an empty accumulator until then.

### 21.4 What `improving` is worth beyond Late Move Pruning

The same flag conventionally scales the Reverse Futility margin and the Late Move Reduction table.
Those are separate changes with separate prices, and nothing else may be bundled into the run that
prices 21.1 plus 21.2.

**The Reverse Futility half, published form: refused 2026-10-10.** The margin
`rfp_margin_per_depth * (depth - improving)` keeps v0.46.0's 80/160/240 when not improving and
takes it to **0**/80/160 when improving, so an improving node at depth 1 cuts on
`static_eval >= beta` alone. The code is on the local branch `rfp-improving-margin` (`8344c76`):
the helper `rfp_margin`, a node-level test at ply 3, and `search-diag` counters for the cuts.

* **Census** (300 pool positions, depth 10): **29%** of all Reverse Futility cuts are ones the
  v0.46.0 margin would not have made, +39% / +48% / +77% at depths 1 / 2 / 3; `improving` at 66% /
  66% / 45% of the candidates.
* **Tree** against v0.46.0, depth 10: 0 of 300 identical on both pools, **21.9%** and **21.4%**
  less time. Recorded, not a gate (rule 1) - and the games went the other way.
* **Gate:** SPRT at 1s + 150ms on `openings_wide.txt`, concurrency 4, run under [0, 5] with
  [0, 10] watched on the same games. [0, 10] **H0 after 1036 games** (LLR -3.22, stopping score
  46.86%); [0, 5] undecided at -1.46. Over the 1038 games **-21.8 Elo, paired 95% [-40, -4]** -
  a stopped run, so not an effect size (rule 7), but the interval excludes zero. No losses on
  time, White 58.14%, design effect 1.03.

**The Reverse Futility half, half a step: refused 2026-10-10.** The margin
`rfp_margin_per_depth * depth - (rfp_margin_per_depth / 2) * improving` keeps a margin at every
depth: **40**/120/200 when improving, v0.46.0's 80/160/240 otherwise. The code is on the local
branch `rfp-improving-half-step` (`3953d74`, on top of the whole step's commit). It also widens
`test_lmp_base_moves_controls_how_much_is_pruned` to depth 9: the half step flipped that test's
eight-cell sum by 0.6%, all of it from `SMOTHERED_MATE_FEN` at depth 8, while base 0 still searched
less in seven of the ten cells. The extension was chosen after that reading and says so.

* **Census** (300 pool positions, depth 10): **17.5%** of the Reverse Futility cuts are new,
  +22% / +19% / +38% at depths 1 / 2 / 3, against the whole step's 29%.
* **Tree** against v0.46.0, depth 10: 0 of 300 identical on both pools, **13.2%** and **12.6%**
  less time. Recorded, not a gate (rule 1).
* **Gate:** the same run shape. [0, 10] **H0 after 4058 games** (LLR -3.09, stopping score
  49.85%, -1.0 Elo); with the estimate not positive the run stopped there instead of continuing
  under [0, 5], which stood undecided at -0.90. Over the 4072 games **-0.9 Elo, paired 95%
  [-10, +8]** - a stopped run, so not an effect size (rule 7). No losses on time, White 61.65%,
  design effect 1.00.

**What the two runs say together.** Halving the step took the result from about -22 Elo to about
zero and the tree saving from 22% to 13%: neither time did the faster tree buy games. Neither run
gives a reason to expect a gain from a step of this kind. Still untried: the step from depth 2
only (80/80/160 when improving), and a tuned margin as a new SPSA-registered parameter, which the
tuner's step-size defect would leave stuck should the value drift near zero (see the open-items
table). Neither is recommended on this evidence.

**The Late Move Reduction half: shipped in v0.46.1 (2026-10-11).** At a node that is not
improving, a move the rule reduces anyway is reduced one ply more: `lmr_reduction` adds the ply
after the damping terms and only to a positive result. No reduction is created where the damping
left none, and the clamp to `depth - 2` absorbs the extra ply at depth 3. A node with no
comparison counts as not improving, as for Late Move Pruning - it is the same flag. The form is
the narrowest of three: Stockfish adds the ply wherever the table cell is at least one, which would
also reduce killers, counter moves and good-history moves the damping exempts here; the other
direction, one ply less when improving, grows the tree at two thirds of the nodes. Neither was
measured.

* **Census** (300 pool positions, depth 10): 6.18 M reduced moves, **68%** of them at nodes not
  improving; the extra ply got past the clamp on half of those (never at depth 3, always from
  depth 7), so **34%** of all reductions are one ply deeper. Re-searches after a reduced search:
  **0.36%** at nodes not improving against **1.36%** at improving ones, the extra ply included.
* **Tree** against v0.46.0, depth 10: 0 of 300 identical on both pools, **14.6%** and **12.5%**
  less time. Recorded, not a gate (rule 1).
* **Gate:** SPRT at 1s + 150ms on `openings_wide.txt`, concurrency 4, run under [0, 5] with
  [0, 10] watched on the same games. **Both H1 after 9846 games** ([0, 5] LLR +3.03, [0, 10]
  +2.98, stopping score 51.07%). Over 9847 games **+7.4 Elo, paired 95% [+2, +13]** - a stopped
  run, so not an effect size (rule 7). On the way the [0, 10] LLR fell as low as -1.88 at 4764
  games. No losses on time, White 60.46%, design effect 1.10.
* **Smoke gauntlet**, 1s + 100ms, 100 games per pairing: 53.0% against v0.46.0, 46.0% against
  v0.45.9, 88.0% against v0.34.0, no losses on time.

## 23. The History Heuristic

Four defects were found on 2026-09-04. Three are repaired and shipped — the tables no longer die
at every iterative deepening iteration (v0.42.0), the table is indexed by side (v0.44.0), and it
is signed and updated by gravity (v0.45.0); all three are in the closed record of section 19. The
fourth is below.

### 23.4 `lmr_history_good_threshold` has never fired, and the curves are still untuned

`[Threshold closed 2026-10-07: kept at 0, +2.7 Elo, 95% [-4, +9]]` — the curves are still untuned.

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

**What it measured.** The run finished on 2026-09-12 and was read on 2026-10-07: **+2.7 Elo, 95%
[-4, +9]** for `good = 0` over 3000 pairs, pair outcomes from the candidate's side 2-0: 208,
1.5-0.5: 749, 1-1: 1142, 0.5-1.5: 683, 0-2: 218; no losses on time, White 60.35%, design effect
1.05. A null: the 12.8% to 17.5% wall-time cost at fixed depth did not show up as a loss, and the
rebate did not show up as a gain. **The threshold stays 0** — it is the shipped default of the
next release, not worse than 4000 and free of the constant.

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

## 27. The source review of 2026-10-07 to 2026-10-09

A full review of `master` at `49480c0` in nine groups — root, node, evaluation, move generation,
table, UCI and time, book and configuration, build and scripts, NNUE — produced 34 findings, R1 to
R34, each classed by what it does to the search tree: **A** identical, **B** different only in
special cases, **C** priced in games, **Q** code quality only. Strength came before code quality,
and nothing shipped without the evidence its class asks for.

| Release | Findings | Evidence |
| :--- | :--- | :--- |
| v0.45.1 | 23.4 alone | +2.7 Elo [-4, +9], 6000 games, section 23.4 |
| v0.45.2 | R1 PV copied only up to its end, R2 SEE stops once the victim outweighs the attacker, R3 the search arena allocated once, R5 the null move restores the game status, R6 a mate bound on Null Move Pruning and Reverse Futility Pruning, R33 huge pages and an earlier table prefetch | 300 of 300 trees identical, **-11.9% time** to depth 10 |
| v0.45.3 | R8 evaluation work the search never reads, set-wise pawn terms, R9 the opposite-bishop scale overflowed `i16`, R10 knight parameters read from the configuration | identical, **-10.4%** |
| v0.45.4 | R11 no regular expression per UCI move, R12 compile-time move tables, R13 a 128 MB default table, R14 tuning parameters at the shipped values, R16a/b book robustness, R34 a replayed threefold no longer answers `bestmove 0000` | identical, +0.9%; start-up 369 to 267 ms, peak RSS 788 to 152 MB |
| v0.45.5 | R17 a threefold is a draw at node entry | SPRT [0, 10] H1 after 1446 games |
| v0.45.6 | R20 a fail low keeps the table move of the same position | SPRT [0, 10] H1 after 3830 games |
| v0.45.7 | R21 a mate stops iterative deepening only inside the completed depth | 539 of 539 won endings mated against 528, McNemar p = 0.0005 |
| v0.45.8 | R19 an interrupted iteration keeps a root move it has already proven better | SPRT [0, 10] H1 after 866 games |
| v0.45.9 | R24 a repetition strictly inside the search tree is a draw at its second occurrence | SPRT [0, 10] H1 after 3860 games |

**The review as a whole**, v0.45.9 against v0.45.0, fixed-N 6000 games at 1s + 150ms, finished
2026-10-09: **+98.6 Elo, 95% [+92, +106]** (63.82%, 3000 pairs: 2-0 585, 1.5-0.5 1001, 1-1 979,
0.5-1.5 358, 0-2 77; no losses on time, design effect 1.00). The SPRT stopping scores above are not
effect sizes (rule 7); this is.

**Not shipped.** R18, the time allocation — no new iteration once 60% of the budget is spent, and
the budget bounded below the clock instead of collapsing to 10 ms: SPRT [0, 10] H0 after 9050
games, [0, 5] undecided at 9244, +2.3 Elo [-3, +8]. Stopped for futility on the owner's decision;
the code is on the local branch `review/r18-time-management` (`e8bda1e`). The release profile
(fat LTO, one codegen unit, `panic = abort`) measured +/-0.1% and `target-cpu=native` -0.7%.

**What the campaign taught, for the next one:**

* **A [0, 10] SPRT cannot see +5 Elo, and the owner accepts +5** (2026-10-08). When a [0, 10] run
  drifts towards its cap with a positive estimate, the same games can be read under [0, 5] — but
  only if the [0, 5] trajectory (`scripts/sprt.py --trajectory --elo1 5`) has never touched a
  bound, so that a [0, 5] test from game 1 would be in the same state; continue with
  `SPRT_APPEND=1` and a round total the first run did not use. Simulated over R18's whole
  procedure, that ships a 0-Elo change 5.4% of the time and a -3-Elo change 0.8%. A futility stop
  never raises those numbers.
* **A change that acts in a corner of the game is priced on positions from that corner.** R21
  acts only once a mate score reaches the root: in the R18 run 2815 of 9245 games reached a queen
  or rook against a bare king and 194 of them were drawn, which a match prices at +3 to +4.5 Elo
  at most. `scripts/conversion_test.py` plays such positions out with each binary as the attacker
  (`openings/bare_king_endings.epd`) and decides on the positions where the two disagree.
* **Nothing runs beside a timing measurement.** R19's first tree reading, taken while a probe
  played games on another core, said -2.3% on identical trees; the repeat said -0.3%.
* **The review's estimate of a tree effect does not transfer.** R20 was found as "25% fewer
  nodes at depth 10" and measured +3.5% time on the pool — and won its SPRT anyway. Price in
  games, as rule 1 says.

**Left open** — in the table at the top: R4, R7 and R16c deferred; R22, R23 and R25 to R30 listed
and unpriced. The NNUE branch was read but not changed: an accumulator update that is bit-identical
at 20% less CPU, lazy child accumulators, the static evaluation in the table entry, Futility and
razoring margins in HCE units, and `NnueModelPath` / `UseNNUE` options that do nothing (N1 to N5).
Q, not done: 26 clippy lints, unused dependencies (`chashmap` with RUSTSEC-2022-0040, `rayon`,
`crossbeam-queue`), the broken legacy scripts `bench.sh`, `benchmark_nps.py`,
`run_perft_bench.py` and `apply_spsa.py`, and a `#[test]` that sits outside `mod tests` at the end
of `model.rs`.

## 28. Principal Variation Search at the root, v0.46.2

`get_moves` searched **every** root move with `is_pv = true` and the full current window, so the
root had one PV node per legal move: under each of them razoring, SEE pruning of bad captures,
Late Move Pruning and Futility Pruning were off on the first line, and the Late Move Reduction
reduced one ply less. Once a pass had failed high against the aspiration window, the remaining
moves got an empty window.

**What v0.46.2 does** (`Config::root_pvs`, UCI `RootPvs`, default on):
* The first root move is searched as before (full window, PV). Every later one on a null window at
  the bound it has to beat (`current_alpha` for White, `current_beta` for Black), as a non-PV node;
  a score strictly better than the bound is searched again with the full window as a PV node -
  also when it is beyond the aspiration window, as the published root does.
* A pass whose window has closed (`current_alpha >= current_beta`, both colours) ends there; the
  aspiration loop searches it again with a wider window.
* A move whose null-window search proved it better but whose re-search the clock cut is kept with
  that bound and the move alone as its line, so R19 can play it.
* Off, the search is v0.46.1's node for node: 300/300 identical trees on both pools at depth 10.

**What was measured** (2026-10-12, host C):
* Census, 300 pool positions at depth 10 (`search-diag`, protocol only): 26.7 / 26.3 null-window
  root moves per pass, **2.06% / 1.79%** of them beat the bound; of the re-searches 79% / 86%
  became the new best; **20.0% / 17.9% of the passes** end on an aspiration fail-high
  (`openings_wide` / `openings_mixed`).
* Tree against v0.46.1 at depth 10: 0/300 identical, **-16.8% / -15.8% time** (median ratio
  1.15 / 1.14). Recorded, not a gate (rule 1).
* **SPRT** against v0.46.1 at 1s + 150ms, `openings_wide.txt`, concurrency 4, run under [0, 5]
  with a [0, 10] monitor: **[0, 10] H1 after 386 games** (LLR +3.03), [0, 5] undecided at +1.66.
  Over 399 games **+69 Elo, paired 95% [+41, +97]** - a stopped run, not an effect size (rule 7).
  No losses on time, White 55.4%, design effect 1.00.
* Smoke gauntlet, 1s + 100ms, 100 games per pairing: 62.0% / 67.5% / 85.0% against v0.46.1 /
  v0.46.0 / v0.34.0, no losses on time.

**Two unit tests re-sampled**, their claims unchanged: `test_dynamic_nmp_verification_search`
moved from depth 4 to 5 (at depth 4 the root's re-search of a fail-high outweighs what the null
move saves); `test_each_new_nmp_guard_changes_the_tree` now runs seeded, as every iteration from
depth 3 does in play - unseeded, the first line from the root has the full window, Null Move
Pruning stays off against a mated `beta`, and with root PVS the NMP PV guard reached no node at
depth 7.

**What it unblocks:** the `!is_pv` guards of 20.2 (re-measured and refused on 2026-10-13, see the
open list), then 21.3 (shipped in v0.46.4). Not in scope and still open: R23 (table cutoffs at the root's children can hide a
repetition).

**On the NNUE branch: v0.46.2-NNUE** (2026-10-12). Root PVS alone, ported onto v0.45.9-NNUE
without the `improving` changes of v0.46.0 and v0.46.1, which were refused there; protected
parameters unchanged. Off-path identity 300/300 on both pools; tree to depth 10 -19.3% / -17.1%;
null-window fail-highs 1.70% / 1.59%, root cutoffs in 13.0% / 12.0% of passes. **SPRT [0, 10] H1
after 158 games** against v0.45.9-NNUE (run under [0, 5] with a [0, 10] monitor); over 169 games
+107 Elo, paired 95% [+72, +144] - a stopped run, not an effect size. Smoke gauntlet 60.5% /
77.5% / 84.5% against v0.45.9- / v0.45.0- / v0.34.0-NNUE. One more test was re-sampled there:
`test_aspiration_window_is_seeded_from_previous_score` runs with the Transposition Table on, as in
play - without it every aspiration re-pass starts from scratch, and with root PVS the seeded search
was the larger one on that branch (22,685 against 21,527 nodes); with the table it saves again.

## 29. The evaluation: audit of 2026-10-16

`[Impact: High]` `[Complexity: Low to High]` `[measured by census and Texel error, not in games]` —
asked for by the owner on 2026-10-16: what can be improved in the evaluation, what is useless, what
is missing, what is slow. **Nothing in the evaluation has been priced under the rules of this
file.** Every term dates from v0.28 or earlier (August 2026 and before), many of the classical ones
from v0.27.0 to v0.27.5 within two days, gated on LCT II or a stopping score if at all; the review
of 27 touched the evaluation for speed only (R8 to R10). **No code was changed.** The instrument is a patched
scratch copy of v0.46.4 that searches v0.46.4's trees while its `PROBE_*` switches are off (40/40
identical at depth 9); patch, README and raw output are in
`<mm>/archive/eval-audit-2026-10-16.tar.xz`. Host C, 2026-10-16, nothing else running.

### 29.1 What the evaluation is

* `EvalService::evaluate` (`src/eval_service.rs`), White-positive. NNUE is off on `master` and the
  network is never loaded.
* **Material and piece-square tables** (`src/pst.rs`, incremental in `Board::pst_mg/pst_eg`):
  P 100, N 300, B 300, R 500, Q 950, **the same in both phases**. The tables are the Chess
  Programming Wiki's "Simplified Evaluation Function" scaled by 0.8 - one table per piece, a
  separate endgame table for the king only, none for the pawn.
* About 90 weights in `Config`, all but `max_eval_mult` reachable by `setoption` (the advertised
  list is shorter), most of them SPSA-registered. Next to them, constants in code that nothing can
  tune: the passed-pawn table 10/40/90/180/320/500 in the endgame (a third of it in the
  middlegame); endgame extras +15 (isolated pawn, bishop pair, rook on the 7th), +10 (backward
  pawn, open file, doubled rooks), +5 (half-open file); a rook's attacked and defended pawns 15 and
  8; the king's distance to the centre at 40 per step below phase 60; the opposition below phase 40.
* After the terms, in this order: the **positional cap** (a positional part beyond 150 keeps
  `150 + excess / 5`), the opposite-bishop scale (51%), **`adjust_eval`** (29.5, E1), mop-up.
* **Lazy evaluation**: `cheap_eval` (tables, cached pawn structure, scaling) is returned when it is
  180 (search) or 120 (Quiescence Search) outside the window, from phase 60 up.

### 29.2 Cost

* **The evaluation is 19.9% / 19.8% of the time to depth 10** (`openings_wide` / `openings_mixed`,
  300 positions each): every node evaluated twice and the second result discarded, 300/300
  identical trees, 37.7 s -> 45.2 s and 35.6 s -> 42.7 s. 34.2M / 33.0M calls, 0.22 us each on
  average; the lazy return answers 43.9% / 44.7% of them.
* Per call on 1000 positions sampled from those searches (cache-resident, best of 200): a full
  evaluation **412 ns**, `cheap_eval` 22 ns. Knock-outs: rook, knight, bishop and queen terms
  **253 ns (61%)**; the pawn structure on a pawn-hash miss +175 ns (hits 94.5% / 93.5%); king
  terms 44; the queen's attacker scan (`get_attackers_mask_for_see`, a pin test per attacker) 29;
  passed-pawn detection 22, recomputed at every call and again per rook for every pawn on its file
  (9 more); dynamic pawn terms 14; positional cap 12; defender scan 8.
* **Speed is a minor lever.** All of it is a fifth of the search. Computing every piece's attacks
  once and sharing them (mobility, threats, king ring, defenders, the queen scan), and summing
  packed `(mg, eg)` with one taper at the end, might take a third off: **5 to 7% of the time to
  depth**. Worth it only together with the knowledge that needs attack maps (E5).

### 29.3 Census (depth 10, 300 positions per pool, `openings_wide` / `openings_mixed`)

* **King danger from piece attacks is absent.** `danger` sums the king-ring squares a piece attacks
  times 1/1/2/4 (N/B/R/Q) and is scaled by `weight[attackers - minor defenders] / 100`, weights
  0/10/50/100/150/200, in integer arithmetic. It fires on 10.1M / 11.3M side-evaluations with a
  **mean of 0.03 cp and a maximum of 37 / 39 cp**, never above 50. What the evaluation knows about
  a king is its pawn shield, open files and `undeveloped_king_malus`.
* **The positional cap fires on 22.6% / 17.2% of full evaluations** and removes 54 / 50 cp each
  time, 100 to 200 cp in 13% / 12% of the firings. The mean |positional part| is 100 / 89 cp.
* **Lazy evaluation answers wrongly on 0.90% / 0.75% of its returns** (135K / 110K per pool): the
  full evaluation lies on the other side of the bound, by 181 / 176 cp on average. **16.3% /
  17.2% of all lazy returns compare a value without the pawn structure** - no pawn-hash entry,
  first visit or evicted, the open-list defect - and they are 14.8% / 18.3% of the wrong ones. Over all lazy returns
  |full - cheap| is 61 / 60 cp on average, above 120 in 11.6% / 10.9%, above 180 in 1.9% / 1.4%.
* **R26 measured:** 0.23% / 0.24% of pawn-hash hits return a blockade term computed for another
  piece placement, by 22 cp on average. The blockade is the only piece-dependent term in the entry.
* **`king_in_check_malus` (135) and `king_in_double_check_malus` (337) are unreachable in play.**
  The search evaluates a node in check only at the ply ceiling (`MAX_PLY - 1`), and
  `calc_eval_known_check` skips the attacker scan everywhere else. Both sit in the SPSA group
  `king_safety`: two dead dimensions.
* Endgame terms barely occur from these opening pools - opposition 0, mop-up 0, opposite bishops
  16 / 13, `adjust_eval` on 0.25% / 0.20% of full evaluations. The Texel sets show them.

### 29.4 Texel error: what the evaluation predicts

**An instrument, not a gate** (rule 1). Quiet positions from games - from ply 16, not in check,
neither the move played nor the one before it a capture or promotion - labelled with the game's
result; the error is the mean square of `result - 1 / (1 + e^(-k * eval))`, `k` refitted for every
variant so that a plain rescale scores nothing. **Set A**: all 13,500 games of `<mm>/all.pgn`
(v0.37.0 to v0.40.0, HCE and NNUE, 1s + 100 or 120 ms), 734,848 positions. **Set B**: the last
30,000 games of `<mm>/all.pgn.backup` (v0.32 to v0.37, mostly faster controls), every second
position, 797,681. The shipped evaluation scores 0.09961 / 0.10489 with `k` 0.00319 / 0.00329:
**a pawn of advantage is worth +7.9 / +8.2 points of expected score**, at a mean |eval| of 502 /
499 cp. The colour mirror is exact on all 734,848 positions of A.

Error change x 10^6 against the shipped evaluation, A / B, negative predicts better. 101 of the
118 variants carry the same sign on both sets.

| Change | A | B | Reading |
| :--- | ---: | ---: | :--- |
| piece-square deltas fitted per square, 6 x 64 x mg/eg, on held-out games | **-4099** | **-4640** | the largest lever by six times; on the training games -6853 / -5441 |
| material deltas only, 5 pieces x mg/eg, held-out games | -1432 | -1788 | knight +85 / +133, bishop +139 / +153, rook +55 / +101 (endgame +178 / +146), queen +279 / +271 (middlegame, A / B), pawn near 0: material is light against the positional mass |
| positional cap off | +4210 | +3716 | **load-bearing**; damping 2 +1403 / +1153, cap 400 +3158 / +2676 |
| `adjust_eval` off | **-634** | **-776** | continuous, on the excess over 200 only, -455 / -527; `max_eval_mult` 1.5 -375 / -372; on every evaluation +149 / +11 |
| mobility, all four factors x2 | -418 | -350 | x1.5 -139 / -112, x0 +565 / +504; bishop mobility x2 alone -331 / -265 |
| bishop pair x2 | -298 | -160 | x1.5 -187 / -120, x0 +596 / +470 |
| pawn rank bonuses 183/104/47 (every pawn) x0 | -469 | -89 | x2 +867 / +700 |
| the hard-coded passed-pawn table off | -758 | +228 | **not robust** alone. With the row above and set A's fitted pawn deltas (-150 to -200 in the endgame on ranks 6 and 7): an advanced pawn is paid by the table, the rank bonus and the PST at once, and how to split it is a tuner's question |
| backward pawn x0 | -283 | -125 | x2 +373 / +231; "backward" here is any pawn with no friendly pawn beside or behind it |
| king centre, 40 per step below phase 60, off | **-272** | **-378** | ungated +187 / +192: it counts the king's endgame table a second time |
| **new:** a no-pawn scale factor | **-255** | **-254** | changes 0.6% of the positions: a stronger side without pawns and at most a minor piece ahead scales to 0, 4/64 or 14/64 |
| king shield x0 | -230 | -170 | x0.5 -166 / -140, x2 +504 / +479 |
| pawn chain (`pawn_structure`) x2 | -108 | -92 | x0 +118 / +104 |
| king-ring attack weights x4 | -75 | -70 | x2 -27 / -26: the absent king danger of 29.3 |
| protected passer x0, connected passers x0 | -60, -50 | -56, -50 | |
| opposite-bishop scale off | +338 | +209 | keep |
| opposition, heavy piece on the king's file, bishop trapped at the rim, pawn storm | within 15 | within 15 | no signal either way |

### 29.5 The candidates, in the recommended order

Each changes the tree (class C of 27) unless marked otherwise, and the Texel number of none of them
is an effect size: a lower error says the static evaluation predicts these engines' results better,
not that the engine wins more.

* **E1 - `adjust_eval`, a step of up to 200 cp at +/-200.** Below phase 155 an evaluation of 200 or
  more is multiplied by `255 / (phase + 100)`, at most 2.0. In a pawn ending 199 stays 199 and 200
  becomes 400; with a rook each, 200 becomes 359. No margin of the search (aspiration 15, Reverse
  Futility 80 per ply, lazy 120 / 180) expects one centipawn to move a score by 200. It is the
  strongest Texel signal that holds on both sets. What the multiplier is for - wanting to trade when
  ahead, since material is the same in both phases - is the risk, so it is priced in games: a UCI
  spin for the multiplier (100% = off) gives identity at the old value, then one SPRT. Small.
* **E2 - a Texel tuner.** Material and piece-square tables alone, fitted as deltas on top of the
  shipped evaluation, cut the held-out error six times as much as any single change, on both sets.
  SPSA cannot do this - 91 parameters, the step defect of the open list, a few Elo per parameter
  that 1s + 150ms cannot resolve. A Texel tuner fits hundreds of weights against results in
  minutes. It needs an evaluation trace (every term reports its feature counts, mg and eg),
  regularisation, an export into `src/pst.rs` and `src/config.rs`, and a dataset (the instrument
  extracts 13,500 games in 1.5 s). The cap and `adjust_eval` are not linear: E1 first, and the cap
  either modelled or tuned with the weights - it cannot simply go (29.4). One SPRT prices the tune.
  Large; the measured lever is the biggest in this file.
* **E3 - the king-centre step below phase 60.** 40 per step of distance to the centre, on top of
  the king's endgame table and `king_trapp_at_baseline_malus`, switched on by the capture that
  takes the phase below 60. Removing it improves both sets (-272 / -378). Small.
* **E4 - a no-pawn scale factor (new).** -255 / -254 from 0.6% of the positions. It acts in a
  corner, so it is priced like R21: a conversion test on pawnless endings from both sides, or an
  SPRT that is told how rarely it fires. Small.
* **E5 - king safety.** Absent today (29.3). The published shape - attacker count and weights,
  attacked ring squares, safe checks, weak ring squares, a discount without the queen, a
  quadratic danger - only with E2, or it is one more hand-set term. With the attack maps of 29.2.
  Large.
* **E6 - two small defects, priced each.** R26: the blockade leaves the cached structure for the
  dynamic terms (set-wise, `(passers << 8) & enemy pieces`). The lazy first visit: `cheap_eval`
  computes and stores the pawn structure on a miss, which the full evaluation would pay anyway.
* **E7 - speed (class A where the value is unchanged).** Passed pawns cached in the pawn entry or
  found set-wise, the rook loops reading them, `get_game_phase` once per call, the attack maps of
  29.2. Summing packed `(mg, eg)` and tapering once changes the rounding and is class B.
* **E8 - dead or silent (class Q / B).** `king_in_check_malus` and `king_in_double_check_malus`
  (unreachable; at least out of the SPSA group); the opposition bonus (no signal, 1.3% of the
  positions); `Aggressiveness` (it scales the dead ring weights and the queen-attack terms and moves
  the cap, which scores +2676 to +3158 at 400); `_knight_moves`; the `print_eval_per_figure`
  branches in the piece loops; the White and Black twin of every term function, about 950 lines of
  which half mirror the other half. The NNUE code on `master` stays - the porting procedure
  diffs against it.

**Weights that `setoption` reaches today** - bishop pair 48 -> 72, mobility x1.5, king shield
x0.5, backward pawn 11 -> 0, the rank bonuses to 0 - can each be played on the v0.46.4 binary as
21.3's margin was, at 2,000 to 4,000 games a run. E2 prices them all in one; a series of single runs
is not recommended.

R28 of 27 is listed as "two steps in the evaluation" and was never written up; the two steps found
here are E1 and E3, and whether they are the same two is not recorded.
