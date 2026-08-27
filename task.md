# Suprah Engine Strength Enhancement Roadmap (`task.md`)

What to build next in **Suprah**, and the record of what has already been tried and failed.
Read "Negative results" before proposing anything: five of the ideas in this document were built,
measured and reversed, and two of them looked excellent on every metric except games won.

---

## 🧭 Start Here

### Where the engine is

| | |
| :--- | :--- |
| Released | **v0.34.0** on `master` (HCE), **v0.34.0-NNUE** on `feature/nnue-evaluation` |
| Throughput | **1.80x** over v0.30.3, from two measured changes on bit-identical search trees |
| Matchplay resolution | **+/-23 Elo at 500 games**, **+/-16 at 1000**, per pairing |
| Uncommitted | nothing — LMP and SEE pruning are merged, both disabled by default |
| Blocked on | one gauntlet, see below. Nothing else is waiting on it. |

The engine searches at roughly 6.5 M nodes/s. Sections 1 and 2 are built and measured; sections 3
to 7 are not started.

### The next action

**Run the gauntlet that decides whether SEE pruning ships alongside LMP — and that cross-checks
both against the released engine.**

The four-way round robin has run: 3000 games, 500 per pairing, 1000ms + 100ms,
`openings_mixed.txt`, PGN at `../matt-magie/lmp_see.pgn`. It answered the top-line question and
left the decomposition open.

| Pairing | Score | Elo (paired) | 95% CI | |
| :--- | ---: | ---: | :--- | :--- |
| base - both | 45.1% | **-34.2** | [-58, -11] | significant |
| base - lmp | 47.0% | -20.9 | [-43, +1] | misses by one point |
| base - see | 52.2% | +15.3 | [-8, +39] | not significant |
| both - lmp | 51.9% | +13.2 | [-10, +37] | **the open question** |
| both - see | 54.1% | **+28.6** | [+6, +51] | significant |
| lmp - see | 54.5% | **+31.4** | [+9, +53] | significant |

Least-squares fit over all six pairings, base = 0: **lmp +19.7, see -8.9, both +28.9**, largest
residual 9.2.

**What is settled.** Enabling both rules is worth roughly **+34 Elo** over v0.34.0. That pairing
excluded zero at every checkpoint of the run (-62 at 70 games, -48 at 152, -43 at 258, -36 at 354,
-34 at 500) and is the only configuration with a proven gain. `see` alone is the weakest of the
four and loses significantly to both `lmp` and `both`.

**What is not settled: where the gain comes from.** `both - lmp` is +13.2 with the interval
[-10, +37]. The additive prediction from the two single-feature ratings is `both = 19.7 - 8.9 =
+10.8` against a fitted +28.9, i.e. an interaction of **+18.1** — and that term is what makes
`both` beat `lmp`. It is the noisiest quantity in the design, a difference of differences, and it
moved +53 / +8.0 / +2.7 / +20.4 / +18.1 across the run. Do not ship on it without resolving it.

**The run's own warning.** `base - lmp` read -32.4 [-62, -4] at 258 games per pairing, excluding
zero, and fell back to -20.9 [-43, +1] by 500. A marginally significant intermediate result in
this harness is not a result. Compare 8.2, where `check_extension_min_depth` went from +34.2 to
-16.0 on re-measurement.

**Next: a gauntlet that settles both open items at once.** `ab-both` as challenger against
`ab-lmp`, `suprah-0.34.0` and `suprah-0.33.1`. This resolves `both - lmp` at a game count that can
actually see +13, and delivers the cross-version gauntlet that
`skills/engine_release_procedure.md` mandates for any change to `search_service.rs` — the whole
round robin above is a self-A/B of four v0.34.0 derivatives and cannot, by rule 2, see a defect
they all share.

Write `../matt-magie/gauntlet_lmp.trn` and run it. All four binaries already exist in
`../matt-magie/engines/` and need no rebuild.

```
# Does bad-capture pruning earn its place next to LMP, and does the pair survive
# a cross-version comparison? Challenger first.
engines = ab-both, ab-lmp, suprah-0.34.0, suprah-0.33.1
time_control = 1000
increment = 100
rounds = 800                      # 1600 games per pairing; ~1.8 h at 45 games/min
pgn = gauntlet_lmp.pgn
engine_options = OwnBook=false, Hash=64, Threads=1
concurrency = 9
openings = openings_mixed.txt
mode = gauntlet
```

Resolving +13 Elo needs roughly 1600 games: the half-width is +/-23.5 at 500, and (23.5/13)^2 * 500
= 1634. At 1000 games it would still read about +/-16.6 and leave +13 unresolved, so 500 rounds is
not enough for this particular question.

**How to read the outcome.**

| `ab-both` - `ab-lmp` comes out | Ship |
| :--- | :--- |
| clearly positive | both flags `true` |
| null or negative | `enable_lmp = true` only; leave `enable_bad_capture_pruning = false` |

Either way `ab-both` and `ab-lmp` must also beat `suprah-0.34.0` in the same run, or nothing ships
— that pairing is the mandated cross-version check and the round robin could not provide it.

**Where everything is.**

| | |
| :--- | :--- |
| Measurement binaries | `../matt-magie/engines/ab-{base,lmp,see,both}`, versions `0.34.0-{BASE,LMP,SEE,BOTH}` |
| Round robin config | `../matt-magie/lmp_see.trn` |
| Round robin PGN | `../matt-magie/lmp_see.pgn`, 3000 games, `Round` tag denominator 3000 |
| Reference releases | `../matt-magie/engines/suprah-0.34.0`, `suprah-0.33.1`, `suprah-0.33.0` |
| Code | `src/search_service.rs` move loop; 8 tests named `test_lmp_*`, `test_bad_capture_*`, `test_pruning_rules_preserve_the_smothered_mate` |

The variants differ only in two `Config` defaults, so rebuilding any of them means editing
`src/config.rs` and `Cargo.toml` per the recipe below. A single stale line in that recipe already
caused all four binaries to be built as `0.34.0-BASE` once and collapse into one PGN row; the
`id name` check catches it in seconds.

**Until it is decided, both flags stay `false`.** `ab-base` reproduced the released v0.34.0 node
count on Kiwipete exactly (1,192,961 at depth 9), so the default path is untouched and nothing
about the current `master` depends on the outcome.

### The backlog after that, in order

| # | Item | Where | Why this order |
| ---: | :--- | :--- | :--- |
| 0 | Ship whatever the gauntlet decides, as v0.35.0 | 1, 2 | Code is written and tested; only the flag defaults are open |
| 1 | Razoring at depth 1 | 3 | Small, self-contained, same move loop |
| 2 | Singular Extensions | 4 | Largest search item; needs TT-move exclusion |
| 3 | `MovePicker` stages 1-3 | 5 | The throughput prize, but read 5.2 and 8.3 before starting |
| 4 | NNUE incremental accumulator | 6 | Only worth it once `use_nnue` is the default path |
| 5 | Negamax refactor | 7 | Pure refactor, no expected Elo, high blast radius. Last. |

### Rules that are not optional

1. **Every search change is priced by matchplay, not by depth or test-suite accuracy.** The
   clearest evidence is the Check Extension frontier restriction in 8.2: it was the best of four
   axes on fixed-time depth and on LCT II, and measured **-26.8 Elo** in games.
2. **A self-A/B cannot see a defect both sides share.** v0.30.0 shipped a regression of roughly
   two hundred Elo (8.1) that four separate 1000-game self-A/B runs could not detect, because
   every one of them pitted a v0.30.x build against another v0.30.x build.
   `skills/engine_release_procedure.md` mandates a cross-version gauntlet for any change to
   `search_service.rs`, `eval_service.rs`, `move_gen_service.rs` or a search parameter default.
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

**Throughput of the harness:** roughly 48 games/minute at `concurrency = 9` on a 20-core host, so
1000 games per pairing over three pairings is about an hour. Do not compile or run tests while a
match is running: 9 concurrent games occupy 9 of the cores and the games are time-controlled.

#### The opening pool

`openings/book_mixed.txt` — **598 lines of mixed 8, 10 and 12 ply**, shuffled, copied to
`../matt-magie/openings_mixed.txt`. Matt-Magie plays one line per colour-swapped game pair, so at
500 rounds every opening is played exactly once and the games stay uncorrelated.

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
| SEE pruning of bad captures, **on its own** | **-8.9 Elo** over 500 games. Loses significantly to LMP and to both-enabled. Shrinks the tree at every depth and buys nothing | 2 |
| Stage-0 short-circuit of the `MovePicker` | **-9.1% throughput**, 13 of 14 positions slower, on a bit-identical tree | 8.3 |
| Removing the `pv_nodes` mutex from move generation | The lock is uncontended and free within noise; the ordering it provides is worth more than it costs | 8.4 |
| `skip_strong_validation` as a proxy for movegen cost | Admits illegal moves and hangs the engine. The parameter no longer exists | 8.4 |

---

## 1. Late Move Pruning (LMP) — built, disabled, decision pending

`[Impact: High]` `[Complexity: Low]`

At low depths, quiet moves appearing late in the move list are statistically irrelevant and can be
skipped entirely rather than searched at a reduced depth. Complements the existing LMR and Futility
Pruning stages, which already run in this loop.

* At depths $1 \le d \le$ `lmp_max_depth`, when **not in check**, prune all further quiet moves
  once the quiet move counter exceeds:
  $$\text{threshold}(d) = \text{lmp\_base\_moves} + 2 \cdot d^2$$
  With `lmp_base_moves = 3` and `lmp_max_depth = 4` this is 5, 11, 21, 35 quiet moves by depth.

**Delivered.** `enable_lmp` (default `false`), `lmp_max_depth` (4) and `lmp_base_moves` (3) in
`src/config.rs`; the prune sits in the `minimax` move loop in `src/search_service.rs` just above
Futility Pruning; UCI options `EnableLmp` / `LmpMaxDepth` / `LmpBaseMoves`; both integers
registered in `tuning/parameters.json` and in the `search_and_ordering` and `all` groups.

**Guards**, all of them load-bearing: not in check, `!is_pv`, non-capture, non-promotion,
`!current_turn.gives_check`, and a mate-score bound on **both** `alpha` and `beta` — the existing
Futility guard bounds only `alpha`, which is incomplete for the minimising side in this asymmetric
`minimax`.

**Measured: +19.7 Elo** alone in the least-squares fit; `base - lmp` is -20.9 [-43, +1] at 500
games, one point short of significance.

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

## 2. SEE pruning of bad captures — built, disabled, decision pending

`[Impact: measured null on its own]` `[Complexity: Medium]`

Captures with $SEE < 0$ are sorted to the end of the move list but still searched. This prunes a
capture outright when $SEE < \text{bad\_capture\_see\_threshold} \cdot depth$. The threshold
tightens with depth, so the rule bites near the horizon and is nearly inert in the upper tree.

**Delivered.** `enable_bad_capture_pruning` (default `false`) and `bad_capture_see_threshold`
(-50) in `src/config.rs`; UCI options `EnableBadCapturePruning` / `BadCaptureSeeThreshold`; the
threshold registered for tuning.

**It cost no extra SEE call.** The move loop already ran `see_ge(..., 0)` on each capture's first
selection in order to demote it. That call now yields the *value* instead of a boolean and serves
both the prune decision and the demotion. The demotion still drops the rank below zero, which is
what keeps the branch from firing twice on the same move — preserve that if you touch it.

**Measured: -8.9 Elo** alone in the least-squares fit. It loses significantly to `lmp`
(+31.4 [+9, +53]) and to `both` (+28.6 [+6, +51]). **On its own the rule is worse than nothing at
`bad_capture_see_threshold = -50`.** Whether it earns its place *alongside* LMP rests entirely on
the unresolved +18.1 interaction — that is what the gauntlet decides.

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
* **Open:** the branch is level with master at `v0.34.0-NNUE` but has not played a game since
  `v0.30.0-nnue`. A gauntlet against `suprah-0.30.0-nnue` should show a large gain, since the
  branch carried the fail-soft regression for its whole dormancy, and it would also confirm that
  `enable_check_extension = false` carries over. That default was measured on the HCE evaluation:
  the extension's cost is a search property and should transfer, but the branch has its own LMR
  tuning, so it is confirmed rather than assumed.
