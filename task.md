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
| Released | **v0.39.0** on `master` (HCE) since 2026-09-02 — the move order as a total order with nesting bands, **+25.6 Elo**. `feature/nnue-evaluation` is not maintained: work is on `master` in HCE mode only, decided 2026-09-02 |
| Throughput | **1.86x** over v0.30.3, from three measured changes on bit-identical search trees |
| Matchplay resolution | **+/-23 Elo at 500 games**, **+/-13 at 3000**, per pairing — measured on host A. On host C with paired openings: **+/-11 at 2000**, **+/-6.5 at 6000**, the last of these confirmed by v0.39.0's run, which returned [+19, +32] around +25.6 |
| Run cost | a **6000-game** fixed-N run is **2.3 s per game** at concurrency 5, i.e. **under 4 hours**. A 200-game smoke gauntlet is 8 minutes. Pricing one change per run is affordable; bundling changes to save a run is not a saving worth having |
| Blocked on | nothing. The staged `MovePicker` was finished, priced and reversed on 2026-09-03: section 5 is a negative result, and `master`'s search is v0.39.0's again |
| Runs on | **host C (ARM, 8 cores)** since 2026-08-28 — resolve `<mm>` and rebuild the binaries there; nothing from host A or host B runs or transfers. Concurrency cap here is **5**, from `floor(nproc * 0.75) - 1` |


### What has shipped

See the Engines Changelog if needed.


### The next action

**The staged `MovePicker` is finished and dead.** It was made node-identical on 2026-09-03 -- 14
of 14 against `suprah-0.39.0` at fixed depth 11, generating about 55% fewer moves -- and it
measures **-4.1% median throughput**, 0 of 14 positions faster, against a gate of +5.0% and 11 of
14. The accounting is section 5, and `master`'s search is v0.39.0's again.

That closes the only large item on the backlog that was not already parked, so the next action is
a choice rather than a queue. The three cheapest, in the order they look worth doing:

1. **The Quiescence Search never generates en passant** -- section 7. It is a missing move rather
   than a re-ordered one, which is why it ranks above the two below.
2. **`singular_margin`, `singular_tt_depth_margin` and `singular_depth_reduction` shipped
   untuned.** The SPSA infrastructure exists and these three have never been through it.
3. **The negative extension**, the other half of the singular rebate, is still unmeasured.

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
| 1 | En passant is invisible to the Quiescence Search | 7 | A missing move, and deterministic to read |
| 2 | `singular_*` shipped untuned | open table | The SPSA infrastructure already exists |
| 3 | NNUE incremental accumulator | 6 | Only worth it once `use_nnue` is the default path, and it is not |

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
| `singular_margin`, `singular_tt_depth_margin` and `singular_depth_reduction` shipped untuned | open tuning |
| The Quiescence Search never generates en passant: the generator's en passant block is behind `!only_captures` and the Quiescence Search is the one caller that passes `true` — section 7 | defect, unpriced, deterministic to read |
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

### 7.3 The Quiescence Search never generates it — open

The en passant block is behind `!only_captures`, and `generate_valid_moves_list_capture` — the
Quiescence Search's generator — is the one caller that passes `true`. The Quiescence Search can
therefore neither play an en passant capture nor see one in its stand-pat, at any depth.

This is a missing move rather than a re-ordered one, which makes it a different kind of change
from 7.2 and the reason it sits at the top of the backlog. It is deterministic to read the same
way: generate it, then `scripts/measure_tree_size.py` over 300 pool positions before anything
else. Note that fixing it also drags the moves into the lazy Static Exchange Evaluation's reach
inside the Quiescence Search, so 7.1's second bullet applies there too.
