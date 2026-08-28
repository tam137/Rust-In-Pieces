# Suprah Engine Strength Enhancement Roadmap (`task.md`)

What to build next in **Suprah**, and the record of what has already been tried and failed.
Read "Negative results" before proposing anything: five of the ideas in this document were built,
measured and reversed, and two of them looked excellent on every metric except games won.

---

## 🧭 Start Here

### Where the engine is

| | |
| :--- | :--- |
| Released | **v0.35.0** and **v0.35.1** on `master` (HCE), **v0.35.0-NNUE** and **v0.35.1-NNUE** on `feature/nnue-evaluation` |
| Throughput | **1.80x** over v0.30.3, from two measured changes on bit-identical search trees |
| Matchplay resolution | **+/-23 Elo at 500 games**, **+/-13 at 3000**, per pairing |
| Uncommitted | nothing |
| Blocked on | nothing. Sections 1 and 2 are shipped; the backlog starts at section 3. |

The engine searches at roughly 6.5 M nodes/s, and reaches **depth 9 to 10** at the 1s + 0.1s
match time control. Sections 1 and 2 are built, measured and released; sections 3 to 7 are not
started. Section 10 is the measurement infrastructure and section 11 a proposal.

> [!IMPORTANT]
> **Elo numbers do not carry across machines**, and the work has moved twice. Measurements before
> 2026-08-27 and from 2026-08-28 onward are from the **20-core Intel Core Ultra 7 265HX**
> (`DE-517XPM4`, 15 GB); the deciding gauntlet of 2026-08-27 evening ran on a **12-core AMD Ryzen
> AI MAX PRO 390** (24 threads, 23 GB). Nothing in `../matt-magie/` crosses between them: `*pgn`
> and `books/` are gitignored, so only `.trn` files and the repository itself travel. Compare
> ratings only within one run, and never read two PGNs from different hosts together.

### What was decided, and what shipped

**Late Move Pruning ships enabled; SEE pruning of bad captures does not.** Two runs on two
machines priced them, and v0.35.0 and v0.35.1 were released on 2026-08-28.

The deciding run was a three-way round robin over `ab-both`, `ab-lmp` and `suprah-0.34.0` —
9000 games, 3000 per pairing, 1000ms + 100ms, `concurrency = 14`, `openings_mixed.txt`,
`../matt-magie/gauntlet_lmp2.pgn`.

| Pairing | Elo (paired) | 95% CI | |
| :--- | ---: | :--- | :--- |
| `ab-lmp` - `suprah-0.34.0` | **+19.4** | [+10, +29] | **significant** |
| `ab-both` - `suprah-0.34.0` | +28.8 | [+20, +38] | significant |
| `ab-both` - `ab-lmp` | +3.9 | [-5, +13] | covers zero |

Least-squares over all three pairings: `lmp` **+21.2 +/- 7.5**, `both` **+27.0 +/- 7.4**,
residuals +/-1.8.

**A round robin was used rather than a gauntlet on purpose.** The preceding run on the Ryzen was
`mode = gauntlet` with `ab-both` as challenger, and in gauntlet mode only the challenger plays
everyone. It therefore never paired `ab-lmp` against `suprah-0.34.0`, and the LMP-only
configuration — the one that shipped — had no direct cross-version check. Its rating there was
chained through `ab-both` at +12.0 +/- 17.6, an interval covering zero. **If a run exists to
qualify one configuration, that configuration has to be in a pairing.**

**`both - lmp` is the quantity that decided against SEE pruning, and it is the reason both runs
had to go the distance.** Its trajectory: +13.2, +10.5, +4.7, +2.2 on the Ryzen, then +3.1, +14.9,
+11.6, +12.0, +3.9 here. It crossed zero in both runs and settled near zero in both. Pooled over
4600 games on two machines it is **+3.3, 95% CI [-3.9, +10.5]**. At 78% of the second run it read
+12.0 [+2, +22] over three stable checkpoints and looked converged; it was not. **Treat no
intermediate value of a difference-of-differences as a result, however stable it looks.**

v0.35.1 ships the both-rules configuration anyway, at the user's request, so the two can be
compared by hand at longer time controls and on hardware the harness has not seen. Its changelog
states that v0.35.0 is the configuration the measurement supports.

### The next action

**Razoring at depth 1 — section 3.** Nothing is blocked and nothing is waiting on a measurement.
It is the smallest remaining item, sits in the same move loop as the two rules just shipped, and
`skills/matchplay_measurement_procedure.md` now has everything needed to price it: state the
hypothesis with `sprt.py --plan` before building, then run it as a *does it hurt* question once
the code exists and only the default is open.

Price it against **`suprah-0.35.0`**, which is now the reference release.

### The backlog after that, in order

| # | Item | Where | Why this order |
| ---: | :--- | :--- | :--- |
| 1 | Razoring at depth 1 | 3 | Small, self-contained, same move loop |
| 2 | Singular Extensions | 4 | Largest search item; needs TT-move exclusion. Read 10.5 first — at depth 9 to 10 it fires at plies 0 to 2 and nowhere else |
| 3 | `MovePicker` stages 1-3 | 5 | The throughput prize, but read 5.2 and 8.3 before starting |
| 4 | NNUE incremental accumulator | 6 | Only worth it once `use_nnue` is the default path |
| 5 | Negamax refactor | 7 | Pure refactor, no expected Elo, high blast radius. Last. |

Section 11 is a proposal rather than a backlog item: it starts with a diagnostic that can kill it
for the price of one build.

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

### How to run a measurement

> [!IMPORTANT]
> The full procedure now lives in `skills/matchplay_measurement_procedure.md`, which adds three
> things this recipe does not have: a **sequential stopping rule**, so a run ends when it is
> decided instead of when its round count runs out; a way to **price a run before starting it**;
> and a **permanent anchor opponent**, so results land on one rating scale across runs. Section
> 10 records what those are worth. The recipe below is still correct for a fixed-length run.

```bash
# 1. Build the variants. Matt-Magie sends ONE engine_options string to both engines, so an A/B of
#    two configurations needs two BINARIES, not two option sets.
#
#    The UCI id name is built from CARGO_PKG_VERSION, so two variants built at the same version
#    report the same name and collapse into one row in the PGN. Give each its own suffix:
#
#      - edit the default in src/config.rs
#      - set Cargo.toml to a suffixed semver prerelease, e.g. version = "0.34.0-LMP"
#      - cargo build --release
#      - cp target/release/suprah ../matt-magie/engines/ab-lmp
#      - restore Cargo.toml and src/config.rs
#
#    Do NOT use ./build_and_release.sh for throwaway variants: it is the release pipeline and it
#    rewrites CHANGELOG.md and Cargo.toml. Plain `cargo build --release` is forbidden only for
#    releasing the engine, not for building a measurement binary.
#
#    Verify the variants really differ before spending hours on them: a fixed-depth node count on
#    Kiwipete separates them in seconds and catches a default that did not take effect.

# 2. Write ../matt-magie/<name>.trn and run it.
#      engines = <a>, <b>[, <c>, <d>]
#      time_control = 1000
#      increment = 100
#      rounds = <games per pairing / 2>
#      concurrency = 9
#      openings = openings_mixed.txt
#      engine_options = OwnBook=false, Hash=64, Threads=1
#      mode = round_robin                # or gauntlet with the challenger first
./mm.sh -t <name>.trn

# 3. Read the result per pairing. Both a paired and an unpaired interval are printed; the paired
#    one is the honest one when an openings file was used.
python3 scripts/pairing_elo.py ../matt-magie/<name>.pgn
```

`OwnBook=false` matters: the engine carries a 93,000-entry book compiled into the binary and would
otherwise play it on top of the manager's opening line. Appending to an existing PGN is safe —
`pairing_elo.py` separates runs by the game-count denominator in the `Round` tag.

**Throughput of the harness**, measured on the 20-core Intel host with the same pairing and time
control, only `concurrency` varying:

| `concurrency` | games/min | losses on time |
| ---: | ---: | :--- |
| 9 | 44.2 | none |
| 14 | **71.3** | none |

`concurrency = 14` is the ceiling and is not a free parameter: **at least 25% of the cores must
stay free**, and 14 searching engines plus the Matt-Magie manager occupy 15 of 20. Compute the cap
as `floor(nproc * 0.75) - 1` on any other host, and re-check forfeits with `match_health.py` at any
new setting — a run with a non-zero forfeit count is discarded rather than corrected, because the
forfeits do not fall evenly on the two engines. At 71 games/minute a 9000-game round robin takes
about two hours. Do not compile or run tests while a match is running.

#### The opening pool

`openings/book_mixed.txt` — **598 lines of mixed 8, 10 and 12 ply**, shuffled, copied to
`../matt-magie/openings_mixed.txt`. Matt-Magie plays one line per colour-swapped game pair, so at
500 rounds every opening is played once; beyond that lines repeat, and at 1500 rounds each is
played between two and three times.

**Repetition is not the problem it looks like.** Over the 9000-game run the design effect measured
**1.00, 1.00 and 1.87** across the three pairings, with 6 identical games in 9000 — search is
time-based, so replaying a line does not reproduce a game. Early in a run the estimate is
unreliable and swings between 1.0 and 1.9 on all pairings at once; it settles once each of the 17
four-ply families holds tens of pairs. Read it at the end of a run, not during one.

Two facts that constrain any future pool. `Performance.bin` **saturates at roughly 100 distinct
8-ply lines** because the book picks moves by weight, so a deep pool cannot be built at a single
shallow depth — measured yield is <25 at 4 ply, ~100 at 8, ~275 at 12. A line is a path through
the book tree, so deeper lines are strictly more numerous, and mixing depths takes the union.
Lengths need not match; `apply_opening_line` in Matt-Magie replays the moves without any length
assumption. **Use only even ply lengths**, so White is on move at handover as in every earlier
measurement. Regenerate with `scripts/make_opening_lines.py --plies <n> --count <n> --out <file>`.

### Negative results — do not repeat these

Each of these was built, measured and reversed. The section named gives the numbers.

| What | Result | Section |
| :--- | :--- | :--- |
| Fail-soft Alpha-Beta | **-168 to -209 Elo.** Cause identified: the Transposition Table write. Clamping it recovers ~133 Elo and still does not reach parity | 8.1 |
| Check Extension, as a feature | **-23.7 Elo** over 1000 games. Disabled by default in v0.34.0 | 8.2 |
| Check Extension, frontier only | **-26.8 Elo**, despite being the best of four axes on every non-matchplay metric | 8.2 |
| Check Extension, deep only | **-9.7 Elo** against the unfiltered extension; the earlier +34.2 was a no-book artefact | 8.2 |
| Check Extension, SEE material filter | Deletes the queen sacrifice in Philidor's Legacy; fails the engine's own smothered-mate test | 8.2 |
| SEE pruning of bad captures, **on its own** | **-8.9 Elo** over 500 games. Loses significantly to LMP and to both-enabled | 2 |
| SEE pruning of bad captures, **on top of LMP** | **+3.3 Elo, [-3.9, +10.5]** pooled over 4600 games on two machines. Not shipped in v0.35.0; v0.35.1 carries it for hand comparison only | 2 |
| Reading a difference-of-differences before a run ends | `both - lmp` crossed zero in both runs and looked converged at +12.0 [+2, +22] over three checkpoints before collapsing to +3.9 | Start Here |
| Stage-0 short-circuit of the `MovePicker` | **-9.1% throughput**, 13 of 14 positions slower, on a bit-identical tree | 8.3 |
| Removing the `pv_nodes` mutex from move generation | The lock is uncontended and free within noise; the ordering it provides is worth more than it costs | 8.4 |
| `skip_strong_validation` as a proxy for movegen cost | Admits illegal moves and hangs the engine. The parameter no longer exists | 8.4 |

---

## 1. Late Move Pruning (LMP) — ✅ shipped enabled in v0.35.0 (+19.4 Elo)

`[Impact: High, measured]` `[Complexity: Low]`

At low depths, quiet moves appearing late in the move list are statistically irrelevant and can be
skipped entirely rather than searched at a reduced depth. Complements the existing LMR and Futility
Pruning stages, which already run in this loop.

* At depths $1 \le d \le$ `lmp_max_depth`, when **not in check**, prune all further quiet moves
  once the quiet move counter exceeds:
  $$\text{threshold}(d) = \text{lmp\_base\_moves} + 2 \cdot d^2$$
  With `lmp_base_moves = 3` and `lmp_max_depth = 4` this is 5, 11, 21, 35 quiet moves by depth.

**Shipped.** `enable_lmp` (**default `true`** since v0.35.0), `lmp_max_depth` (4) and
`lmp_base_moves` (3) in `src/config.rs`; the prune sits in the `minimax` move loop in
`src/search_service.rs` just above Futility Pruning; UCI options `EnableLmp` / `LmpMaxDepth` /
`LmpBaseMoves`; both integers registered in `tuning/parameters.json` and in the
`search_and_ordering` and `all` groups. Ported to `feature/nnue-evaluation` as v0.35.0-NNUE.

**Guards**, all of them load-bearing: not in check, `!is_pv`, non-capture, non-promotion,
`!current_turn.gives_check`, and a mate-score bound on **both** `alpha` and `beta` — the existing
Futility guard bounds only `alpha`, which is incomplete for the minimising side in this asymmetric
`minimax`.

**Measured: +19.4 Elo, 95% CI [+10, +29]**, in a direct 3000-game pairing against `suprah-0.34.0`
at 1000ms + 100ms. The least-squares fit over all three pairings of that run places it at
**+21.2 +/- 7.5**. Correcting the pairing's interval for its design effect of 1.87 gives
**[+6, +32]**, still clear of zero. Two earlier readings were weaker and both were undersized: the
four-way round robin put it at +19.7 in a fit but could not resolve `base - lmp` (-20.9 [-43, +1]
at 500 games), and the first gauntlet chained it through `ab-both` at +12.0 +/- 17.6 because
gauntlet mode never paired it with the release.

> [!WARNING]
> **LMP does not reliably shrink the tree in this engine, and the sign is depth-dependent.**
> Searched nodes on Kiwipete with the transposition table enabled move **-6.5% at depth 6, -4.9%
> at depth 7 and +10.9% at depth 8**. The mechanism is PVS: pruning a late quiet move that would
> have produced a beta cutoff turns a cutting node into a fail-low one, and the parent then widens
> its null window and re-searches. LMP buys shallower subtrees at the price of occasional
> re-searches, and which side wins depends on depth. Do not read the large drop in the UCI `nodes`
> field as a speedup — that field counts *generated* moves (rule 4), not searched nodes.
> `test_lmp_changes_the_tree` therefore asserts only that the rule fires.

> [!NOTE]
> **The Philidor canary belongs to LMP, not to section 2.** In `SMOTHERED_MATE_FEN` the square g8
> is empty, so `3.Qg8+` is a *quiet* checking move and cannot reach the capture rule at all. It is
> LMP that prunes quiet moves. Its `give_check_rank_bonus * 10000` ranks it far too early to be
> pruned in practice; the `!gives_check` guard is what makes that a property of the rule rather
> than of the current move ordering. `test_pruning_rules_preserve_the_smothered_mate` covers both.

### 1.1 Advertised UCI defaults drift from `Config::default()` — open

The option strings in `src/threads.rs` are hardcoded literals. A comparison against
`Config::default()` finds **twelve** stale numeric defaults, not one, all from SPSA runs that
updated `src/config.rs` and not `src/threads.rs`:

| Option | Advertised | Actual | | Option | Advertised | Actual |
| :--- | ---: | ---: | :-- | :--- | ---: | ---: |
| `KingOpenFileMalus` | 40 | 37 | | `LazyEvalMinGamePhase` | 50 | 60 |
| `ThreatMinorAttacksRook` | 15 | 13 | | `KnightOutpostTrueMg` | 30 | 29 |
| `ThreatMinorAttacksQueen` | 30 | 24 | | `BishopOutpostTrueMg` | 20 | 21 |
| `ConnectedPassedPawnEg` | 30 | 29 | | `BishopOutpostTrueEg` | 10 | 11 |
| `KingPawnShieldKingside` | 39 | 37 | | `OppositeBishopsDrawScale` | 50 | 51 |
| `KingPieceShieldKingside` | 16 | 15 | | `RookBehindEnemyPassedPawnEg` | 25 | 24 |

No search behaviour depends on it — `scripts/apply_spsa.py` reads `tuning/parameters.json`, not the
UCI output — so this is the facade that GUIs and third-party harnesses read, nothing more. The fix
is to derive the strings from `Config::default()` rather than to correct twelve literals that will
drift again.

The two `check`-type options in this family are current: `EnableLmp` was corrected to `default
true` in v0.35.0 and `EnableBadCapturePruning` to `default true` in v0.35.1, each by hand in the
same commit that moved the corresponding `Config` default. That is the third release in a row to
fix one of these literals by hand — `EnableCheckExtension` was the one in v0.34.0 — which is the
argument for deriving them.

## 2. SEE pruning of bad captures — ⛔ measured neutral, off in v0.35.0, on in v0.35.1

`[Impact: measured null]` `[Complexity: Medium]`

Captures with $SEE < 0$ are sorted to the end of the move list but still searched. This prunes a
capture outright when $SEE < \text{bad\_capture\_see\_threshold} \cdot depth$. The threshold
tightens with depth, so the rule bites near the horizon and is nearly inert in the upper tree.

**Shipped, disabled.** `enable_bad_capture_pruning` (**default `false` in v0.35.0**, `true` in
v0.35.1) and `bad_capture_see_threshold` (-50) in `src/config.rs`; UCI options
`EnableBadCapturePruning` / `BadCaptureSeeThreshold`; the threshold registered for tuning. The
same pair of releases exists on `feature/nnue-evaluation`.

**It cost no extra SEE call.** The move loop already ran `see_ge(..., 0)` on each capture's first
selection in order to demote it. That call now yields the *value* instead of a boolean and serves
both the prune decision and the demotion. The demotion still drops the rank below zero, which is
what keeps the branch from firing twice on the same move — preserve that if you touch it.

**Measured: -8.9 Elo** alone in the least-squares fit. It loses significantly to `lmp`
(+31.4 [+9, +53]) and to `both` (+28.6 [+6, +51]). **On its own the rule is worse than nothing at
`bad_capture_see_threshold = -50`.**

**And it adds nothing on top of LMP.** The interaction that made `both` beat `lmp` in the round
robin — +18.1, the noisiest quantity in that design — never survived a full run. Measured as
`both - lmp`:

| Run | Machine | Games | Result |
| :--- | :--- | ---: | :--- |
| `gauntlet_lmp.pgn`, 2026-08-27 | Ryzen | 1600 | +2.2 [-10, +14] |
| `gauntlet_lmp2.pgn`, 2026-08-28 | Intel | 3000 | +3.9 [-5, +13] |
| pooled | both | 4600 | **+3.3 [-3.9, +10.5]** |

The two machines differ by +1.7 Elo on this quantity, well inside noise. Pooling is defensible
here in a way that pooling absolute ratings is not: `both - lmp` is an internal contrast between
two binaries inside one run, so it does not depend on the host's speed the way a rating against a
frozen opponent does.

The rule measures neutral, not harmful, and it costs nothing at runtime. **It is not disproven —
it is smaller than this harness can resolve.** Separating +5 from zero would need roughly 6000 to
8000 games in that single pairing, more than the whole deciding run. v0.35.0 therefore ships it
off and v0.35.1 ships it on, so the question can be carried by hand testing rather than reopened
as a measurement.

Unlike LMP it shrinks the tree at every depth measured: **-4.2 / -7.7 / -7.7%** at depths 6, 7, 8
without the transposition table, **-2.8 / -7.0 / -8.4%** with it. It bought fewer nodes and no Elo,
which is the same lesson as 8.3 from the other direction.

> [!NOTE]
> The Philidor warning that used to sit here was **filed against the wrong rule**. In
> `SMOTHERED_MATE_FEN` the square g8 is empty, so `3.Qg8+` is a *quiet* checking move: it cannot
> reach the capture rule at all. The rule that could delete it is LMP. See the note in section 1.
> The `!gives_check` guard is kept here anyway, for a checking sacrifice that *is* a capture.

## 3. Razoring at depth 1

`[Impact: Medium]` `[Complexity: Medium]`

* At depth 1, if `static_eval + razoring_margin < alpha`, run a Quiescence Search directly. If the
  result is still below alpha, return that score immediately.
* **Tasks**:
    - `[ ]` Add `enable_razoring: bool` and `razoring_margin: i16` to `Config`, exposed via UCI.

## 4. Singular Extensions (SE)

`[Impact: High]` `[Complexity: High]`

* **Trigger**: at non-root PV nodes with depth $\ge 8$, when a TT entry exists with
  `depth >= search_depth - 3`, `entry_type == LowerBound | Exact`, and a valid TT best move.
* **Verification search**: a reduced search at $\text{depth} = (\text{depth} - 1) / 2$ with the
  singular window $[\text{tt\_eval} - s, \text{tt\_eval} - s + 1]$ where $s = 2 \cdot \text{depth}$,
  **excluding the TT move**.
* **Action**: if no other move meets the threshold, extend the TT move by $+1$ ply.
* **Tasks**:
    - `[ ]` Add `enable_singular_extensions`, `singular_margin`, `singular_depth_reduction`.
    - `[ ]` Acceptance: a tactical suite confirms the $+1$ ply on forced tactical moves.

> [!NOTE]
> SE grants extra plies, which is exactly what the Check Extension did before it was measured at
> **-23.7 Elo** (8.2). The mechanism that made that expensive — an extension spends its ply at the
> node class where Null Move, RFP and Futility Pruning are all disabled — does **not** apply here,
> because SE extends a TT move at a PV node rather than every checking move. Price it against
> v0.34.0 the same way regardless, and do not assume the sign.

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

* **Build stages 1-3 together, never Stage 0 alone.** Stage 0 is worth 19.1% and was measured at
  **-9.1% throughput** (8.3). The prize is the **57.2%** of interior nodes that cut before a quiet
  move is generated — roughly three times Stage 0.
* **Stage 1b is the structural obstacle and it is small.** A quiet move giving check carries
  `give_check_rank_bonus * 10000` = 50,000, which is what lets quiets outrank captures today and
  is why the current order cannot be produced lazily at all. It is 2.6% of interior nodes. Moving
  that bonus out of the rank function into stage assignment unlocks every stage below it.
* **Killers and counters need no generation at all** — two or three remembered moves validated
  against `NodeMasks`, for 10.4%.
* Stage 0's ceiling is presence, not quality: the PV/TT move cuts at 77.5% of nodes where it
  exists, and 24.2% is simply the TT hit rate on interior nodes.

### 5.2 The constraint that governs the whole item

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

## 7. Negamax refactor — last

`[Impact: Low]` `[Complexity: High]`

`src/search_service.rs` uses an asymmetric `minimax` with parallel `if white { ... } else { ... }`
blocks across every pruning rule, PVS null-window and TT update. Converting to canonical negamax
is a pure refactor with **no expected Elo**, and it would touch every pruning stage at once in a
file whose last large change cost 209 Elo. Do it only when nothing better is available.

---

## 8. Negative results in detail

### 8.1 Fail-Soft Alpha-Beta — ⛔ tried and reverted (v0.30.0 → v0.30.3)

Initialising the running score to `i16::MIN` / `i16::MAX` instead of to the window bound was
released in v0.30.0 and **reverted in v0.30.3**. It is one of the most reliable Elo gains in the
literature, and in this engine it costs roughly two hundred rating points.

| Comparison | Games | Elo |
| :--- | ---: | ---: |
| v0.29.1 vs. v0.30.0 | 60 | **+206.7** for v0.29.1 |
| v0.29.1 vs. v0.29.1 + fail-soft only | 60 | **+168.4** for v0.29.1 |
| v0.29.1 vs. v0.29.1 + bound classification only | 60 | +34.9, not significant |
| v0.30.3 (revert) vs. v0.30.2 | 80 | **+208.7** for the revert |

The bisection isolates the running-score initialisation as the sole cause; the TT bound
reclassification that shipped in the same commit is innocent and was kept.

**Mechanism — identified.** Nothing deterministic reveals it: fixed-depth node counts, scores,
best moves and principal variations are all comparable to v0.29.1, mean completed depth is
identical, and all tests pass. The reason is that **every benchmark in this repository sends
`ucinewgame` before each position and therefore searches with an empty Transposition Table**,
while a played game accumulates entries across eighty moves. Measuring one build twice over a
fixed 60-move sequence — once with the table cleared per position, once left to accumulate —
isolates it:

| Build | positions drifting > 50cp | mean drift | max drift |
| :--- | ---: | ---: | ---: |
| fail-hard (v0.29.1) | **0 / 60** | 5.5 | **31** |
| fail-soft (v0.30.0) | 2 / 60 | 16.8 | **522** |

Clamping the value *written to the table* back into the window actually searched — while still
returning the unclamped score to the parent — restores fail-hard's stability completely.
**Fail-soft's out-of-window values are harmless as return values; they are poison once they enter
a table that outlives the move.** LMR is not the specific culprit: disabling it improves both
builds by a similar factor, so it is a general amplifier rather than a fail-soft-specific trigger.

**The fix is only partial in Elo terms.** The table-write clamp (`ab-ttclamp`) recovers roughly
133 of the 168 Elo — down to -35.3 against v0.29.1 over 79 games, 95% CI [-97, +24] — and still
does not reach parity.

* **Should it be attempted again**: the cold-versus-warm drift measurement is the cheap
  deterministic gate. It exposed in minutes what four 1000-game matches could not. Any retry must
  keep it at fail-hard levels *before* a single game is played.
    - `[ ]` Establish whether the residual -35 is real: 79 games resolve only about +/-60 Elo, so
      500+ games against v0.30.3 are needed to tell a small regression from noise.
    - `[ ]` If real, audit whether the pruning stages that consume the return value (LMR, futility,
      PVS null-window) remain sound against an unclamped score.
    - `[ ]` **Do not pursue "never store deeper than searched" as the fix.** Sound in general, but
      it is not what the measurement identified here.
    - `[ ]` Write the warm-table consistency regression test. The current suite passes completely
      on the broken build.

Binaries and PGNs are kept outside the repository in `../matt-magie/engines/` and `../matt-magie/`:
`ab-bisA` (fail-soft only), `ab-bisB` (bound reclassification only), `ab-revert`, `ab-ttclamp`,
and `ab_bisect.pgn`, `ab_revert.pgn`, `ab_gauntlet.pgn`, `ab_ttclamp.pgn`.

### 8.2 Check Extensions — ⛔ delivered v0.29.0, disabled by default v0.34.0

Implemented in v0.29.0: a move giving check was searched at `depth - 1 + 1`, keeping remaining
depth constant along the forcing line, with termination enforced by a hard `MAX_PLY` ceiling at
node entry. It works as designed — it resolves Philidor's Legacy a ply earlier and solves more
LCT II positions at fixed depth. It also loses games.

**The decision run: 1000 games per pairing at 1000ms + 100ms, 598 mixed opening lines each played
exactly once.** Three configurations of the same v0.33.1 tree, verified distinct beforehand by node
count on Kiwipete at fixed depth 9 (off 1,192,961 < deep 1,616,835 < unfiltered 2,047,451).

| Comparison | Elo (paired) | 95% CI |
| :--- | ---: | :--- |
| unfiltered - deep | -9.7 | [-26, +6] |
| **unfiltered - off** | **-23.7** | **[-40, -8]** |
| deep - off | -16.0 | [-31, -1] |

**Disabling the extension is worth +23.7 Elo**, and `check_extension_min_depth` does nothing.
`enable_check_extension` ships `false` from v0.34.0. The five shaping parameters are kept as
tunables; their unit tests now enable the extension explicitly, since the default no longer does.

**The mechanism.** An extension spends its extra ply at precisely the node class where every
pruning stage is disabled: Null Move, Reverse Futility and Futility Pruning are all guarded by
`!turn.gives_check`, and LMR never reduces a checking move. Measured cost of the unfiltered
feature: -1.06 to -1.49 ply at 1s per move, 1.75x the nodes at fixed depth, in-check share of
interior nodes rising from 17.8% to 38.9%, and 59% of extensions granted to checks with $SEE < 0$.

**Four cost-control axes were built and none rescued it.** All remain available as tunables.

| Axis | Parameter | Verdict |
| :--- | :--- | :--- |
| Frontier only | `check_extension_max_depth` | **-26.8 Elo.** Best of four axes on depth and LCT II, worst in games. Near the horizon QSearch already resolves checks, so the extension there is close to pure cost. |
| Deep only | `check_extension_min_depth` | **-16.0 Elo.** Had measured +34.2 on the bookless harness; the sign reversed on re-measurement. It was pool variance, not a search property. |
| Material filter | `check_extension_require_safe` | Rejected. Gating on $SEE \ge 0$ deletes `3.Qg8+` of Philidor's Legacy and fails the engine's own smothered-mate test. |
| Per-path budget | `check_extension_budget_divisor` | Ineffective. Removes only 6% of the tree; the cost is the *first* extension on each path, which any budget grants. |
| One-Reply Extension | `enable_one_reply_extension` | Cheap (-0.20 ply) and preserves forced sacrificial lines, but on its own does not reproduce the tactical benefit. |

**Withdrawn as a consequence.** "Extend at the Root" — `get_moves` searches every root move at
`depth - 1` unconditionally, so a checking move was treated differently at the root than at every
other ply. With the extension off there is no asymmetry left to correct. Likewise the SPSA item:
all five parameters shape a feature that no longer runs. Re-open both only if
`enable_check_extension` is ever set back to `true`.

**See section 11 before treating this as closed.** The guards that make an extension expensive
also exempt every checking move from LMR, futility, LMP and SEE pruning, whether or not the
extension runs. Whether that exemption is itself the cost has not been measured.

**This is also the clearest evidence in the repository that depth and test-suite accuracy are not
adequate proxies for playing strength.** The frontier restriction was rated the most promising axis
by a wide margin on every non-matchplay metric, and matchplay reversed the verdict completely.

### 8.3 Stage-0 short-circuit — ⛔ built, node-identical, measured negative, reverted

Searching the PV/TT move before generating anything, so that a cutoff never pays for generation,
ranking or buffer initialisation. Implemented, verified bit-identical, and **slower in every
configuration measured**. Reverted from `master`; preserved on `experiment/stage0-short-circuit`
(commit `ca45d7a`), never released.

| Configuration | vs. v0.32.0 |
| :--- | ---: |
| Stage 0 + entry-time history snapshot (tree bit-identical) | **-3.0%** |
| Stage 0 + live history (tree differs) | **-21.4%** |
| Paired A/B, median over 14 positions | **-9.1%** |

13 of 14 positions lose; only 1 is faster and that by 1.9%, inside the noise. The 16 KB history
snapshot costs about as much as the generation it saves. Sharing the PV-map and TT lookups between
Stage 0 and generation (`OrderingLookups`) was tried and **the double lookup was not the cost** —
it stayed negative at -8.8%.

On `master` only the measurement apparatus remains: `src/search_diag.rs`, the `search-diag` feature
and its call sites, and `scripts/measure_stage0.py`, `scripts/verify_stage0_identity.py`,
`scripts/measure_stage0_throughput.py`. The latter two set `EnableTtMoveFirst`, a UCI option that
no longer exists on `master`; they run only against the branch.

### 8.4 Move generation cost breakdown — why Milestone 1's projection was never reachable

`perf` is unavailable on this host (WSL2, no matching `linux-tools`), so the profile was taken by
**duplication**: a diagnostic build performs a piece of work twice and discards the copy, and the
wall-time delta is that work's cost. The tree stays bit-identical, so there is no measurement bias.
Measured on v0.31.0 over 14 positions at fixed depth 10, 17,662,630 nodes.

| Component | Share of total runtime |
| :--- | ---: |
| **Interior move generation, total** | **84.4%** |
| — of which `do_move` + `undo_move` | **27.6%** |
| — of which `MoveList::new()` buffer init | 3.1% |
| — of which the two `get_attackers_mask` calls | ~0% (within noise) |
| — remaining: bitboard generation and move ranking | ~54% |
| Everything else: eval, TT, search logic, QSearch | ~16% |

* The attacker masks are free; **the validation cost was the move making**. v0.32.0 removed that
  27.6% in full and returned **+34.8%**, against the 1.38x the profile predicted. The shortfall is
  the cost of the masks that replaced it.
* **Generation and ranking are the bigger half at ~54%.** That is what section 5 attacks.
* **The original "+300% to +600% NPS" was never reachable from these items.** Even driving interior
  move generation to zero caps the speedup at 6.4x. A realistic joint ceiling is **2x to 2.5x**;
  1.80x is delivered.
* `skip_strong_validation` is **not** a usable proxy for any of this — it admits illegal moves and
  hangs the engine, and no longer exists.
* Removing the `pv_nodes` mutex from the generator inner loop: **measured, premise wrong.**
  Disabling the block entirely moves throughput from 5.26 to 5.13 M nodes/s, i.e. it is free
  within noise, and the ordering it provides is worth more than it costs.

---

## 9. The NNUE Branch

`feature/nnue-evaluation` carries its own SPSA-tuned parameters and is **never merged** — master
changes are ported selectively. See `skills/nnue_porting_and_release_procedure.md`.

* Branch-owned, never take from master: `CHANGELOG.md`, `build_and_release.sh` (only its copy
  appends the `-nnue` binary suffix), `skills/engine_release_procedure.md`,
  `skills/nnue_porting_and_release_procedure.md`, `src/nnue_service.rs` (it embeds the network via
  `include_bytes!`), `src/eval_service.rs` (dead-draw guard ahead of the NNUE hook), and `tuning/`.
* Protected `config.rs` values: `use_nnue = true`, `lmr_divisor = 140` with its `lmr_table`,
  `lmr_move_threshold = 2`, `lmr_history_bad_threshold = 550`,
  `aspiration_window_initial_delta = 16`, `aspiration_window_multiplier = 5`,
  `your_turn_bonus = 18`. The `UseNNUE` UCI option is advertised as `default true`.
* Everything else in `src/` can be taken from master wholesale.
* The branch is level with master at **`v0.35.1-NNUE`**. The v0.35.0-NNUE port was larger than a
  default flip: LMP and SEE pruning landed on master *after* v0.34.0 and had never been ported, so
  the release brought across five `Config` fields, the prune logic, five UCI options, three SPSA
  registrations and eight tests. All protected values were verified unchanged after the
  cherry-pick and `git diff HEAD -- src/config.rs` showed additions only.
* **Open: the branch has not played a game since `v0.30.0-nnue`.** A gauntlet against
  `suprah-0.30.0-nnue` should show a large gain, since the branch carried the fail-soft regression
  for its whole dormancy, and it would confirm three defaults that were all measured on the HCE
  evaluation and merely assumed to transfer: `enable_check_extension = false` (-23.7 Elo on HCE),
  `enable_lmp = true` (+19.4) and the choice to leave bad capture pruning off. Each is a search
  property that should carry, but the branch has its own LMR tuning, so none is confirmed.

---

## 10. Measurement infrastructure

Built 2026-08-27, while the LMP/SEE gauntlet occupied the machine. The engine was not touched;
everything here is analysis of the harness the engine is measured in.

The problem it addresses: at 40 games per minute, every remaining backlog item costs one to two
hours of wall time per decision, and five of them are left. Throughput of the *measurement*, not
of the search, is what now paces the project.

### 10.1 What was added

| Tool | Answers |
| :--- | :--- |
| `scripts/sprt.py` | Is this comparison decided yet? Pentanomial GSPRT over paired openings. |
| `scripts/sprt.py --plan` | How many games will this comparison need, before it is started? |
| `scripts/sprt.py --trajectory` | Where would a finished run have stopped? |
| `scripts/run_sprt_match.sh` | Runs a tournament and ends it at the first decision. |
| `scripts/match_health.py` | Is this run trustworthy at all — forfeits, duplicates, colour bias, pair shape, design effect? |
| `scripts/version_curve.py` | One rating scale across runs, anchored to a frozen opponent. |
| `scripts/book_lines.py` | Walks a PolyGlot book directly. Surveys every book in `books/` for breadth, and builds pools from any of them without the engine. |
| `scripts/test_sprt.py` | 20 tests over the statistics, standard library only. |

### 10.2 The test is pentanomial, and why that is not a detail

Matt-Magie plays one opening line twice with the colours swapped. The pair is the observation,
and its normalised score is one of five values. `pairing_elo.py` already treats it that way for
its interval; the SPRT does the same, so the two agree by construction.

The cost of a comparison is set by the **variance of that pair distribution**, not by the score.
Suprah's is high — measured 0.060 to 0.069 across three pairings, with only about a third of
pairs coming back level. It is a sharp engine, and sharp engines are expensive to measure.

### 10.3 Stating the hypothesis is worth more than running more games

`sprt.py --plan`, on the pair distribution this harness actually produces:

| True effect | `--elo0 0 --elo1 10` "does it gain?" | `--elo0 -10 --elo1 0` "does it hurt?" |
| ---: | ---: | ---: |
| +20 Elo | 1340 games | 806 games |
| +13 Elo | 2490 games | 1106 games |
| 0 Elo | 3970 games (accepts H0) | 3940 games (accepts H1) |
| −13 Elo | 1100 games | 2476 games |

Both columns test the same games. The second asks whether a rule that is already written and free
to keep does any harm, which is the actual decision whenever the code exists and only the default
is open. For the LMP/SEE question it is **less than half the cost** at the effect size in play.

The first row of the fixed-length plan for that same question was 1600 games per pairing, sized
from a half-width. At the measured variance that resolves about ±12.5 Elo and the effect being
hunted is +13.2 — the run was sized to be marginal, and no amount of care in reading it fixes
that. `--plan` would have said so in a second.

### 10.4 What was measured about the harness

**Losses on time: zero** in the first 700 games at `concurrency = 9` on the 12-core Ryzen, and
zero again in all 9000 games at `concurrency = 14` on the 20-core Intel host. Matt-Magie writes
`WhiteWinByTime` / `BlackWinByTime` into the `Termination` tag, so this is directly checkable, and
`match_health.py` checks it. Raising concurrency is a legitimate way to buy throughput — it bought
**+61%** here — but it must be re-checked at the new setting, it is capped by the 25% core-headroom
rule in "How to run a measurement", and any run with a non-zero forfeit count is discarded rather
than corrected.

**Duplicate games: none.** Search is time-based, so replaying an opening line does not reproduce
a game. The pool being shorter than the run is therefore not the problem it looked like.

**The narrow opening tree does not inflate the intervals.** `openings/book_mixed.txt` has 598
lines but only **17 distinct four-ply starts**, 49 at six plies and 121 at eight, because
`Performance.bin` picks moves by weight and popularity is concentrated. The intraclass
correlation of pair scores within an opening family measures **0.00 to 0.03**, i.e. a design
effect of 1.00 to 1.27. The pool's narrowness is a limit on **what a result means** — Elo over
those opening families — and not on whether its interval is honest.

**It is fixable, and the fix is free.** `books/` holds thirteen PolyGlot books — **not in version
control**, `.gitignore` excludes the directory, so they have to be carried across by hand when the
work moves machines — and
`scripts/book_lines.py` walks them directly rather than asking the engine for a move, so it can
draw **uniformly** instead of by popularity. Measured over 400 sampled 10-ply lines, distinct
four-ply starts: `codekiddy.bin` 307, `DCbook_large.bin` 284, `Elo2400.bin` 279, against
`Performance.bin`'s 17 and `komodo.bin`'s 3. The engine's own book is close to the worst of them
for this purpose, and `komodo.bin` — a deep best-play book with two root moves — is the worst.

A `--temperature` of 0.25 beats both extremes: it yields more distinct prefixes *and* more usable
lines than uniform sampling, because following weight keeps the walk inside the book instead of
running it off the edge. The candidate pool `openings/book_codekiddy_10ply.txt` holds **2000
lines with 831 distinct four-ply starts**, against 598 lines with 17.

`scripts/book_lines.py --self-test` checks the PolyGlot key against the nine published vectors
before any of this is trusted, and the Zobrist table is read out of `src/polyglot.rs` so the
script cannot drift from the engine.

**Whether the wider pool costs or saves games is open.** Broader openings are sharper, sharper
openings raise the pair variance, and higher variance means more games per decision — but sharper
positions also convert small strength differences into results instead of drawing them away.
Which effect wins is empirical: run the same pairing on both pools and compare
`scripts/sprt.py --plan`.

**Depth reached at the match time control.** 1000ms + 100ms yields **depth 9 to 10**; 5s gives 11,
10s gives 12, 30s gives 14. Roughly one ply per tenfold increase in time.

### 10.5 The consequence for the backlog

Every remaining search item is a depth-conditional rule, and they are being priced at a root depth
of 9 to 10.

* **LMP** fires at depths 1 to 4 of 9, a large share of the tree. Its measurement transfers.
* **Razoring at depth 1** fires everywhere. Fine.
* **ProbCut at depth ≥ 5** reaches only the top few plies. Its trigger depth is a tunable and has
  to be set against this harness, not against the literature.
* **Singular Extensions at depth ≥ 8** fire at plies 0 to 2 and nowhere else. The feature as
  specified in section 4 is close to untestable here. Either its trigger depth comes down — which
  makes it a different feature from the published one — or the time control goes up, and one ply
  costs a factor of ten in wall time. **Decide which before building it.**

### 10.6 A defect found while reading the code

`quiet_count` in `src/search_service.rs` stops incrementing at 64, because it indexes the
`searched_quiet_moves` array that feeds the history malus. The LMP threshold is
`lmp_base_moves + 2 · depth²`, i.e. 5, 11, 21, 35, 53, **75**, 101, 131 for depths 1 to 8. From
depth 6 up the threshold is above the cap and **LMP silently never fires**.

`tuning/parameters.json` registers `lmp_max_depth` with `max: 8` and the UCI option advertises
`max 10`, so SPSA can wander over a flat region from 5 to 8 and tune nothing.

**This got sharper with v0.35.0.** At the shipping default of 4 the cap does not bind, so the
release is unaffected and the measurement behind it was valid. But LMP is now on by default, which
means a user raising `LmpMaxDepth` through the UCI facade, or an SPSA run exploring up to 8, gets
silence instead of an error. Both limits advertise a range the rule cannot honour. Documented
under *Known limitations* in the v0.35.0 and v0.35.1 changelogs.

* `[ ]` Count quiet moves in a separate counter that is not bounded by the array length.
* `[ ]` Add a test that LMP still changes the tree at `lmp_max_depth = 8`.
* `[ ]` Until then, lower the advertised `max` in both `src/threads.rs` and
  `tuning/parameters.json` to 5, so neither a GUI nor the tuner can enter the dead region.

### 10.7 Not done, and why

* **Adopting the wider pool.** `openings/book_codekiddy_10ply.txt` is built and 49 times broader
  at four plies, but nothing has been played on it. Two things are unmeasured: whether it costs
  or saves games per decision (10.4), and whether its lines are balanced enough — a line whose
  final position already favours one side is not wrong under colour-swapped pairing, but nobody
  has looked. Balance filtering needs the engine to evaluate every candidate and was not done.
* **Comparing the two pools.** One pairing, run twice, `sprt.py --plan` on each. That settles the
  cost question in one measurement and should precede adopting the new pool.
* ~~**Raising `concurrency`.**~~ Done 2026-08-28: calibrated at 9 and 14 on the Intel host with
  120 games each, `match_health.py` clean at both, and 14 adopted for the 9000-game run. 44.2 to
  71.3 games/minute.

---

## 11. Proposal: reduce late checking moves, then re-price the Check Extension

`[Impact: unknown, plausibly high]` `[Complexity: Low]` — not started, and **measure step 1 first**.

Section 8.2 disabled the Check Extension for +23.7 Elo and explained the cost as "an extension
spends its extra ply at precisely the node class where every pruning stage is disabled". Reading
the guards in `src/search_service.rs` sharpens that, and the sharper version suggests the
extension may not have been the problem.

Two different conditions switch rules off, and they are not the same node class.

| Rule | off when **in check** (`turn.gives_check`) | off when the move **gives check** (`current_turn.gives_check`) |
| :--- | :--- | :--- |
| static evaluation | yes | — |
| Null Move Pruning | yes | — |
| Reverse Futility | yes | — |
| Futility Pruning | yes | yes |
| Late Move Pruning | yes | yes |
| SEE pruning of bad captures | yes | yes |
| **Late Move Reductions** | **no — evasions are reduced** | **yes** |

So LMR does reduce evasions; what it never touches is a move that *gives* check. Together with the
three pruning rules that also exempt it, **a checking move is exempt from every reduction and
every pruning rule in the engine**. The Check Extension then granted that same class an extra
ply on top, which is why it cost 1.75x the nodes at fixed depth.

The question section 8.2 did not ask is whether the exemption itself is right. A quiet move that
gives check late in the move list, with poor history, at low depth, is not obviously worth a full
search — every strong engine reduces it, and reduces rather than deletes it precisely because the
occasional queen sacrifice has to survive.

### The order this has to be done in

1. **Measure the size of the class before touching anything.** `src/search_diag.rs` and the
   `search-diag` feature already exist and already classify a node's first move as
   `QuietCheck`; that measured 2.6% of interior nodes. What is needed is different: the share of
   *searched moves* that give check, and the share of subtree time spent under them. If it is
   small, everything below is worthless and the item ends here for the price of one diagnostic
   build.
2. **Add `lmr_check_damping: i32`, defaulting to a value that reproduces today's tree.** A
   checking move becomes reducible, with its reduction damped the way killers and counter moves
   already are in the same block. A damping large enough to zero every reduction is the current
   behaviour, so the default is bit-identical and provable by node count before a game is played.
3. **Run the smothered-mate canary first.** `test_pruning_rules_preserve_the_smothered_mate`
   covers LMP and SEE pruning; extend it to this. Reduction is safer than pruning but not safe:
   this engine does **not** re-search a reduced move that fails low, so a reduced queen sacrifice
   is effectively skipped. That is the whole risk of the item and the test is the only thing
   watching it.
4. **Price it against `suprah-0.35.0`** by the procedure in
   `skills/matchplay_measurement_procedure.md`, as a *does it hurt* question — the code is a
   damping factor and free to keep.
5. **Only if it gains, re-open the Check Extension.** With checking moves reducible, the
   mechanism that made the extension expensive is weaker, and the four cost-control axes in 8.2
   are all still present as tunables. This is the reason the item is worth doing at all: 8.2
   closed the extension as a feature, not as a question, and the three positive findings it
   recorded — a ply earlier on Philidor's Legacy, more LCT II solutions at fixed depth — were
   never disputed. Only the price was.

> [!CAUTION]
> Nothing above is measured. It is a reading of the guards plus the mechanism 8.2 already
> established, and step 1 exists to kill it cheaply if the class is too small to matter. Do not
> let the fact that it explains a known result stand in for evidence that changing it helps —
> that is exactly the error 8.2 records, where the frontier restriction was the best of four axes
> on every metric except the one that counts.
