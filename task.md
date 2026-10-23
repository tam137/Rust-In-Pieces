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
| Released | **v0.47.3** on `master` (HCE) since 2026-10-18 — the Reverse Futility margin 80 -> 90 for the wider evaluation (29.13), SPRT [0, 10] H1 after 4138 games against v0.47.2, +10.6 Elo [+2, +19] over 4152 games, a stopped run. Before it **v0.47.2** (2026-10-18), the king tables re-fitted and the king-centre step removed (29.10), SPRT [0, 10] H1 after 4994 games against v0.47.1, +10.4 Elo [+2, +18] over 5005 games, a stopped run. Before it **v0.47.1** (2026-10-17), the tables of rook, knight, bishop and queen re-fitted with the material (29.9), SPRT [0, 10] H1 after 848 games against v0.47.0, +40.2 Elo [+19, +62] over 861 games, a stopped run. Before it **v0.47.0** (2026-10-17), the material re-fitted by the Texel tuner (29.7), SPRT [0, 10] H1 against v0.46.4, +107.2 Elo [+81, +135] over 488 games, a run stopped by hand. Before it **v0.46.4** (2026-10-14), Reverse Futility Pruning to depth 8 and ahead of Null Move Pruning (21.3), SPRT [0, 10] H1 after 1876 games against v0.46.3. Before it **v0.46.3** (2026-10-13), Null Move Pruning at the published depth and without the verification search (section 20), SPRT [0, 10] H1 after 3308 games against v0.46.2. Before that **v0.46.2** (2026-10-12), Principal Variation Search at the root (section 28), SPRT [0, 10] H1 after 386 games against v0.46.1. Before that **v0.46.1** (2026-10-11), `improving` in the Late Move Reduction (21.4), SPRT [0, 5] and [0, 10] H1 after 9846 games against v0.46.0. Before that **v0.46.0** (2026-10-10), `improving` and the Late Move Pruning threshold of section 21, SPRT [0, 10] H1 after 4580 games and [0, 5] H1 after 6234 against v0.45.9. Before that, **v0.45.9** (2026-10-09), the end of a full source review, section 27: nine releases from v0.45.1, three of them tree-identical speed and robustness work (**22% less time to fixed depth** on identical trees) and five priced in games or positions — R17, R20, R19 and R24 by SPRT [0, 10], R21 by a targeted conversion test. Against v0.45.0, fixed-N 6000 games at 1s + 150ms: **+98.6 Elo, 95% [+92, +106]**. Porting to `feature/nnue-evaluation` is governed by `skills/nnue_porting_and_release_procedure.md` |
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
independent game sets. The rest is in 29.5. **E2 was built and refused on 2026-10-17 (29.6):** the
tune cut the held-out Texel error by 8200 x 10^-6 and the search needed 58% / 69% more time to
depth 10; SPRT [0, 10] H0 after 2564 games against v0.46.4, -8.2 Elo [-21, +5] over 2576.
**The material alone shipped in v0.47.0 the same day (29.7):** the tables are what slowed the
search, the material makes it faster; SPRT [0, 10] H1 against v0.46.4, +107.2 Elo [+81, +135] over
488 games, a run stopped by hand and not an effect size. **The tables of rook, knight, bishop and
queen followed in v0.47.1 (29.9):** the pawn table is the brake (29.8), the other four make the
search faster; SPRT [0, 10] H1 after 848 games against v0.47.0, +40.2 Elo [+19, +62] over 861, a
stopped run. **The king tables followed in v0.47.2 (29.10)** with the king-centre step (E3)
removed: SPRT [0, 10] H1 after 4994 games against v0.47.1, +10.4 Elo [+2, +18] over 5005, a
stopped run. **The pawn table was measured on 2026-10-18 (29.11):** no variant within the time
stop; the brake is spread over the whole tree, not re-searches, lazy evaluation or the rank
bonuses. **The pruning margins scaled by 1.12 to the new evaluation (29.12)** cost +6.2% / +2.0%
time, over the owner's +5% rule: no game; **RFP alone at 90 (29.13)** cost +5.1% / +3.5%, over it
by 0.1 point; **the owner let it play and it shipped in v0.47.3 (29.13)**: SPRT [0, 10] H1 after 4138
games against v0.47.2, +10.6 Elo [+2, +19] over 4152, a stopped run. Next: futility and razoring one
at a time the same way, the passed-pawn table, or E1. **Futility alone at 134 + 90 (29.17)** cost
almost nothing (+0.3% / +0.2%) and was **refused**: SPRT [0, 10] H0 after 1424 games against
v0.47.3, -8.9 Elo [-23, +5] over 1438. Futility stays at 120 + 80. **E1, `adjust_eval` off, shipped in
v0.47.4 (29.18):** SPRT [0, 10] H1 after 7610 games against v0.47.3, +7.9 Elo [+2, +14] over 7622,
a stopped run. **The material re-fitted without it shipped in v0.47.5 (29.19):** SPRT [0, 10] H1
after 2944 games against v0.47.4, +13.1 Elo [+3, +23] over 2956, a stopped run. The search
margins x0.822 (29.20) shipped nothing. **The passed-pawn table re-fitted by Texel shipped in v0.47.6
(29.21):** +22.5 Elo [+10, +35] over 2006 games against v0.47.5, run to a fixed 2000. The pawn
rank bonuses re-fitted stopped at the Texel bar (29.22), no game. A new Texel data set from the
games of v0.47.x (29.23) ranks what is left: the bishop pair and mobility first. **They shipped in
v0.47.7 (29.24):** +14.2 Elo [+3, +25] over 2494 games against v0.47.6, a stopped run. The existing
king-safety group re-fitted stopped at the bar (29.25), no game. **The threat values re-fitted
shipped in v0.47.8 (29.26):** +13.6 Elo [+3, +24] over 2980 games against v0.47.7. **The lazy margins
fitted to it shipped in v0.47.9 (29.27):** +12.5 Elo [+3, +22] over 3122 games against v0.47.8. Next:
new king-safety and threat features on shared attack maps (stage B).

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
| 12 | A Texel tuner for material, piece-square tables and the evaluation weights | 29.5 E2, 29.6, 29.7, 29.8, 29.9, 29.10, 29.11 | **Material shipped in v0.47.0 (2026-10-17)**: SPRT [0, 10] H1, +107.2 Elo [+81, +135] over 488 games. The full re-fit was refused (H0, -8.2 Elo, 58% / 69% slower to depth 10), material plus tables and six smoothed or shrunk table fits stopped before any game (+26% to +86%). The brake is the pawn table (+69% alone; 29.8). **The tables of rook, knight, bishop and queen shipped in v0.47.1** (29.9): 4.8% / 4.0% faster to depth 10, SPRT [0, 10] H1 after 848 games, +40.2 Elo [+19, +62] over 861. **The king tables shipped in v0.47.2** (29.10) with the king-centre step removed: +5.2% / +1.8% time, SPRT [0, 10] H1 after 4994 games, +10.4 Elo [+2, +18] over 5005. **The pawn table** (29.11): no variant within the time stop, the brake measured - spread over the tree, growing with how far the table moves. Left: the passed-pawn table and the weights |
| 13 | A no-pawn scale factor, R26, the lazy first visit | 29.5 E4, E6 | Small, each priced on its own; E4 acts in a corner and is priced like R21. E3, the king-centre step, went with the king tables in v0.47.2 (29.10) |
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
| `rfp_margin_per_depth` 80 against a published 90 to 100: **refused 2026-10-15** at 90 against v0.46.4. Census at depth 10 against 80, `openings_wide` / `openings_mixed`: 70 -6.3% / -7.2% time, 90 +0.4% / +1.0%, 100 +7.8% / +6.0%, 120 +17.3% / +12.3%; 100 was not played (more than 5% more time on both pools). 90: SPRT [0, 10] H0 after 3490 games, -2.3 Elo [-12, +7] over 3503, [0, 5] undecided at -1.07. Code on the local branch `rfp-margin`. **70 refused 2026-10-16**: census -5.4% / -6.5% time, SPRT [0, 10] H0 after 1424 games, -12.6 Elo [-27, +2] over 1438, code on the local branch `rfp-margin-70`. 100 in games is untried. All on v0.46.4's scale: **on the 1.12x wider evaluation of v0.47, 90 shipped in v0.47.3** (29.13, SPRT [0, 10] H1, +10.6 Elo [+2, +19]) | negative result on the old scale, section 21.3; superseded by 29.13 |
| Internal Iterative Reduction ships `false`: it was refused at the gate on 2026-09-09 because **41.4% of the nodes at depth 4 and above have no table move** here, which is a property of the Transposition Table and not of the rule. Worth exactly one re-measurement after backlog item 7, starting with the census share | negative result, section 22.3 |
| There is still no Internal Iterative Deepening — the other member of the family searches the node at reduced depth first and uses its move, which is a second search and not a decrement | proposal, unmeasured |
| There is no quiet-only move generator, which is what forced the staged picker's last stage to regenerate everything — see 19 | property, established 2026-09-04 |
| `SearchTables::age` halves the history once per iterative deepening *iteration*, because `get_moves` is one iteration; the published discipline halves once per `go` | untested variant, established 2026-09-07, section 23.1 |
| `time_check::run_time_check` drives its position list through one `EngineState`, so with persistent search tables its printed node counts are order-dependent | diagnostic only, no decision reads it, established 2026-09-07 |
| The startup benchmark `calculate_benchmark` runs a depth-3 search on the real `EngineState`, so the tables carry its history until the first `ucinewgame` | latent, harmless under any GUI that sends `ucinewgame`; the Transposition Table already had this property |
| Review leftovers, deferred: R4 `pv_nodes` behind a mutex and a SipHash map at every interior node (0.3% to 2%, 18 call sites), R7 a table move that is a promotion or en passant never gets the table band, R16c a `stop` that arrives before its `go` is lost | deferred, section 27 |
| Review leftovers, unpriced: R22 the lazy margin does not bound the positional terms (a tuning question; measured 2026-10-16, 29.3), R23 table cutoffs at the root's children can hide a repetition, R25 no quiet queen promotion in the Quiescence Search, R26 the pawn hash caches a blockade by pieces (measured: 0.23% of hits, 22 cp, 29.3), R27 the null child's counter-move entry and an abort sentinel that looks like a cutoff, R28 two steps in the evaluation (see the end of 29.5), R29 no fifty-move rule and the start position missing from the repetition history, R30 rook and bishop promotions off by default | listed, section 27; R22 and R26 measured in 29 |
| v0.47.0 ships the Texel material (29.7); the re-fitted piece-square tables made the search 70% slower to depth 10 on their own, the material 5% faster. Smoothing the tables, or a time-aware objective, is untried | open, section 29.7 |
| The Texel re-fit of 29.6, refused 2026-10-17: the held-out error -8200 x 10^-6, the search 58% / 69% slower to depth 10, SPRT [0, 10] H0 after 2564 games against v0.46.4, -8.2 Elo [-21, +5] over 2576. Code on the local branch `texel-tuner` (`cfa9f75` tree-identical tooling, `3d65e7e`, `49e76c2` the values) | negative result, section 29.6 |
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

**25.3 The entry caches no static evaluation** - *shipped in v0.48.0, see 30.1.* It holds the *search score* only, so every revisit
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

### 29.6 E2 measured: the Texel re-fit, refused on 2026-10-17

**What was built** (local branch `texel-tuner`, three commits on `de74cd1`):

* `cfa9f75`, **tree-identical** (300/300 on both pools at depth 10, time -0.2% / +0.0%): material and
  piece-square tables become a generated block in `src/pst.rs` with a middlegame and an endgame
  table for every piece, the positional cap reads its material from that block, the passed-pawn
  table becomes `Config::passed_pawn_bonus_mg/eg`. Behind the Cargo feature `texel`, compiled out
  of the release build: `src/eval_trace.rs`, where every term of the evaluation reports the
  parameter it read and its middlegame and endgame coefficient, and `src/texel.rs` with
  `texel-extract`, `texel-check`, `texel-tune`, `texel-export`. Tests: the trace rebuilds the
  evaluation, tracing changes no evaluation, the model follows a changed configuration, the
  analytic gradient equals finite differences, exporting the shipped parameters changes no file.
* `3d65e7e`: the tuner holds twelve scalars that depend on nothing but a piece and its square, re-
  centres each table's mean into its material after every step, and gains `texel-stats`,
  `texel-lazy` and `texel-mirror`.
* `49e76c2`: the tuned values, and ten tests re-sampled with their claims unchanged (the six
  hand-ranged evaluation tests run without the cap, without tempo and without threats, which is
  not the evaluation the engine plays with).

**The data.** 438,827 games of `<mm>/all.pgn`, `<mm>/all.pgn.backup` and the archived runs, at most
ten quiet positions each (the filter of 29.4 plus no capture with SEE > 0 for the side to move):
4,141,547 positions, 225,790 left out because a bare king (mop-up) or insufficient material scores
them. The SPRT runs of 2026-10-13 to 16 are the test set (187,879 positions). The trace explains
every unit of the evaluation to 3 cp at worst (0.046% of 60M units above 1 cp, all integer
rounding), and the model rebuilds the engine's evaluation within 0.31 cp on average, 99.98% of
positions within 3 cp; the maximum, 202 cp, is the `adjust_eval` step of E1.

**The tune.** `K` fitted once on the shipped evaluation (0.003411) and held, Adam over the full
batch, the pipeline (cap, opposite-bishop scale, `adjust_eval`) modelled as shipped. The first run
left every scalar free: test error -8272 x 10^-6, and nonsense - the rook on the seventh 33 -> 474
against table entries near -500, a king table entry of -1104, the endgame pawn at 23. A term that
a piece-square table can express exactly leaves a direction the error cannot see, and Adam drifts
along it; projecting the gradient did not hold the table means either, because Adam rescales each
parameter. The second run held those twelve scalars, re-centred the tables into material after
every step and put L2 on the tables only (lambda 1e-10 of 1e-10 / 1e-9 / 1e-8, chosen on
validation): error train -6143, validation -6063, **test -8200 x 10^-6** (0.10860 -> 0.10040).
Material, middlegame / endgame: pawn 97 / 63, knight 468 / 247, bishop 494 / 253, rook 706 / 453,
queen 1530 / 935; no table entry beyond +/-182; the colour mirror exact on 183,712 positions. The
fit moved what 29.3 and 29.4 said it would: king danger weights 10..200 -> 167..984, the threat
terms 5..24 -> 54..143, mobility 3/3/2/1 -> 10/10/5/3, the backward and isolated pawn to about 0.

**The census** against v0.46.4 (protocol only, rule 1), depth 10, 300 positions: **+58% / +69%
time** (`openings_wide` / `openings_mixed`), +69% / +80% generated moves, 0 of 300 identical.
`cheap_eval` lies more than 180 cp from the full evaluation on 5.8% of positions sampled from a
search against 1.8% (above 120: 19.8% against 9.1%). Resetting one group by `setoption` at depth 9:
the old threat weights save 10% of the time, the old mobility 11.5%, the old king danger nothing,
lazy evaluation off shrinks the tree by 13% and costs 5.5% time. No single group explains it.

**The gate:** SPRT at 1s + 150ms on `openings_wide.txt`, concurrency 4, [0, 5] with the [0, 10]
monitor. **[0, 10] H0 after 2564 games** (LLR -2.945, -8.4 Elo at the stop); [0, 5] undecided at
-1.18. Over the 2576 games that finished **-8.2 Elo, paired 95% [-21, +5]** - a stopped run, not
an effect size (rule 7). Draws 11.7% and a pair variance of 0.121 against about 0.08 in the runs
before: the two evaluations disagree often and decisively. No losses on time, White 50.4%, design
effect 1.04. The reading was -9.8 after 248 games, -3.5 after 400, -6.2 after 1004, -9.5 after 2002.

**What it says.** The re-fit predicts game results far better, and the search pays more than that
buys: at a fixed time it reaches well over half a ply less. Rule 1 again, from the evaluation's
side: a better predictor of results is not a stronger engine when the search was shaped around
the old evaluation - its margins (aspiration 15, Reverse Futility 80 per ply, lazy 120 / 180), its
pruning and its lazy shortcut all read a value whose terms are now up to ten times larger. What is
left of E2 is a re-fit the search can afford, and the tuner makes trying one cheap (extraction
1.5 minutes, a tune 15 to 25):

* material and tables only (29.5 D1b), the largest lever of 29.4, leaving the threat, mobility and
  king-danger weights the search is used to;
* a pre-registered census stop before any game, e.g. no more than 10% more time to depth 10;
* the lazy margins and `cheap_eval` measured against the new evaluation's spread.

The tree-identical tooling of `cfa9f75` could go to `master` on its own; it changes no tree. The
code is on the local branch `texel-tuner`.

### 29.7 The material alone: shipped in v0.47.0 (2026-10-17)

**Material and tables, stopped before any game.** The step after 29.6 re-fitted material and the
piece-square tables only (`texel-tune --free material,tables`), on the same data: held-out error
-6773 x 10^-6, material ordered in both phases, no table entry beyond +/-176. The time stop set in
advance - no game above +10% time to depth 10 on either pool - fired: **+81% / +77%**
(`openings_wide` / `openings_mixed`), twice the generated moves. Lazy evaluation was not the
cause this time (`cheap_eval` beyond 180 cp on 2.5% of search positions against 1.8%).

**The diagnosis**, two builds against v0.46.4 at depth 10, 100 positions: the **new material with
the old tables 4.9% faster**, held-out error -3120; the **new tables with the old material 70.5%
slower**, -2032. The tables slow the search, not the piece values; 29.6 had blamed the threat,
mobility and king-danger weights, and the tables carried most of it there too. Fitted square by
square, the tables differ sharply between neighbours, so most quiet moves move the evaluation -
an explanation, not yet a measurement.

**The material alone** (`--free material`; the re-centring fixed to run only when tables and
material are both free, as it divided by zero squares otherwise): held-out error **-4152 x
10^-6**. Middlegame / endgame: pawn 98 / 83, knight 464 / 316, bishop 507 / 335, rook 662 / 575,
queen 1437 / 1163, against 100, 300, 300, 500 and 950 in both phases; tables and weights as
shipped. Time to depth 10 against v0.46.4: **-2.6% / -7.1%**, generated moves -4.4% / -10.5%;
`cheap_eval` and the positional cap behave as in v0.46.4. Two tests re-sampled, claims unchanged:
`eval_fig_value_test` follows the material, and the Reverse Futility order test takes `beta` 2500
(the two rooks are worth more, so up to 2200 the null search itself cuts).

**The gate:** SPRT at 1s + 150ms on `openings_wide.txt`, concurrency 4, [0, 5] with the [0, 10]
monitor. At 216 games (+106 Elo, [0, 10] LLR +2.08) the owner asked for at least 1000 games
whatever H1 said; the wrapper and the monitor were stopped and a watcher took over that would
apply the same rules from game 1000. [0, 10] reached H1 at 300 games (LLR +3.06; `sprt.py --trajectory`) and stayed above it. At 488
games the owner stopped the run ("das ergebnis ist eindeutig"): **+107.2 Elo, paired 95% [+81,
+135]** over 488 games, [0, 10] LLR +4.92 (H1), [0, 5] +2.51 undecided - a stopped run, not an
effect size (rule 7). Pairs 0-2 / 0.5-1.5 / 1-1 / 1.5-0.5 / 2-0 from the candidate's side 11 /
25 / 85 / 53 / 70; no losses on time, White 55.1%, design effect 1.00. The reading was +113 after
108 games, +100 after 224, +113 after 398.

**Smoke gauntlet**, 1s + 100ms, 100 games per pairing: 59.5% against v0.46.4, 61.5% against
v0.46.3, 91.5% against v0.34.0, no losses on time.

**What it says.** One number per piece and phase was worth more than every other evaluation change
measured in this file, and it needed nothing the search had to pay for. The tables carry the rest
of the predictive gain (29.6: material and tables together -6773, material alone -4152), and they
are what the search cannot afford as fitted. Next, each behind the same time stop: tables
regularised towards smoothness (neighbouring squares), or a few squares at a time; then the
passed-pawn table; E1 `adjust_eval` with a re-tune. The tuner is in `master` behind the feature
`texel`.

### 29.8 Smoothed tables: no fit within the time stop (2026-10-17)

The step after 29.7 asked whether the tables' gain survives a constraint, and whether roughness or
size is what slows the search. Branch `texel-smooth` (local): `texel-tune --smooth` adds the
squared jumps between king-adjacent squares of every table to the error, every fit of a grid is
kept, `texel-stats` reports roughness and range, `texel-check` splits its deviations by whether
`adjust_eval` fired - all behind the feature `texel`, the release build identical to v0.47.0.

**The data** was extracted again with v0.47.0 (same sources and seed). `texel-check` read 99.425%
of positions within 3 cp against the 99.5% of 26: 99.994% where `adjust_eval` stays idle, 98.2%
where it fires - it multiplies the integer rounding of the cap and the taper by up to 2, and with
the v0.47.0 material it fires on 32% of positions. The hooks are unchanged (the unit check reads as
in 26); rounding, not a missing term.

**Six fits** from v0.47.0, material and tables free, against v0.47.0's held-out error (0.104902;
its tables: mean roughness 7.1 cp between neighbours, mean range 39 cp), timed to depth 10 against
v0.47.0 on 100 positions of `openings_wide`:

| Fit | Held-out error x 10^-6 | Roughness | Range | Time to depth 10 |
| :--- | ---: | ---: | ---: | ---: |
| smoothness 6e-12 | -3060 | 29.5 | 195 | +85.6% |
| smoothness 6e-11 | -3040 | 21.2 | 152 | +81.4% |
| smoothness 6e-10 | -2743 | 10.4 | 84 | +73.0% |
| L2 1e-9, no smoothness | -2734 | 17.4 | 105 | +72.4% |
| smoothness 6e-9 | -1929 | 3.2 | 31 | +33.2% |
| L2 1e-8, no smoothness | -1932 | 9.3 | 56 | +26.1% |

No fit within +10%: no game. **Neither roughness nor size is the brake.** Pairs with the same gain
cost the same time whatever their roughness and range - the rougher, larger one of each pair is a
little faster - and the time grows with the gain: what slows the search is where the tables move.

**Which table:** the tables of the smoothness-6e-10 fit for one piece at a time, v0.47.0
everywhere else - pawn -1022 / **+69.0%**, king -304 / +16.5%, knight -92 / +6.1%, bishop -274 /
+6.0%, queen -302 / +4.0%, rook -288 / -1.8% (held-out error x 10^-6 / time). **The pawn table
carries almost all of the brake** and the largest single gain; the others cost little each.

**Open, as candidates:** the tables of rook, knight, bishop and queen alone (single-piece gains
-956 together, single-piece times -2% to +6%), behind the same time stop; why the pawn table slows
the search - a pawn push that changes the evaluation may turn reduced searches into re-searches -
measured before anything is built on it. The tuner extension is in `master` since v0.47.1.

### 29.9 The tables of rook, knight, bishop and queen: shipped in v0.47.1 (2026-10-17)

The candidate 29.8 left: those four tables re-fitted with the material of all five pieces, the
pawn, king and passed-pawn tables and every weight as in v0.47.0. `texel-tune --tables
rook,knight,bishop,queen` moves only the named tables, and the re-centring skips the held ones
(it divided by zero squares otherwise); the test that the held tables stay byte-identical after
real epochs was red first. Release build of that commit identical to v0.47.0, 100/100 trees.

**Two fits** on the v0.47.0 data of 29.8, K held: L2 10^-10 **-1864 x 10^-6**, L2 10^-9 -1606 -
fitted together, twice the -956 of the single-piece fits added up. Time to depth 10 against v0.47.0
on 100 positions: +/-0.0% and -4.1%; the larger gain was taken. Material ordered in both phases
(table means moved into it), no table entry beyond +/-141, pawn and king tables unchanged, the
mirror test 0 asymmetric. **Full measurement: -4.8% / -4.0% time to depth 10** (`openings_wide` /
`openings_mixed`, 300 positions each), generated moves -1.0% / +2.8%. A gain the search does not
pay for, as with the material in 29.7.

Five tests re-sampled, claims unchanged: `eval_fig_value_test` (ranges follow the material),
`get_eval_even_test` (the "equal" rook a4 / a3 pair replaced by its mirror image),
`knight_position_test` (development to c6 instead of a6: the new knight table rates a6 12 cp below
b8), and the null-move verification and futility tests from depth 5 to 7 (with the new evaluation
their claim holds at 3, 4 and 7, not at 5, 6 or 8; noted in the tests).

**The gate:** SPRT at 1s + 150ms on `openings_wide.txt`, concurrency 4, [0, 5] with the [0, 10]
monitor, against v0.47.0. **[0, 10] H1 after 848 games** (LLR +2.97, +40.3 Elo), [0, 5] +1.59
undecided. At the end of the run, 861 games: **+40.2 Elo, paired 95% [+19, +62]** - a stopped run,
not an effect size (rule 7). Pairs 0-2 / 0.5-1.5 / 1-1 / 1.5-0.5 / 2-0 from the candidate's side
52 / 65 / 134 / 90 / 89; no losses on time, White 54.0%, design effect 1.01.

**Smoke gauntlet**, 1s + 100ms, 100 games per pairing: 52.0% against v0.47.0, 66.5% against
v0.46.4, 89.0% against v0.34.0, no losses on time. Shipped as **v0.47.1**, the release binary
identical to the candidate (100/100 trees).

**What it says.** The tables are affordable piece by piece: the pawn table is the brake (29.8), the
other four together make the search faster, not slower, and the fit gained more jointly than its
parts. Next, each behind the same time stop: the king table alone (+16.5% in 29.8, likely over the
stop); why the pawn table slows the search, measured before a fit is built on it; E1 `adjust_eval`.

### 29.10 The king tables, with the king-centre step removed: shipped in v0.47.2 (2026-10-18)

The candidate 29.9 left, and E3 with it. The king table of 29.8's fit alone cost +16.5% time and
had turned its endgame half round - d4 -53 against +32 shipped - because below phase 60 a
hard-coded step of 40 cp per step of distance to the centre (up to 240) counted centralisation a
second time; the fit cancelled it through the table. Three variants were priced before any game:
(i) the step off, (ii) the king tables re-fitted with the step kept, (iii) both. Material, the
other tables and every weight stayed as in v0.47.1; the king's material and a uniform shift of its
table cancel between the sides, so nothing was re-centred.

**Tooling** (`28f7d99`, tree-identical to v0.47.1, 100/100 trees): the step became
`Config::king_centre_step`, traced as a weight; `texel-tune --set name=value` holds a scalar at a
value. The test that the model follows the step was red first (model 533 against engine 502).
**Data** extracted again with v0.47.1: the same 4,141,547 positions and split as 29.8;
`texel-check` 99.39% within 3 cp, 99.996% where `adjust_eval` stays idle - rounding, as in 29.8;
`K` 0.002999.

| Variant | Held-out error x 10^-6 | Time to depth 10, 100 positions |
| :--- | ---: | ---: |
| (i) step off | -373 | +1.6% (90/100 trees identical) |
| (ii) king tables, L2 1e-10 / 1e-9 / 1e-8 | -443 / -397 / -186 | -0.1% / +2.2% / +5.7% |
| (iii) both, L2 1e-10 / 1e-9 / 1e-8 | **-615** / -580 / -491 | **+4.5%** / -3.8% / -0.1% |

Every variant stayed within the +10% stop; 29.8's +16.5% did not reproduce. The largest gain was
taken: (iii) at 1e-10. Its checks: castled squares ahead of e1 in the middlegame (net of
`undeveloped_king_malus`: g1 101, c1 69, e1 56); in the endgame the centre about 75 cp over the
corners instead of about 310 with the step, the advanced ranks positive; no entry beyond +/-143.
**Full measurement: +5.2% / +1.8% time to depth 10** (`openings_wide` / `openings_mixed`, 300
positions each), generated moves +7.9% / -2.6%.

The release removes the term and its `Config` field (tree-identical to the measured build with the
step at 0, 100/100). Two tests re-sampled, claims unchanged: `unequal_position_test` (the
centralised black king still better, -112 -> -54); and the LMP-base test, whose plain sum over 8
cells came out 0.4% the wrong way through single large cells while the geometric mean of the cell
ratios stayed at 0.89 - base 0 against base 12 on 100 pool positions at depth 8: 11.3% fewer
generated moves. It now asserts that mean.

**The gate:** SPRT at 1s + 150ms on `openings_wide.txt`, concurrency 4, [0, 5] with the [0, 10]
monitor, against v0.47.1. **[0, 10] H1 after 4994 games** (LLR +3.15, +10.2 Elo), [0, 5] +2.34
undecided. At the end of the run, 5005 games: **+10.4 Elo, paired 95% [+2, +18]** - a stopped run,
not an effect size (rule 7). Pairs 0-2 / 0.5-1.5 / 1-1 / 1.5-0.5 / 2-0 from the candidate's side
266 / 503 / 870 / 541 / 322; no losses on time, White 53.1%, design effect 1.05. The reading was
+16 after 1816 games ([0, 10] LLR +2.38), +4 after 2890, +9 after 4602.

**Smoke gauntlet**, 1s + 100ms, 100 games per pairing: 61.0% against v0.47.1, 57.0% against
v0.47.0, 93.5% against v0.34.0, no losses on time. Shipped as **v0.47.2**, the release binary
identical to the candidate (100/100 trees).

**What it says.** A hard-coded term the tables already express is worth removing before the table
is fitted: the step off alone gained more than half of what the joint change did, and the table
fitted without it is the one that kept its chess shape. A few percent more time to depth 10 did not
stop a gain of -615. Next: why the pawn table slows the search (29.8), measured before a fit is
built on it; then the passed-pawn table and E1 `adjust_eval`.

### 29.11 The pawn table: the brake measured, no variant within the time stop (2026-10-18)

The candidate 29.8 and 29.10 left. Three variants as for the king, plus a measurement of the
brake: (i) the rank bonuses off (eg 183 / 104 / 47 for every pawn on the 7th / 6th / 5th rank, mg
half - an advanced pawn is paid by them, the passed-pawn table and the pawn table at once); (ii)
the pawn tables re-fitted, bonuses kept; (iii) both. Material, other tables, passers and weights
as v0.47.2. Branch `texel-pawn` (local): `texel-swing` (`5ef64b3`, feature `texel`) measures how far
one quiet move moves the static evaluation, by the moving piece, with the side to move held. Data
extracted again with v0.47.2 (same positions and split as 29.8), `K` 0.003038.

| Variant | Held-out error x 10^-6 | Time to depth 10, 100 positions | 300 per pool |
| :--- | ---: | ---: | ---: |
| (i) bonuses off | -371 (train -11, validation +4) | -2.8% | +0.4% / -1.8% |
| (ii) L2 1e-10 / 1e-9 / 1e-8 / 1e-7 | -793 / -667 / -403 / -167 | +47 / +47 / +21 / +4.1% | |
| (iii) L2 1e-10 / 1e-9 / 1e-8 / 1e-7 | -824 / -789 / -662 / **-535** | +49 / +50 / +25 / **+2.1%** | **+6.2% / +15.4%** |

The largest gain within the screen, (iii) at 1e-7 (validation -365, plausibility passed), went over
the stop on `openings_mixed`: no game. (i) is time-neutral and over the bar on the test set only -
on training and validation it is worth nothing, so it was not played without the owner. The two
fits at 1e-10 failed the plausibility check (in the endgame the 5th rank below the 2nd): without
the passed-pawn bonus the data value a pawn on ranks 2 to 6 about alike and only the 7th higher.

**The brake**, `search-diag` at depth 10, 300 positions per pool, v0.47.2 against (iii) at 1e-9
(+50%) and at 1e-7: interior nodes +39% / +48% and +6% / +14% (`openings_wide` / `openings_mixed`);
every pruning count grows with them (RFP cuts +28%, null-move searches +31%, LMP +40% at 1e-9).
Per node little moves: the first-move cut 58.2% -> 57.3% / 56.9%, the null-move cut rate down 1.5
points, `improving` down 1 point, root fail-highs +9%; LMR re-searches stay at 0.8%, lazy
evaluation misses beyond 180 cp 2.0% -> 2.0% / 1.7%, the cap 36.3% -> 36.8% / 33.4%. A pawn move
moves the evaluation by 17 cp on average in v0.47.2 (6% beyond 50), by 25 cp (14%) at 1e-9 -
the fit asks for the centre pawns off d2/e2 (-72 / -70) and onto d5-f5 (+61 to +72) - and by 16 cp
(6%) at 1e-7, which still costs +14% on `openings_mixed`.

**What it says.** Neither suspicion holds alone. The rank bonuses are not the brake ((i) is
neutral; (ii) and (iii) cost alike). It is not re-searches, lazy evaluation or the cap. The extra
work is spread over the whole tree: a point or so less cut efficiency per node, compounded over
ten plies, and it grows with how far the pawn table moves from the shipped one - as the tables did
in 29.8. The larger pawn swings fit the slowest fit but not the 1e-7 one. The search's margins were
tuned against the old evaluation; whether they, rather than the table, are what no longer fits is
open. Code and tool on `texel-pawn`; `master` unchanged.

### 29.12 The pruning margins against the new evaluation scale: over the time stop, no game (2026-10-18)

The margins that compare the static evaluation with alpha/beta - `rfp_margin_per_depth` 80,
`futility_margin_base` 120 and `futility_margin_slope` 80, `razoring_margin` 300 - were set against
the evaluation up to v0.46.4. The Texel `K` fitted on the same positions and results fell from
0.003411 (v0.46.4) to 0.003038 (v0.47.2): the evaluation is 1.12x wider, every margin effectively
12% tighter, and today's RFP 80 is the old scale's ~71, next to the 70 that lost in 21.3. The plan
scaled the four by one factor and fixed s = 1.12 for the game before any measurement, behind the
owner's +5% time rule for margins (2026-10-15).

Census, `engines/suprah-0.47.2` against itself with the four values by `setoption`, depth 10, 300
positions per pool (`openings_wide` / `openings_mixed`), time and generated moves:

| s | RFP / futility / razoring | Time | Generated moves |
| :--- | :--- | ---: | ---: |
| 1.0 (control) | 80 / 120 + 80 / 300 | 0.0% / +0.1%, 300/300 identical | 0.0% / 0.0% |
| 0.9 | 72 / 108 + 72 / 270 | -3.4% / -2.2% | -5.7% / -1.2% |
| **1.12** | 90 / 134 + 90 / 336 | **+6.2%** / +2.0% | +6.7% / +2.7% |
| 1.25 | 100 / 150 + 100 / 375 | +8.4% / +8.6% | +10.9% / +8.3% |

s = 1.12 is over +5% on `openings_wide`: no game (plan D3a). The census prices time, not strength;
whether the wider margins win the time back in accuracy only games can say, and the rule keeps
them out. Open, each on its own plan: the owner lifting the rule for this one candidate (it is
within +10%), or the margins one at a time - RFP alone cost +0.4% / +1.0% at 90 on v0.46.4.

### 29.13 The Reverse Futility margin alone, 80 -> 90: shipped in v0.47.3 (2026-10-18)

29.12 left which margin carries the time. RFP alone, 90 fixed before any measurement (80 x 1.12, the
old scale's 80), behind the owner's +5% rule. Census, `engines/suprah-0.47.2` against itself with
`RfpMarginPerDepth`, depth 10, 300 positions per pool (`openings_wide` / `openings_mixed`), time /
generated moves:

| RFP margin | `openings_wide` | `openings_mixed` |
| :--- | ---: | ---: |
| 80 (control) | -0.1% / 0.0%, 300/300 identical | +0.1% / 0.0%, 300/300 identical |
| 85 | +1.7% / +0.3% | +1.6% / +2.7% |
| **90** | **+5.1%** / +3.6% | +3.5% / +5.6% |
| 95 | +5.4% / +2.3% | +8.9% / +15.1% |
| 100 | +9.2% / +13.2% | +6.5% / +3.7% |

90 is over +5% on `openings_wide` by 0.1 point. The control reads to 0.1%, but between different
trees the census is coarser - 95 against 100 on `openings_mixed` runs the wrong way by 11 points of
generated moves - so the 0.1 point is a rule, not a measurement. On v0.46.4 the same step cost
+0.4% / +1.0% (21.3): on the new evaluation RFP cuts far more at 80, the tightening the scale
predicts. **The owner let 90 play** ("90 spielen, leg los"), the +5% rule lifted for this one
candidate. One commit, `rfp_margin_per_depth` 90 in `Config` and `tuning/parameters.json`; three
tests that pinned 80 re-sampled to 90 with their claims unchanged; the candidate identical to
v0.47.2 with `RfpMarginPerDepth=90` (100/100 trees).

**The gate:** SPRT at 1s + 150ms on `openings_wide.txt`, concurrency 4, [0, 5] with the [0, 10]
monitor, against v0.47.2. **[0, 10] H1 after 4138 games** (LLR +3.07, +10.7 Elo), [0, 5] +2.21
undecided. At the end of the run, 4152 games: **+10.6 Elo, paired 95% [+2, +19]** - a stopped run,
not an effect size (rule 7). Pairs 0-2 / 0.5-1.5 / 1-1 / 1.5-0.5 / 2-0 from the candidate's side
192 / 447 / 707 / 501 / 228; no losses on time, White 55.4%, design effect 1.07. The reading was +53
after 118 games, +17 after 956, +13 after 2530 ([0, 10] LLR +2.73), +10 after 3248 (+1.99).

**Smoke gauntlet**, 1s + 100ms, 100 games per pairing: 47.0% against v0.47.2, 58.0% against
v0.47.1, 93.5% against v0.34.0, no losses on time. Shipped as **v0.47.3**, the release binary
identical to the candidate (100/100 trees).

**What it says.** A margin in centipawns is a statement about the evaluation's scale, and five
evaluation releases had moved that scale by 12%. Put back where the scale says, RFP won its games
although it cost 3-5% more time to a fixed depth. Futility and razoring are left at 120 + 80 and
300: scaled together with RFP they cost +6.2% (29.12) and were not played.

### 29.14 The NNUE net measured with the Texel tools (2026-10-19)

The owner asked what Texel tuning could do for the NNUE branch (v0.46.4-NNUE). There the evaluation
is the net alone - after the insufficient-material guard its output is returned with no HCE term -
and the search margins are master's HCE values (RFP 80 x depth, futility 120 + 80 x depth, razoring
300; "margins in HCE units" was review item N4). The net: 768 inputs x 10 king buckets -> 256 per
side, SCReLU, -> 8 output buckets by piece count, `NNUE_SCALE` 400.

`texel-extract --fens` (`5c2ff87`, feature `texel`, release build identical to v0.47.3) wrote the
4,141,547 positions of the data set as `fen;result;set;eval`; a scratch copy of the NNUE branch (not
committed) evaluated them with the embedded net in 13 s. K fitted on the training set, as the tuner:

| | K | MSE train | validation | test |
| :--- | ---: | ---: | ---: | ---: |
| HCE v0.47.3 | 0.003042 | 0.104031 | 0.103732 | 0.102579 |
| NNUE v0.46.4 | **0.001967** | 0.096076 | 0.095701 | **0.088225** |

The net predicts the results far better: -14,350 x 10^-6 on the test set (games of 2026-10-13 to
16, after the net's date of 2026-08-04), where every HCE re-fit here gained 600 to 4,000. Its scale
is 1.55x HCE v0.47.3 and **1.73x the HCE of v0.46.4** the margins were set against - and it varies
with the phase: per output bucket the net's K runs 0.00173-0.00190 over buckets 0-4 (2 to 21
pieces), 0.00215 / 0.00250 / 0.00324 over buckets 5-7 (22 to 33), so the opening's scale is about
HCE's and the middlegame's nearly twice it. A logit blend of net and HCE is best at 100% net (test
MSE 0.088207 against 0.089439 at a quarter HCE): HCE terms on top of the net have no signal here.

Strength, for orientation: v0.46.4-NNUE against v0.47.3, 1s + 150ms, `openings_wide.txt`, stopped
by the owner's word at 290 of 400 games ("dass sie besser ist als HCE ist klar"): 95.5%, +271 =12 -7,
paired +530 Elo [+462, +638]; no losses on time. Every NNUE change is gated against its NNUE
predecessor, not against HCE.

Options (Matt-Magie plan step 34): **A** the margins to the net's scale (futility and razoring were
never set on it); **E** the net's output calibrated per output bucket (8 factors, the phase-dependent
scale above); **B** the output layer re-fitted (4,104 weights, linear on fixed features); C HCE
corrections (no signal in the blend); D training the net (not with 4M positions and no GPU).

### 29.15 NNUE: futility and razoring to the net's scale refused, the output calibration stopped at the fit (2026-10-19)

**Margins (Matt-Magie step 35).** On the NNUE branch futility (120 + 80) and razoring (300) are HCE
values; the net is 1.73x wider (29.14). Census on v0.46.4-NNUE by `setoption`, depth 10, 300
positions per pool: s = 1.0 control 300/300 identical; 1.25 +1.0% / +0.4% time; 1.5 +3.5% / +1.4%;
1.73 +4.8% / +3.1% - within the owner's +5% rule, so 1.73 played: futility 208 + 138, razoring 519
(local branch `nnue-margins`, `2709631`, in the NNUE repository). SPRT against v0.46.4-NNUE, 1s +
150ms, `openings_wide.txt`: **[0, 10] H0** after about 920 games (LLR -2.98, -14.4 Elo); 931 games:
**-14.2 Elo, paired 95% [-30, +2]**, 52% draws, no losses on time. The scale is right; the net does not
need the wider margins - it plays better with the tighter, cheaper search. The branch stays at
v0.46.4-NNUE.

**Output calibration (option E, step 36).** Eight factors per output bucket, f_b = K_b / 0.001967:
56, 58, 57, 58, 62, 70, 81, 105 / 64. Error with one common K: training -396, validation -361, **test
+242** x 10^-6. The phase-dependent scale holds on the older games and not on the October test
games: it belongs to the data, not to the net. Stopped at the pre-set bar; no code, no game.

### 29.16 NNUE: the output layer re-fitted by Texel - no gain on the test set, no game (2026-10-19)

Matt-Magie step 37, option B. With the feature transformer held, the output layer (per output
bucket 512 weights on `clamp(acc, 0, 255)^2` and a bias; 4,104 values) is linear - a Texel fit
proper. Tool on the NNUE repository's local branch `nnue-output-fit` (`da61d55`, feature
`nnue-texel`, release build identical to v0.46.4-NNUE): features of the 4,141,547 positions of 29.14
(2.1 GB, 10 s), Adam from the shipped layer with K held at 0.001967, weights held within +/-127 (the
engine's i32 sum), export byte-identical for the shipped layer; four tests, two of which were shown
to fail on injected faults.

| L2 towards the shipped layer | train | validation | test |
| :--- | ---: | ---: | ---: |
| 1e-8, 1e-7, 1e-6, 1e-5 (alike) | -2186 x 10^-6 | -1945 | **+170** |

Twenty epochs already read -1417 / -1371 / **+276**: from the first steps the fit improves the older
games and worsens the October test games, as the per-bucket rescale of 29.15 did. The bar (a test
gain of at least 200 together with a validation gain) failed: no game. The net predicts the newest
games best as it is; fitted to this data - mostly HCE games of older versions - it moves towards
them. A fit of the net would need positions from its own recent games, many more of them.

### 29.17 The futility margins alone, 120 + 80 -> 134 + 90: refused by SPRT H0 (2026-10-19)

Matt-Magie step 38, after 29.13: futility alone, scaled by 1.12 (fixed before any measurement),
behind the owner's +5% rule. Census, `engines/suprah-0.47.3` against itself with
`FutilityMarginBase` / `FutilityMarginSlope`, depth 10, 300 positions per pool, time / generated
moves:

| s | futility | `openings_wide` | `openings_mixed` |
| :--- | :--- | ---: | ---: |
| 1.00 (control) | 120 + 80 | +0.5% / 0.0%, 300/300 identical | -0.1% / 0.0%, 300/300 identical |
| **1.12** | **134 + 90** | **+0.3%** / -0.1%, 232/300 identical | **+0.2%** / +0.4%, 265/300 identical |
| 1.25 | 150 + 100 | +0.1% / 0.0%, 220/300 identical | +0.4% / +0.4%, 248/300 identical |

Unlike RFP, futility is nearly free in time and leaves most trees to depth 10 unchanged. One commit
on the local branch `futility-scale` (`5256679`), the two defaults in `Config`; futility is not in
`tuning/parameters.json` and no test pinned 120 / 80; tests 269 (286 with `texel`). The candidate
`0.47.4-rc` identical to v0.47.3 with 134 / 90 (100/100 trees).

**The gate:** SPRT at 1s + 150ms on `openings_wide.txt`, concurrency 4, [0, 5] with the [0, 10]
monitor, against v0.47.3. **[0, 10] H0 after 1424 games** (LLR -3.00, -9.8 Elo), [0, 5] -1.25
undecided; the monitor stopped the run on H0 with a non-positive estimate. At the end, 1438 games:
**-8.9 Elo, paired 95% [-23, +5]** - a stopped run, not an effect size (rule 7). Pairs 0-2 /
0.5-1.5 / 1-1 / 1.5-0.5 / 2-0 from the candidate's side 69 / 172 / 262 / 159 / 57; no losses on
time, White 53.4%. The reading was +22 after 64 games, -0.6 after 546, -8.1 after 906.

**What it says.** The scale argument that carried RFP (29.13) does not carry futility: wider
futility margins prune fewer quiet moves near the leaves, and the games did not want that. Futility
stays at 120 + 80; `master` unchanged, the branch kept as evidence. As on NNUE (29.15), the scale
alone does not set a margin - the games do.

### 29.18 E1, `adjust_eval` off: shipped in v0.47.4 (2026-10-20)

Matt-Magie step 39. `adjust_eval` multiplied an evaluation of +/-200 or more below phase 155 by
`255 / (phase + 100)`, at most `max_eval_mult` 2.0: in a pawn ending 199 stayed 199 and 200 became
400. One commit, `max_eval_mult` 2.0 -> 1.0 - `min(mult, 1.0)` is 1.0 and `x * 1.0` is exact, so it
is bit-identical to never firing; the function stays, the way back is one constant.

**Before games.** Texel on the v0.47.3 data (`texel-tune --epochs 0`, K refitted per pipeline):
-403 / -395 / **-437** x 10^-6 train / validation / test (K 0.003038 -> 0.003790), the audit's
signal (29.4, on v0.46.4: -634 / -776) holding after the re-fits. Time to depth 10, 300 positions
per pool: **-0.2%** / -0.5% (`openings_wide`) and **-0.8%** / -0.9% (`openings_mixed`), time /
generated moves; the control v0.47.3 against itself 300/300 identical on both.

**Tests.** Four tests pinned numbers the step had made, each re-sampled with its claim unchanged:
the bare-king material test (5034 -> 4482, its range moved by the same offset); the queen-en-prise
position of the two RFP tests (static evaluation 3276 -> 2457, `beta` 2500 -> 1900, the depth-4 cut
holding to 2000 by scan); the razoring loss of the smothered mate at depth 5, now 200 to 1200 (the
mate survives from 1300 on, where with the step it was lost to 2000 - the queen deficit is no longer
doubled; the Quiescence Search is unchanged).

**The gate:** SPRT at 1s + 150ms on `openings_wide.txt`, concurrency 4, [0, 5] with the [0, 10]
monitor, against v0.47.3. **[0, 10] H1 after 7610 games** (LLR +2.97, +7.9 Elo), [0, 5] +2.75
undecided. At the end, 7622 games: **+7.9 Elo, paired 95% [+2, +14]** - a stopped run, not an effect
size (rule 7). Pairs 0-2 / 0.5-1.5 / 1-1 / 1.5-0.5 / 2-0 from the candidate's side 355 / 820 / 1341
/ 886 / 409; no losses on time, White 54.2%. The reading was -48 after 94 games, -6 after 670, -4
after 1890 ([0, 10] LLR -2.26), +1.8 after 4408.

**Smoke gauntlet**, 1s + 100ms, 100 games per pairing: 56.5% against v0.47.3, 50.5% against
v0.47.2, 91.5% against v0.34.0, no losses on time. Shipped as **v0.47.4**, the release binary
identical to the candidate (100/100 trees).

**What it says.** The step was a defect against every search margin, and the wish it served -
trade when ahead - did not need it, or not at this price. The fits of v0.47.0 to v0.47.2 learned
material and tables with the step in the model; a re-fit without it is the next candidate.

### 29.19 The material re-fitted without `adjust_eval`: shipped in v0.47.5 (2026-10-20)

Matt-Magie step 40, after 29.18. The material of v0.47.4 came from fits that modelled `adjust_eval`.
`texel-tune --free material` on the v0.47.3 data (K 0.003790, fitted on v0.47.4 and held; best epoch
750 of 1000): -419 / -440 / **-785** x 10^-6 train / validation / test.

| mg / eg | pawn | knight | bishop | rook | queen |
| :--- | ---: | ---: | ---: | ---: | ---: |
| v0.47.4 | 116 / 83 | 543 / 309 | 580 / 329 | 782 / 565 | 1883 / 972 |
| **re-fit** | **78 / 123** | **408 / 438** | **433 / 462** | **562 / 776** | **1390 / 1413** |

Not only the endgame rose: the middlegame fell by a quarter. Without the step K grew by 25%, and
against it the middlegame material was too large - so the scale moved against the tables, the
weights and the search margins, all held. Ordered in both phases; `texel-mirror` 0 asymmetric;
`texel-stats` mean |eval| 271.8 -> 297.2, cap rate unchanged (36.3%); `texel-lazy` 9.24% / 2.00%
above 120 / 180 (9.25% / 2.00%). Time to depth 10, 300 per pool: **+4.9%** / +6.7% (`openings_wide`),
**+3.5%** / +2.6% (`openings_mixed`), time / generated moves; control 300/300.

**Tests.** `eval_fig_value_test`: ranges moved with the values. The check-extension min-depth and
budget tests inverted at depth 7 with Late Move Pruning on (39,363 nodes unlimited against 53,563 and
49,038) - the inversion 21.2 recorded for their sibling; with LMP off, as the sibling, every claim
holds again on the new and the old material (scan of depth 6 to 8). The mop-up edge-push test read
the whole evaluation with the White king fixed on b1: centre and semi-centre were 667 and 668 on
v0.47.4 (+10 edge push against -15 proximity, the tables deciding) and tied at 843. With the
owner's choice it was rebuilt: the mop-up bonus alone (with minus without, as the proximity test),
the White king two squares away in every position, 95 / 105 / 135 / 155 exactly; it passes on both
materials and fails with `mopup_center_weight` 0.

**The gate:** SPRT at 1s + 150ms on `openings_wide.txt`, concurrency 4, [0, 5] with the [0, 10]
monitor, against v0.47.4. **[0, 10] H1 after 2944 games** (LLR +3.14, +13.2 Elo), [0, 5] +2.02
undecided. At the end, 2956 games: **+13.1 Elo, paired 95% [+3, +23]** - a stopped run, not an effect
size (rule 7). Pairs 0-2 / 0.5-1.5 / 1-1 / 1.5-0.5 / 2-0 from the candidate's side 136 / 303 / 532 /
328 / 179; no losses on time, White 54.5%. The reading was -3.7 after 742 games, +7.9 after 1234,
+12.9 after 2228, +11.1 after 2512.

**Smoke gauntlet**, 1s + 100ms, 100 games per pairing: 47.5% against v0.47.4, 55.0% against
v0.47.3, 92.0% against v0.34.0, no losses on time. Shipped as **v0.47.5**, the release binary
identical to the candidate (100/100 trees).

**What it says.** Removing a pipeline step changes what the fitted weights should be; 29.18 and
29.19 together are the step and its re-fit. The scale of v0.47.5 is no longer the one the margins of
29.13 and 29.17 were judged on - in the middlegame the evaluation is narrower, in the endgame wider.

### 29.20 The search margins on the scale of v0.47.5, one at a time (2026-10-20)

Matt-Magie step 41. Texel K on the v0.47.3 data: v0.47.3 0.003038 (where RFP 90 won and futility
x1.12 lost), v0.47.5 0.003695 - the same chance of winning at 0.822 times the centipawns, so every
margin is about 22% wider than where it was judged. Factor 0.822 fixed before any game; futility,
then RFP, then razoring, each its own SPRT against the then current release. Narrower margins prune
more, so the +5% time rule does not bind; the census is recorded only.

**Futility, 120 + 80 -> 99 + 66: refused.** Census on `suprah-0.47.5` by setoption, depth 10, 300
per pool, time / generated moves: control 300/300 identical on both; 99 + 66 -1.9% / -3.0%
(`openings_wide`), -0.9% / -0.6% (`openings_mixed`); 84 + 56 -2.3% / -3.7%, +1.6% / +4.1%. Branch
`futility-0822` (`dc83703`), candidate identical to the census build (100/100). SPRT against v0.47.5:
**[0, 10] H0 after 2682 games** (LLR -3.05, -4.0 Elo), [0, 5] -1.11 undecided; at the end, 2696
games: **-4.0 Elo, paired 95% [-15, +7]** - a stopped run, not an effect size. Pairs 0-2 / 0.5-1.5
/ 1-1 / 1.5-0.5 / 2-0 from the candidate's side 154 / 295 / 468 / 290 / 141; no losses on time,
White 52.7%. The reading was +19.8 after 756 games, +6.8 after 1232, +2.8 after 1834. Futility stays
at 120 + 80: with 29.17 (x1.12, -8.9 Elo) both directions have lost - the margin sits where it is.

**RFP, 90 -> 74: not shipped, the run stopped by the owner.** Census on `suprah-0.47.5`: control
300/300 identical on both pools; 74 -5.1% / -4.9% (`openings_wide`), -7.8% / -13.0% (`openings_mixed`)
time / generated moves; 65 -9.4% / -10.8%. Branch `rfp-0822` (`ab4ef7b`; six tests re-sampled - three
pinned 90, three single-position node counts moved from depth 7 to 8, where every claim holds),
candidate identical to the census build and to the commit (100/100). SPRT against v0.47.5: **[0, 10]
H0 after 8634 games with +2.3 Elo** - above 0, so [0, 5] decided by the rule; stopped by the owner at
10,816 games with [0, 5] at LLR -0.81: **+1.4 Elo, paired 95% [-4, +6]** - a stopped run, not an
effect size. Pairs 519 / 1229 / 1902 / 1206 / 552; no losses on time, White 54.1%. The reading was
+25 after 488 games, +17 after 1040, +6.6 after 4358, +3.0 after 7244. RFP stays at 90: on the
scale of v0.47.5, 74 searches 5-8% faster and plays the same. Razoring was not played.

### 29.21 The passed-pawn table re-fitted by Texel: shipped in v0.47.6 (2026-10-21)

Matt-Magie step 42. After 29.20 two Texel probes on v0.47.5 (no code, no games; K 0.003695, 1000
epochs): material + the tables of rook, knight, bishop, queen and king -135 / -130 / -318 x 10^-6
(tables rougher, range 168 -> 217); **the passed-pawn table alone -139 / -128 / -422**, 12 values,
never fitted before. Played as fitted:

| ranks 2-7 | mg | eg |
| :--- | :--- | :--- |
| v0.47.5 (hand-set) | 3 / 13 / 30 / 60 / 106 / 166 | 10 / 40 / 90 / 180 / 320 / 500 |
| **re-fit** | **-11 / -22 / -15 / 3 / 24 / 72** | **34 / 28 / 97 / 116 / 235 / 450** |

The middlegame values fall and turn negative on ranks 2 to 4: an advanced pawn is already paid by
the pawn table and the rank bonuses (the double count 29.4 named), and the fit takes it back here.
`texel-export` changed the two lines only; `texel-mirror` 0 asymmetric; `texel-stats` test
0.101544 -> 0.101048, the positional cap firing on 33.1% instead of 36.3%, mean |positional| 161.7
-> 144.7; `texel-lazy` 2.00% -> 1.73% above 180. Time to depth 10, 300 per pool: **+0.4%** / +2.2%
(`openings_wide`), **+0.8%** / +0.5% (`openings_mixed`); control 300/300. No test changed (269 / 286).
Branch `passers-refit` (`ec6db9f`).

**The run.** SPRT against v0.47.5 at 1s + 150ms, `openings_wide.txt`, concurrency 4. At 822 games
(+30.5 Elo, [0, 10] LLR +2.80) the owner asked for 2000 games regardless of a verdict: the monitor
and the [0, 5] wrapper were killed, the tournament ran on and a watcher stopped it at 2006 games.
**+22.5 Elo, paired 95% [+10, +35]** over 2006 games - a fixed-length reading, not a stopped one;
[0, 10] H1 (LLR +4.58, first crossed near 900 games), [0, 5] +2.61. Pairs 0-2 / 0.5-1.5 / 1-1 /
1.5-0.5 / 2-0 from the candidate's side 88 / 185 / 363 / 243 / 124; no losses on time, White 54.5%.
The reading was +28.3 after 492 games, +30.5 after 822, +25.5 after 1662.

**Smoke gauntlet**, 1s + 100ms, 100 games per pairing: 57.5% against v0.47.5, 53.5% against
v0.47.4, 91.0% against v0.34.0, no losses on time. Shipped as **v0.47.6**, the release binary
identical to the candidate (100/100 trees).

**What it says.** The largest gain since v0.47.0 came from the one evaluation table nobody had
fitted, at no cost in time. The pawn's value is now split between the table, the rank bonuses and
the passer table by data rather than by hand; the rank bonuses (183 / 104 / 47) are the next
candidate of the same kind.

### 29.22 The pawn rank bonuses re-fitted by Texel: stopped at the bar, no game (2026-10-21)

Matt-Magie step 43. The rank bonuses (183 / 104 / 47 in the endgame on ranks 7 / 6 / 5, half in the
middlegame, every pawn) were the last hand-set layer of the three that pay an advanced pawn. The
tuner holds them with the other piece-square scalars even under `--free scalars` (they would trade
against the tables), so `--free scalars` with every other scalar held by `--set` moved nothing. A
new group `--free rank_bonus` frees exactly the three (feature `texel`, release build identical to
v0.47.6; test `test_rank_bonus_alone_moves_only_the_three_bonuses`, red first; branch
`rank-bonus-refit`, `3694d34`). The fit on v0.47.6 (K 0.003730, best epoch 50 of 1000):
183 / 104 / 47 -> 151 / 66 / 39, **-22 / -14 / -100 x 10^-6** train / validation / test - below the
bar of 200 on the test set, no census and no game. After 29.21 the passer table already took back
most of the double count; what is left in the bonuses is small, as 29.11 found for "bonuses off"
(-11 / +4 / -371 on v0.47.2).

### 29.23 A Texel data set from the games of v0.47.x, seven probes (2026-10-21)

Matt-Magie step 44, analysis only. Every fit since 29.6 used a data set whose test games were
v0.46.x SPRT runs. `data0476.bin`: training / validation from the old sources plus the old test set
and the runs of v0.47.0-v0.47.3 (13,146 games); **test = the runs of v0.47.3-v0.47.6** (27,004 games,
252,376 positions). 478,977 games, 4,516,779 positions; `texel-check` 100% within 3 cp (mean 0.20).
On it v0.47.6 reads 0.103021 / 0.102812 / 0.099825 (K 0.003799). Each probe 1000 epochs, the rest
held; error change x 10^-6, train / validation / test:

| probe | train | validation | test |
| :--- | ---: | ---: | ---: |
| 1 material | -19 | +1 | -213 |
| 2 material + tables of rook, knight, bishop, queen, king (L2 1e-10) | -164 | -163 | -241 |
| 3 the pawn table alone (L2 1e-7) | -401 | -376 | -308 |
| 4 the passed-pawn table | -5 | +1 | -11 |
| 5 the rank bonuses | -30 | -24 | -70 |
| **6 bishop pair + the four mobility factors** | **-135** | **-145** | **-400** |
| 7 every scalar | -1138 | -950 | -1738 |

The passed-pawn table (29.21) and the rank bonuses (29.22) are done: nothing left on new games. Probe
6 is the best per value: bishop pair 48 -> 51, mobility knight / bishop / rook / queen 3 / 3 / 2 / 1 ->
**7 / 7 / 4 / 3**. Mobility is counted whatever its weight, so the cost is the tree, not the term;
29.6 saw mobility x3 among the terms that slowed the search. Probe 3 passes the bar on all three sets
for the first time, but the pawn table slowed the search in 29.11 at every strength tried. Probe 7
shows the scalars still hold most of the error, and 29.6 that taking them all costs 58-69% time.

### 29.24 The bishop pair and mobility re-fitted by Texel: shipped in v0.47.7 (2026-10-21)

Matt-Magie step 45, probe 6 of 29.23. `bishop_pair_bonus` 48 -> 51, mobility knight / bishop / rook /
queen 3 / 3 / 2 / 1 -> **7 / 7 / 4 / 3**; -135 / -145 / -400 x 10^-6 on `data0476`. Census by setoption
on `suprah-0.47.6`, depth 10, 300 per pool, time / generated moves: control 300/300 identical on
both; **fitted +4.0% / +1.7% (`openings_wide`), +9.9% / +14.8% (`openings_mixed`)**; half-way (5 / 5 / 3
/ 2) +5.0% / +4.5%, +3.7% / +6.4%. The fitted values held the +10% rule by 0.1 point - a rule, not a
measurement (as 29.13 missed +5% by 0.1) - so they played, as fixed beforehand. `texel-mirror` 0
asymmetric; the cap fires on 37.2% instead of 34.4% of the test positions; lazy distance above 120 /
180 cp 8.95% / 1.73% -> 10.43% / 1.86%: a busier evaluation, as expected.

Tests: the pawn range of `eval_fig_value_test` (66 -> 39, a missing e-pawn opens Black's bishop and
queen) and three single-position node counts each moved to the next depth at which their claim holds
(null-move verification 7 -> 8, check extension 4 -> 5, QS table 5 -> 6). **Found on the way:**
`get_moves` leaves the halfmove clock of the board it was given changed (`... 2 3` -> `... 0 3`), so a
test that runs two searches on one board measures the second on another position - the order
dependence seen in 29.20. Not fixed. Commit `966ece0`.

**The gate:** SPRT against v0.47.6 at 1s + 150ms, `openings_wide.txt`, concurrency 4, [0, 5] with the
[0, 10] monitor. **[0, 10] H1 after 2482 games** (LLR +3.02, +14.4 Elo), [0, 5] +1.88 undecided. At the
end, 2494 games: **+14.2 Elo, paired 95% [+3, +25]** - a stopped run, not an effect size. Pairs 0-2 /
0.5-1.5 / 1-1 / 1.5-0.5 / 2-0 from the candidate's side 115 / 256 / 440 / 284 / 152; no losses on time,
White 53.8%. The reading was +36 after 310 games, +14.6 after 594, +15.3 after 1204, +13.3 after 1724.

**Smoke gauntlet**, 1s + 100ms, 100 games per pairing: 50.0% against v0.47.6, 54.5% against v0.47.5,
95.5% against v0.34.0, no losses on time. Shipped as **v0.47.7**, the release binary identical to the
candidate (100/100 trees).

**What it says.** The new data set paid at once: on the old one this probe was not measured, and the
gain on the games of the current engine was real in play. A busier evaluation cost up to 10% time to
depth and still won - the time stop is a guard, not a verdict.

### 29.25 King safety, stage A - the existing king group re-fitted: stopped at the bar, no game (2026-10-21)

Matt-Magie step 46, stage A, on v0.47.7. Sixteen scalars free (the five king-danger weights, six
shield values, open / half-open king file, the heavy piece on it, rook and bishop attacking the
king), the other 63 held at their v0.47.7 values, `data0476.bin`, K 0.003713.

| epochs | train | validation | test |
| :--- | ---: | ---: | ---: |
| 1000 (best = last) | -127 | -77 | -218 |
| 3000 (one re-run, fixed beforehand; best = last) | -132 | **-89** | -237 |

The bar is test >= 200 and validation >= 100: missed on validation both times, no census and no game.
What the fit wants: the king-danger weights 10 / 50 / 100 / 150 / 200 -> 206 / 397 / **1697 / 2474 /
2398** at 3000 epochs (903 / 1019 / 992 at 1000) - still climbing with almost no gain, the mark of a
rare feature with tiny units: `danger` sums ring squares x 1/1/2/4 (29.3, mean 0.03 cp), so the
weights grow to make it say anything. The open king file 37 -> 88, half-open 20 -> 41, the heavy piece
on it 15 -> 54; the pawn shield shrinks (37 -> 8, kingside 37 -> 18, queenside 25 -> 4). **What it
says:** the data want king danger, but this feature cannot carry it - its unit is the problem, not
its weights. That is stage B: attack units with safe checks and weak ring squares, built at weight 0
and fitted on traces of their own.

### 29.26 Threats, stage A - the thirteen threat values re-fitted by Texel: shipped in v0.47.8 (2026-10-22)

Matt-Magie step 47. The thirteen hand-set threat bonuses (pawn attacks a piece, knight on bishop /
rook, minor on rook / queen, rook on queen, rook and bishop on the queen's line, queen attacked, with
their tempo variants), the other 66 scalars held at v0.47.7, `data0476.bin`, 3000 epochs (best 200).
**-191 / -192 / -188 x 10^-6** train / validation / test: the most even split of any fit, but the test
gain 12 below the bar of 200; **the owner lifted the bar for this candidate** ("b, leg los").

Fitted: pawn attacks a piece 35 -> **113**; knight attacks bishop 5 -> **61**, rook 16 -> **119**
(tempo 9 -> 1, 13 kept); minor attacks rook 13 -> **105**, queen 24 -> **63**; rook attacks queen 20 ->
**114**; rook on the queen's file 10 -> 25, bishop on its diagonal 10 -> 31; queen attacked 53 -> **8**
(tempo 29 -> 6) - the single threats take over what the attacker scan said. Census by setoption on
`suprah-0.47.7`, depth 10, 300 per pool: control 300/300 identical; fitted **+6.1% / +5.4%**
(`openings_wide`), **+1.3% / -1.3%** (`openings_mixed`) time / generated moves; half-way +3.2% / -0.2%.
`texel-mirror` 0 asymmetric; cap 37.2% -> 37.5%; **lazy distance above 120 / 180 cp 10.43% / 1.86% ->
14.11% / 3.07%** - the threats are not in `cheap_eval`.

Tests: three single-position node counts re-sampled by running the real test at each depth -
futility 7 -> 8, null-move verification 8 -> 9, the aspiration sum 5 -> 4 (at 5, 6 and 7 the seeded
window lost). **Found:** a `Service` carries state between searches - the same position and
configuration count differently depending on what ran before in it; with the halfmove clock of 29.24
the second source of order dependence in these tests. Not investigated. Commit `099d578`.

**The gate:** SPRT against v0.47.7 at 1s + 150ms, `openings_wide.txt`, concurrency 4, [0, 5] with the
[0, 10] monitor. **[0, 10] H1 after 2968 games** (LLR +3.05, +13.1 Elo), [0, 5] +2.10 undecided. At the
end, 2980 games: **+13.6 Elo, paired 95% [+3, +24]** - a stopped run, not an effect size. Pairs 142 /
309 / 515 / 338 / 186; no losses on time, White 53.5%. The reading was +10.2 after 2410 games.

**Smoke gauntlet**, 1s + 100ms, 100 games per pairing: 55.0% against v0.47.7, 51.0% against v0.47.6,
87.5% against v0.34.0, no losses on time. Shipped as **v0.47.8**, the release binary identical to the
candidate (100/100 trees).

**What it says.** Common features with a sound unit fit well and won in play; the bar missed by 12
was a bar, not a measurement. The price is a lazy evaluation that is wrong twice as often - the next
lever is to give `cheap_eval` the threats or a margin that knows them.

### 29.27 Lazy evaluation fitted to the evaluation of v0.47.8: margins 200 / 140, shipped in v0.47.9 (2026-10-22)

Matt-Magie step 48. `cheap_eval` (tables, cached pawn structure, scaling) has neither mobility nor
threats; both grew in v0.47.7 and v0.47.8. `texel-lazy` now reads the windows of its search sample
(`fen;alpha;beta`) and counts, at a search and a QS margin, the lazy returns and the wrong ones - the
full evaluation on the other side of the bound (`lazy_return` mirrors `evaluate`; test red first;
`0523eff`). Wrong returns in % of windowed positions, `samples_openings_wide` / `_mixed`:

| | search margin | QS margin |
| :--- | ---: | ---: |
| v0.47.6 at 180 / 120 (target) | 0.111 / 0.095 | 0.660 / 0.542 |
| v0.47.7 at 180 / 120 | 0.102 / 0.090 | 0.677 / 0.573 |
| v0.47.8 at 180 / 120 | 0.172 / 0.124 | 0.922 / 0.740 |
| **(i) v0.47.8 at 200 / 140** | **0.090 / 0.055** | **0.519 / 0.403** |
| (ii) pawn threat in `cheap_eval`, 180 / 120 | 0.151 / 0.117 | 0.809 / 0.680 |
| (iii) (ii) at 200 / 130 | 0.086 / 0.064 | 0.630 / 0.516 |

The margins were the smallest multiples of 10 at or below the target, fixed beforehand. (ii) - the
pawn-threat term, set-wise cheap, added to `cheap_eval` through two shared helpers (the refactor
tree-identical, `fd56399`) - did not reach the target alone. Found on the way: the term counts
attacking pawns, not attacked pieces; a fork counts once. Time to depth 10 against v0.47.8, 300 per
pool: (i) **+4.3% / +2.6%**, (iii) +8.6% / +1.4%; control 300/300 (noise 1.6% this time). By the rule,
the faster: **(i)**, no change to the evaluation; the `cheap_eval` change was dropped. One commit
`97790ea` (`Config`, `tuning/parameters.json`); the futility node-count test moved from depth 8 to 9.

**The gate:** SPRT against v0.47.8 at 1s + 150ms, `openings_wide.txt`, concurrency 4, [0, 5] with the
[0, 10] monitor. **[0, 10] H1 after 3106 games** (LLR +3.09, +12.6 Elo), [0, 5] +2.03 undecided. At the
end, 3122 games: **+12.5 Elo, paired 95% [+3, +22]** - a stopped run, not an effect size. Pairs 146 /
323 / 541 / 375 / 176; no losses on time, White 53.5%. The reading was +39 after 80 games, -6.5 after
964, +1.9 after 1450, +8.3 after 2544.

**Smoke gauntlet**, 1s + 100ms, 100 games per pairing: 50.0% against v0.47.8, 55.5% against v0.47.7,
92.0% against v0.34.0, no losses on time. Shipped as **v0.47.9**, the release binary identical to the
candidate (100/100 trees).

**What it says.** A margin is a statement about how far the cheap evaluation can lie, and every term
that `cheap_eval` does not carry moves it. Measured on wrong returns rather than on the distance
alone, the fix was two constants, and it paid more than the time it cost.


### 29.28 King safety on attack maps, stage B: shipped in v0.47.10 (2026-10-22)

Matt-Magie step 49, B1. Stage A (29.25) showed the data want king danger but the old feature - ring
squares x 1/1/2/4 into five buckets - could not carry it. Stage B builds what that feature lacked:

* **Attack maps** (`494f1d3`, tree-identical, time to depth 10 within noise): per side and piece type,
  their union, and the squares attacked at least twice, from the attack sets the piece terms already
  compute; the defender scan of the ring is folded into the piece loops. Test: the maps against
  `get_attackers_mask` on every square of six positions.
* **A non-linear unit in the Texel tuner** (`37fdc67`): per king `u_mg`, `u_eg` are weighted sums of
  its features; the penalty is `max(u_mg, 0)^2 / 720` and `max(u_eg, 0) / 20` (Ethereal's form), under
  or outside the positional cap. Gradient tested against a central difference, both cap modes; two
  injected faults turn it red. Data format `TEXEL002` (the new scalars move the parameter indices, so
  `TEXEL001` sets do not read any more).
* **The features at weight 0** (`86a9fcd`, tree-identical, +3.6 to +4.0% time): attackers on the ring
  by type, ring hits scaled to nine squares, weak ring squares (attacked by them, defended at most once
  and only by king or queen), safe checks by type, no enemy queen, a constant; gate as Ethereal's
  (attackers > 1 - enemy queens). Also the threats on the maps for step B2 (hanging, lesser attacker on
  minor / rook / queen, safe pawn push). Tests on constructed positions, trace and model with non-zero
  weights (under and outside the cap), colour symmetry.

**The fit** on `data0479.bin` (the games of `data0476`, traced by this build; `texel-check` 100% within
3 cp): the 24 weights started at Ethereal's values (a quadratic term has no gradient at 0), the eleven
other king values free, the bucket danger held at 0, everything else at v0.47.9; 3000 epochs. Error
change x 10^-6, train / validation / test:

| | train | validation | test |
| :--- | ---: | ---: | ---: |
| Ethereal's values, under the cap | -239 | -281 | -416 |
| Ethereal's values, outside the cap | -332 | -391 | -676 |
| fitted, under the cap | -469 | -485 | -570 |
| **fitted, outside the cap** | **-644** | **-642** | **-874** |

Outside the cap won by 157 on validation (rule fixed beforehand: tie below 50 means under). The pawn
shield gives way (central 37 -> 5), the open king file grows (37 -> 73), the heavy piece on it 15 -> 63.
`texel-mirror` 0 of 183,712 asymmetric. Lazy evaluation: at 200 / 140 the search margin missed the
target of 29.27 on `wide` (0.130%), so by the same rule **210** (0.090 / 0.068%), QS stays 140. Time to
depth 10 against v0.47.9, 300 per pool: **+7.1% / +5.1%** (bar +10%). Six tests re-sampled with the
same statement (three node-count depths, the pawn range of the figure-value test, the bishop alignment
position, the feature-test helper); `99b2a1b`.

**The gate:** SPRT against v0.47.9 at 1s + 150ms, `openings_wide.txt`, concurrency 4, [0, 5] with the
[0, 10] monitor. The first start stopped after 8 games on a `sprt.py` error - a pair outcome never
observed sat on the pole of the constrained fit, 0/0 (fixed with a regression test, `06d6dbb`; the 8
games not counted). Restarted: **[0, 10] H1 after about 570 games** (LLR +3.11). At the end, 575 games:
**+50.6 Elo, paired 95% [+27, +74]** - a stopped run, not an effect size. Pairs 22 / 47 / 89 / 84 / 45;
no losses on time.

**Smoke gauntlet**, 1s + 100ms, 100 games per pairing: 51.5% against v0.47.9, 54.5% against v0.47.8,
93.5% against v0.34.0, no losses on time. Shipped as **v0.47.10**, the release binary identical to the
candidate (100/100 trees).

**What it says.** The unit was the problem, as 29.25 read it: with features that count what an attack
is made of, and a penalty that grows faster than the attack, the same data that refused stage A pay for
the extra time several times over. Next: the threats on the same maps (B2), started from here.

### 29.29 Threats on the attack maps, stage B2: shipped in v0.47.11 (2026-10-22)

Matt-Magie step 49, B2, on v0.47.10. The threat features built at weight 0 in 29.28 - hanging pieces
(attacked by us, not defended), a minor attacked by a pawn, a rook by a pawn or minor, the queen by a
pawn, minor or rook, and the pieces our pawns would attack after a safe push (one or two squares,
the target not attacked by an enemy pawn, and defended by us or not attacked at all) - each counted per
victim, mg and eg, read off the attack maps.

**The fit** on `data0479.bin` with the build of v0.47.10 (king safety outside the cap): the ten new
weights from 0, the thirteen older threat values free with them, everything else held; 3000 epochs,
best at 2900. Error change x 10^-6 against v0.47.10: **-265 / -240 / -265** (train / validation /
test) - over the bar, and even across the three sets. New weights (mg / eg): hanging 24 / 73, lesser
on minor 175 / 161, on rook 210 / 38, on queen -45 / 23, safe pawn push 33 / 29. The older pairs count
per attacker and overlap the per-victim terms, so the fit redistributes: pawn attacks piece 113 -> -82,
knight attacks rook 119 -> -124, minor attacks rook 105 -> -131 and queen 63 -> -117, rook attacks
queen 114 -> -67, queen attacked 8 -> 221 (tempo 6 -> 9), knight attacks bishop 61 -> 58, rook on the
queen's file 25 -> 27, bishop on its diagonal 31 -> 45. The net of each threat stays positive; the
differences carry what the data tell apart (one victim attacked twice, a queen attacked by anything).

`texel-mirror` 0 asymmetric. `texel-lazy` at 210 / 140: wrong returns 0.085 / 0.065% (search) and
0.579 / 0.463% (QS), under the target of 29.27 - margins unchanged. Time to depth 10 against v0.47.10,
300 per pool: **-7.5% / +7.8%** (`wide` faster). Three advertised UCI ranges widened to -500..500, their
defaults now negative. Four tests re-sampled with the same statement: the advanced-knight pair (a
knight on f5 in the opening is kicked by e4 or g4 with tempo; the new positions with d5 / h5 against
d4 / h4 allow no safe push), the rook's alignment with the queen as an x-ray through knights on d5 /
e5, the aspiration sum at depth 5 instead of 4, futility at 8 instead of 9; `9a15414`.

**The gate:** SPRT against v0.47.10 at 1s + 150ms, `openings_wide.txt`, concurrency 4, [0, 5] with the
[0, 10] monitor. **[0, 10] H1 after about 3110 games** (LLR +2.95), [0, 5] +2.04 undecided. At the end,
3114 games: **+12.9 Elo, paired 95% [+3, +23]** - a stopped run, not an effect size. Pairs 154 / 316 /
538 / 358 / 191; no losses on time. The reading was -45 after 116 games, +1.3 after 1092, +11.2 after
2394.

**Smoke gauntlet**, 1s + 100ms, 100 games per pairing: 53.0% against v0.47.10, 51.0% against v0.47.9,
91.5% against v0.34.0, no losses on time. Shipped as **v0.47.11**, the release binary identical to the
candidate (100/100 trees).

**What it says.** On top of king safety the threats were smaller than the fit of the older pairs
(29.26) had been, as expected when both read the same maps; counted per victim they still paid. Stage
B of the evaluation audit (29.2: "attack maps worth it only together with E5") is closed.

## 30. Move order and Transposition Table measured, 2026-10-22

Matt-Magie step 50, measurement only, on v0.47.11. `search-diag` now counts every cutoff (index and
class of the move that cut, remaining depth), every table probe, every store by what it did to the
slot (empty, same key replaced or refused, another key evicted or kept) with the age in searches and
the depth of an evicted entry, and the table's occupancy by age over every 64th slot - the age from a
shadow array that exists only in that build. `scripts/measure_order_tt.py` drives it at a fixed depth
over a pool, or in self-play at a real clock with one process per colour. The shipped build is
unchanged (100/100 trees against v0.47.11 on both pools).

| | depth 10 `wide` | depth 10 `mixed` | 20 games x 20 plies | 20 games x 60 plies |
| :--- | ---: | ---: | ---: | ---: |
| cutoffs on the first move, of all cutoffs | 81.0% | 81.6% | 83.0% | 87.3% |
| ... at remaining depth 1-4 / 9 and deeper | 79-85% / 92% | 80-85% / 92% | 82-85% / 91-95% | 87-88% / 93-100% |
| cutoffs on the second move / the fifth or later | 11.9 / 1.5% | 11.3 / 1.5% | 10.0 / 1.6% | 7.0 / 1.7% |
| the move that cut: TT / capture / killer-counter / quiet | 25 / 46 / 24 / 4.5% | 26 / 50 / 20 / 4% | 26 / 52 / 17 / 4% | 28 / 50 / 14 / 4% |
| key hits, main search / Quiescence Search | 33 / 11% | 33 / 11% | 31 / 11% | 32 / 15% |
| static evaluations at a key hit, main / QS | 27.8 / 1.9% | 27.9 / 2.0% | 25.7 / 2.2% | 25.9 / 3.4% |
| stores evicting another key | 1.3% | 1.3% | 7.6% | 16.5% |
| ... of which the victim was deeper | 2.0% | 1.9% | 2.4% | 4.9% |
| ... of which the victim was from this search | 47% | 45% | 5.5% | 2.1% |
| table empty, mean over searches | 99.5% | 99.5% | 90.4% | **73.9%** |

The games are 1s + 150ms with `Hash=64`, the match conditions.

* **25.2 (buckets, ageing) has nothing to do at match conditions.** The table stays three quarters
  empty after 60 plies; what a store evicts is almost always stale (96% at least three searches
  old), and a deeper entry of the running search is lost in 0.1% of the evictions, about 0.02% of all
  stores. A thread that cleans aged entries in the background has nothing to clean either, and would
  cost a core per game. Re-measure before a much longer time control or a much smaller table.
* **25.3 (the static evaluation in the entry) is the table's one lever.** About 26-28% of the main
  search's static evaluations, and 2-3% of the Quiescence Search's, are taken at a node whose entry
  exists - about 11% of all evaluations, worth some 2-3% of the time at the evaluation's share of the
  search. The entry has the room: 16 of the 64 data bits are padding. Only full evaluations may be
  stored, never a lazy return.
* **The move order is in the published range** (85-90% of cutoffs on the first move): 81-87% overall,
  91-100% from remaining depth 9. What is left sits near the horizon (79-88% at depth 1-4), where the
  second move cuts in 7-12%. Captures make half of all cutoffs, so a capture history is the obvious
  candidate; continuation history (24) stays blocked by the absolute SPSA step (23.4).

### 30.1 The static evaluation in the Transposition Table entry: shipped in v0.48.0 (2026-10-22)

Matt-Magie step 51, `task.md` 25.3, on v0.47.11. The entry carries the position's full static
evaluation, White-positive, in the 16 data bits that were padding (`static_eval`, sentinel
`i16::MIN`); a store of the same position without one keeps it, as R20 keeps the move. The main
search and the Quiescence Search take it from the probe they already make and skip `calc_eval`;
otherwise the value they compute is stored with the node's entry - only when it is a full
evaluation (`calc_eval_known_check_full`), never a lazy return. A served value is used where the
window would have allowed a lazy return too. The entry itself first, tree-identical (100/100 on both
pools, `3d5471a`); the search (`3b088d6`). Tests red first: the stored value is the full one at either
colour, a lazy stand pat is never stored, a stored value is read instead of computed.

`search-diag` counts the served evaluations and computes a fresh one beside each
(`scripts/measure_order_tt.py` reports both). Depth 10, 300 positions, `wide` / `mixed`:

| | served, main / QS / all | time to depth 10 vs v0.47.11 | fresh differs |
| :--- | ---: | ---: | ---: |
| (i) at the node's own stores | 20.5 / 1.4 / **8.4%**, 20.4 / 1.5 / **8.3%** | **-1.9% / -2.2%** | 39 / 23 |
| (ii) also evaluation-only entries at RFP, NMP, razoring | 25.4 / 1.7 / 10.4%, 25.0 / 1.7 / 10.2% | -2.8% / -3.2% | 45 / 34 |

(ii) against (i) directly: 0.0% / -1.0%, not faster on both pools, so (i) by the rule fixed
beforehand; (ii) stays on the local branch `tt-static-eval-ii` (`8bc490a`).

**The mismatches are not this change's.** With the pawn cache bypassed in a scratch build, both
pools give 0. The passed-pawn blockade malus (`passed_pawn_blockaded_malus`) reads the enemy pieces
but is cached with the pawn structure under the pawn key, so a position's evaluation depends on which
position filled the cache entry first (7 cp mg, 15 cp eg per blockaded passer) - in v0.47.11 as much
as here. The owner shipped (i) and moved the fix to its own step (30.2).

**The gate:** SPRT against v0.47.11 at 1s + 150ms, `openings_wide.txt`, concurrency 4, [0, 5] with the
[0, 10] monitor. **[0, 10] H1 after 1306 games** (LLR +3.00), [0, 5] +1.73 undecided. At the end,
1318 games: **+21.9 Elo, paired 95% [+7, +37]** - a stopped run, not an effect size. Pairs 47 / 141 /
236 / 152 / 83; no losses on time, White 52.8%. The reading was -8 after 220 games, +15 after 620,
+22 after 990.

**Smoke gauntlet**, 1s + 100ms, 100 games per pairing: 51.0% against v0.47.11, 58.5% against v0.47.10,
91.5% against v0.34.0, no losses on time. Shipped as **v0.48.0**, the release binary identical to the
candidate (100/100 trees).

**What it says.** Two percent of time to depth bought more than two percent usually does at this
control; the served value also replaces lazy returns with exact evaluations where the window allowed
one, which the census cannot price. The check that every served value equals a fresh one is what
found a defect older than the change - the cache that holds the evaluation is only as good as the
evaluation being a function of the position.

### 30.2 The passed-pawn blockade out of the pawn cache: shipped in v0.48.1 (2026-10-23)

Matt-Magie step 52, the defect 30.1 found. `passed_pawn_blockaded_malus` (15; 7 mg, 15 eg per
blockaded passer) read the enemy pieces in front of a passer inside `white/black_pawn_structure_score`,
whose result the pawn cache keeps under a key of the pawns alone - a position took the malus of
whichever position with the same pawns had filled the entry. The term now sits set-wise in
`pawn_dynamic_scores`, which is never cached: `((white_passed << 8) & black_pieces)` against
`((black_passed >> 8) & white_pieces)`, the values a fresh evaluation always gave. `cheap_eval`
reads only the cached structure and no longer sees it. Test red first: a position evaluated after
another with the same pawns filled the cache equals its evaluation with an empty cache, both
colours; the set-wise / per-pawn equivalence test carries the term in its reference (`a230192`).

Checks: the served-evaluation check of 30.1 **0 / 0 mismatches** (`wide` / `mixed`, depth 10, 300
positions); time to depth 10 against v0.48.0 -0.2% / +1.3%; `texel-mirror` 0 asymmetric; `texel-lazy`
at 210 / 140 wrong returns 0.086 / 0.066% (search), 0.580 / 0.465% (QS), as before.

**The gate:** a correctness fix ships unless it hurts - SPRT [-10, 0] against v0.48.0 at 1s + 150ms,
`openings_wide.txt`, concurrency 4. **H1 after 4278 games** (LLR +2.97): **+0.2 Elo, paired 95% [-8,
+8]** - neutral, as a term worth a few centipawns in a few positions should be. Pairs 205 / 493 /
743 / 491 / 207; no losses on time, White 55.3%.

**Smoke gauntlet**, 1s + 100ms, 100 games per pairing: 51.0% against v0.48.0, 54.0% against v0.47.11,
93.0% against v0.34.0, no losses on time. Shipped as **v0.48.1**, the release binary identical to the
candidate (100/100 trees).

### 30.3 The capture history as a tie-break inside one MVV-LVA step: stopped, not shipped (2026-10-23)

Matt-Magie step 53, on v0.48.1. `HistoryTables` holds the quiet history and a capture history
`[moved piece][to][captured kind]` (gravity, `MAX_HISTORY`, aged and reset with the quiet one;
`01cd1f3`, tree-identical). A cutoff of the main search credits the capture that cut with `depth²` and
debits every capture searched before the move that cut; the capture rank adds `entry * 9,999 /
MAX_HISTORY`, a tie-break that can neither jump an MVV-LVA step (10,000 apart) nor move a capture across
the floor of the band (`a044baa`, branch `capture-history`). Time to depth 10 against v0.48.1 -2.3% /
-0.8%, generated moves -4.2% / -0.6%; cutoffs on the first move 81.2 -> 81.7% / 81.7 -> 81.9%.

**The gate:** SPRT [0, 5] with the [0, 10] monitor against v0.48.1, 1s + 150ms, `openings_wide.txt`,
concurrency 4. Stopped by the owner after 5088 games, undecided: **+1.4 Elo, paired 95% [-6, +9]**,
[0, 5] LLR -0.35, [0, 10] LLR -2.36. Pairs 274 / 545 / 878 / 581 / 266; no losses on time. The reading
was +24 after 310 games, +7.7 after 2400, +0.7 after 4790. `master` unchanged.

**What it says.** Two captures in one MVV-LVA step are rare in the tree, so a tie-break has little to
break. Next: the history allowed to cross steps, and loosening the SEE demotion for captures with a
good history (Matt-Magie step 55) - after the fix of the queen-takes-pawn rank below the band floor
(step 54).
