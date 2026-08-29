# Suprah Engine Strength Enhancement Roadmap (`task.md`)

What to build next in **Suprah**, and the record of what has already been tried and failed.
Read "Negative results" before proposing anything: five of the ideas in this document were built,
measured and reversed, and two of them looked excellent on every metric except games won.

---

## 🧭 Start Here

### Where the engine is

| | |
| :--- | :--- |
| Released | **v0.37.2** on `master` (HCE), five defect repairs. The NNUE branch is three releases behind at **v0.36.0-NNUE** on `feature/nnue-evaluation` — none of the v0.37.x ports has been done |
| Throughput | **1.80x** over v0.30.3, from two measured changes on bit-identical search trees |
| Matchplay resolution | **+/-23 Elo at 500 games**, **+/-13 at 3000**, per pairing — measured on host A |
| Uncommitted | nothing |
| Unreleased | nothing. v0.37.2 shipped on 2026-08-29 |
| Blocked on | nothing. The next action is 7.1, and its first step is deterministic and costs no games |
| Runs on | **host C (ARM, 8 cores)** since 2026-08-28 — resolve `<mm>` and rebuild the binaries there; nothing from host A or host B runs or transfers. Concurrency cap here is **5**, from `floor(nproc * 0.75) - 1` |

The engine searches at roughly 6.5 M nodes/s **on host A**, and reaches **depth 9 to 10** at the
1s + 0.1s match time control there. Both figures are host-dependent and unmeasured on host C.
Sections 1 to 4 are built, measured and released, section 7 is released and its two open
questions are now separated — 7.2 is answered, 7.1 is not — and sections 5 and 6 are not started.
Section 10 is the measurement infrastructure.

> [!IMPORTANT]
> **Elo numbers do not carry across hosts**, and the work has moved three times. Runs are labelled
> by host throughout this file:
>
> | Host | What | When |
> | :--- | :--- | :--- |
> | **host A** | 20-core x86-64 | up to 2026-08-27, and again from 2026-08-28 |
> | **host B** | 12-core x86-64 | the deciding LMP/SEE gauntlet, 2026-08-27 evening |
> | **host C** | ARM | from 2026-08-28, the razoring gauntlet onward |
>
> Nothing in the match manager's directory crosses between hosts: `*.pgn` and `books/` are
> gitignored, so only `.trn` files and the repository itself travel. **Compare ratings only within
> one run, and never read two PGNs from different hosts together.** Per-host figures such as
> games/minute and the concurrency ceiling have to be re-measured on each host, never carried
> over.

### What has shipped

| Version | What | Measured |
| :--- | :--- | :--- |
| v0.35.0 | Late Move Pruning enabled | **+19.4 Elo** [+10, +29] vs v0.34.0, 3000 games |
| v0.35.1 | SEE pruning of bad captures also enabled, for hand comparison | +3.3 [-3.9, +10.5] on top of LMP — neutral |
| v0.35.2 | Four defect repairs, no changed default, gauntlet waived | — |
| v0.36.0 | Razoring enabled by default | SPRT H1 accepted (elo0=-10, elo1=0) on host C, **+14.1 Elo, paired 95% CI [-1, +30]**, 1034 games — read as an upper estimate, see 4.3 |
| v0.37.0 | Singular Extensions enabled by default, trigger depth 6 | Two independent SPRTs accepted H1 (elo0=-10, elo1=0); pooled **+10.2 Elo [-1, +22] over 2591 games**, best read as **+5 to +10** |
| v0.37.1 | The negamax refactor. No behaviour change | Node-identical to v0.37.0 on 28 positions at depth 8 and 10. Smoke gauntlet on host C: 52.0% vs v0.37.0, 50.5% vs v0.36.0, 100 games each — neither is a measurement |
| v0.37.2 | Five defect repairs: a colour-asymmetric pawn mask, thirteen inert UCI options, twelve drifting advertised defaults, a dead config field, a dead tuning range | The pawn fix is deterministic — 0 of 21,300 mirror pairs asymmetric, from 65 of 1,905 before. **Not priced**. Smoke gauntlet on host C: 48.5% vs v0.37.1, 52.5% vs v0.37.0, 100 games each, no losses on time |

Sections 1, 2 and 3 carry the numbers and the reasoning. Lessons from those rounds are general
enough to become rules below rather than history: **a run must pair the configuration it exists to
qualify** (rule 2), **no intermediate value of a difference-of-differences is a result, however
stable it looks** — `both - lmp` read +12.0 [+2, +22] over three stable checkpoints at 78% of a
9000-game run and finished at +3.9 — and **a sequential test can decide *does it hurt* long before
its point estimate is precise**: the razoring gauntlet's LLR crossed the H1 bound at 517 pairs
while the running Elo estimate was still swinging between +2.7 and +24.5 a few hundred games
earlier — and, established in v0.37.0, **a stopped SPRT decides whether to ship but overstates by
how much**: the singular-extension pricing run read +30.6 Elo and a 1945-game confirmation run of
the same binary read +6.6. Section 4.3 states the rule; it applies retroactively to the +14.1
above.

### The next action — the empty-window Transposition Table bound (7.1)

7.2 is answered. The colour asymmetry is a **move-ordering tie-break artefact**, not a defect: the
generated move set is the exact mirror image and every move's rank is identical, but the list
order is not mirror-invariant, and LMP, LMR and Futility Pruning all key on a move's index. One
genuine evaluation asymmetry surfaced on the way and shipped as a fix in v0.37.2. The full
measurement, including what it eliminates, is in 7.2.

The next action is **7.1**, the Transposition Table bound stored on an empty `alpha == beta`
window. Its first step is the 8.1 cold-versus-warm gate, which is deterministic and costs no
games. Two things now qualify that step, both established this session:

* Making `bound_for` colour-blind does **not** reduce the mirror asymmetry, so 7.1 can no longer
  borrow 7.2's motivation. It has to stand on its own defect argument.
* There is a **second, independent source of cold-versus-warm drift** in the evaluation, recorded
  in 10.8. Run the gate against both or it will credit 7.1 with drift it does not cause.

### The backlog, in order

| # | Item | Where | Why this order |
| ---: | :--- | :--- | :--- |
| 1 | The empty-window Transposition Table bound | 7.1 | Run the 8.1 cold-versus-warm gate first, against both this and 10.8. Deterministic, costs no games |
| 2 | Lazy Evaluation reads an unfilled pawn hash table | 10.8 | Same gate, same run. A tree change, so it needs pricing after it |
| 3 | `MovePicker` stages 1-3 | 5 | The throughput prize, but read 5.2 and 8.3 before starting |
| 4 | NNUE incremental accumulator | 6 | Only worth it once `use_nnue` is the default path |

> [!IMPORTANT]
> Items 1 and 2 are **not bugfixes to be bundled into a release**. Both move the search tree, so
> rule 1 applies and both need pricing. This document's own history is the argument: fail-soft
> (8.1) is one of the most reliable gains in the literature and cost roughly two hundred Elo here,
> and the Check Extension frontier restriction (8.2) was the best of four axes on every
> deterministic metric and measured -26.8 Elo in games.
>
> v0.37.2 is not a counter-example. The one tree change in it — the pawn mask of 7.2 — was
> released as a **correctness** repair with a deterministic proof and an explicit statement that
> it is unpriced, not as a strength claim.

The `MovePicker` item is the most dangerous in this document — **read 5.2 and 8.3 before writing
any code**. 8.3 is a stage-0 short-circuit that was built, verified node-identical, measured
negative and reverted; 5.2 states the constraint that governs the whole item (the history table
must be snapshotted at node entry). It is a throughput change, not a tree change, so it is priced
differently from sections 1 to 4: the question is nodes per second at equal tree, and only then
games.

The proposal that used to be section 11 — damping the check exemption — was measured on
2026-08-28 and is dead. It is written up as a negative result in 8.5.

### Everything still open, in one place

A new session can start from this table; each row says where the detail is.

| Open | Where | Kind |
| :--- | :--- | :--- |
| The Transposition Table stores an unproven bound at Black nodes on an empty window | 7.1 | defect, needs pricing |
| The root can hand a node an empty `alpha == beta` window | 7.1 | open question |
| Lazy Evaluation compares a `cheap_eval` that is missing the pawn structure on first visit | 10.8 | defect, needs pricing |
| `singular_margin`, `singular_tt_depth_margin` and `singular_depth_reduction` shipped untuned | 4.4 | open tuning |
| `MovePicker` stages 1-3: needs an entry-time history snapshot, and five `#[allow(dead_code)]` attributes to resolve | 5, 5.2 | large item, constraints known |
| NNUE incremental accumulator, and making `use_nnue` the default | 6 | large item |
| The NNUE branch has not played a game since v0.30.0-nnue | 9 | unverified defaults |
| Whether the wider opening pool costs or saves games per decision, and whether its lines are balanced | 10.7 | open measurement |
| `scripts/measure_stage0.py` still uses a fixed `sleep` instead of `uci_driver.py` | 10.1 | unsafe measurement |
| Whether mirror-invariant move generation is worth measuring at all | 7.2 | open question, no prior reason to gain |

Closed in v0.37.2: the twelve stale advertised UCI defaults and the thirteen inert option names
(1.1), the dead `Config::search_threads` field (10.6a), the dead `lmp_max_depth` tuning range
(10.6), and the evaluation half of the colour asymmetry (7.2).

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

### First, resolve `<mm>` — it is not the same on every host

`<mm>` throughout this file is the **Matt-Magie working directory**: where the engine binaries,
`.trn` tournament files and PGNs live. It is a sibling of this repository on some hosts and
somewhere else on others, and the repository is not always checked out at the same place either,
so **resolve it once at the start of a session and use the result** rather than assuming a layout.

```bash
# From the repository root. Take the first hit that contains an `engines/` directory and mm.sh.
find .. ~ -maxdepth 3 -name mm.sh -type f 2>/dev/null
```

Per `AGENTS.md`, the resolved value must not be written back into this file, `CHANGELOG.md` or a
skill document. `build_and_release.sh` and `scripts/run_sprt_match.sh` carry a default and are the
only two places allowed to.

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
#      - cp target/release/suprah <mm>/engines/ab-lmp
#      - restore Cargo.toml and src/config.rs
#
#    Do NOT use ./build_and_release.sh for throwaway variants: it is the release pipeline and it
#    rewrites CHANGELOG.md and Cargo.toml. Plain `cargo build --release` is forbidden only for
#    releasing the engine, not for building a measurement binary.
#
#    Verify the variants really differ before spending hours on them: a fixed-depth node count on
#    Kiwipete separates them in seconds and catches a default that did not take effect.

# 2. Write <mm>/<name>.trn and run it.
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
python3 scripts/pairing_elo.py <mm>/<name>.pgn
```

`OwnBook=false` matters: the engine carries a 93,000-entry book compiled into the binary and would
otherwise play it on top of the manager's opening line. Appending to an existing PGN is safe —
`pairing_elo.py` separates runs by the game-count denominator in the `Round` tag.

**Throughput of the harness**, measured on **host A** with the same pairing and time
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
`<mm>/openings_mixed.txt`. Matt-Magie plays one line per colour-swapped game pair, so at
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
| SEE pruning of bad captures, **on top of LMP** | **+3.3 Elo, [-3.9, +10.5]** pooled over 4600 games on two hosts. Not shipped in v0.35.0; v0.35.1 carries it for hand comparison only | 2 |
| Reading a difference-of-differences before a run ends | `both - lmp` crossed zero in both runs and looked converged at +12.0 [+2, +22] over three checkpoints before collapsing to +3.9 | Start Here |
| Stage-0 short-circuit of the `MovePicker` | **-9.1% throughput**, 13 of 14 positions slower, on a bit-identical tree | 8.3 |
| Damping the check exemption (LMR / LMP on checking moves) | **0.34% and 0.02% of the searched tree.** The move ordering already searches checking moves early, so the two rules would almost never have fired on them | 8.5 |
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
at 1000ms + 100ms on host A. Corrected for that pairing's design effect of 1.87 it is
**[+6, +32]**, still clear of zero.

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
> **8.5 has since priced "far too early to be pruned in practice": with the guard removed, LMP
> would delete 271 moves out of 2.2 M searched, 0.02% of the tree.**

### 1.1 The UCI facade lied twice — ✅ both fixed in v0.37.2

Two independent defects in the same facade, found by auditing the advertised option list against
`Config` and verified end-to-end against the shipped v0.37.1 binary.

**Twelve advertised defaults had drifted**, one SPSA run at a time, because `src/config.rs` was
updated and `src/threads.rs` was not — `KingOpenFileMalus` advertised 40 against an actual 37,
`LazyEvalMinGamePhase` 50 against 60, `ThreatMinorAttacksQueen` 30 against 24, and nine more.
Three separate releases had each corrected one of these by hand.

**Thirteen of the sixty-one advertised options were inert.** `setoption` joined the whitespace
tokens of the name with `_` and lowercased the result, so a single-token CamelCase name collapsed
to `connectedpassedpawnmg` and matched no arm — the arms were written in snake_case, and only
those that happened to carry a hand-written lowercase alias worked. The engine advertised
`ConnectedPassedPawnMg`, `KnightOutpostTrueMg`, `BishopOutpostTrueMg/Eg`, `KingPawnShieldKingside/
Queenside`, `KingPieceShieldKingside/Queenside`, `OppositeBishopsDrawScale`,
`RookBehindEnemyPassedPawnMg/Eg` and `ConnectedPassedPawnEg`, accepted them, and silently ignored
the value. Eight of these are also among the twelve stale defaults, which is why the two defects
were mistaken for one.

SPSA was never affected: `tuning/parameters.json` uses snake_case throughout, and
`scripts/apply_spsa.py` writes `src/config.rs` directly. What it affected is every GUI and
third-party harness, including the `engine_options` line of every `.trn` file.

**The repair is structural, not twelve corrected literals.** The option list moved into
`threads::uci_options(&Config)`, which reads every default from the configuration, so drift is no
longer expressible. The dispatch moved into `Config::apply_uci_option`, which matches the name
case-insensitively with separators removed, so `ConnectedPassedPawnMg`,
`connected_passed_pawn_mg` and `Connected Passed Pawn Mg` are one option. Three tests hold the
line: `test_every_advertised_uci_option_is_accepted`,
`test_every_advertised_spin_option_changes_the_configuration` — which catches an arm that parses
a value and then drops it, not merely a missing arm — and
`test_uci_option_names_ignore_case_and_separators`.

`SyzygyPath` was removed from the advertised list in the same release. There is no tablebase code
in the engine, so advertising it invited a GUI to configure something that does not exist.

## 2. SEE pruning of bad captures — ⛔ measured neutral, off in v0.35.0, on in v0.35.1

`[Impact: measured null]` `[Complexity: Medium]`

Captures with $SEE < 0$ are sorted to the end of the move list but still searched. This prunes a
capture outright when $SEE < \text{bad\_capture\_see\_threshold} \cdot depth$. The threshold
tightens with depth, so the rule bites near the horizon and is nearly inert in the upper tree.

**Shipped. The current default on `master` is `true`**, from v0.35.1 onward; v0.35.0 shipped it
`false`. `enable_bad_capture_pruning` and `bad_capture_see_threshold` (-50) in `src/config.rs`; UCI options
`EnableBadCapturePruning` / `BadCaptureSeeThreshold`; the threshold registered for tuning. The
same pair of releases exists on `feature/nnue-evaluation`.

**It cost no extra SEE call.** The move loop already ran `see_ge(..., 0)` on each capture's first
selection in order to demote it. That call now yields the *value* instead of a boolean and serves
both the prune decision and the demotion. The demotion still drops the rank below zero, which is
what keeps the branch from firing twice on the same move — preserve that if you touch it.

**Measured: -8.9 Elo** alone in the least-squares fit, losing significantly to both `lmp` and
`both`. **On its own the rule is worse than nothing at `bad_capture_see_threshold = -50`.**

**And it adds nothing on top of LMP.** The interaction that made `both` beat `lmp` in the round
robin — +18.1, the noisiest quantity in that design — never survived a full run. Measured as
`both - lmp`:

| Run | Host | Games | Result |
| :--- | :--- | ---: | :--- |
| `gauntlet_lmp.pgn`, 2026-08-27 | host B | 1600 | +2.2 [-10, +14] |
| `gauntlet_lmp2.pgn`, 2026-08-28 | host A | 3000 | +3.9 [-5, +13] |
| pooled | both | 4600 | **+3.3 [-3.9, +10.5]** |

The two hosts differ by +1.7 Elo on this quantity, well inside noise. Pooling is defensible
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
> The `!gives_check` guard here is kept for a checking sacrifice that *is* a capture. The Philidor
> canary itself belongs to LMP — see the note in section 1.

## 3. Razoring at depth 1 — ✅ shipped enabled in v0.36.0 (+14.1 Elo, CI crosses zero)

`[Impact: measured, does-it-hurt confirmed]` `[Complexity: Medium]`

At depth 1, when the static evaluation trails the window by more than `razoring_margin`, one ply
is unlikely to recover the gap and the node's whole move loop is spent proving a fail-low. The
rule runs a Quiescence Search directly and returns that score **only if it confirms the fail-low**;
otherwise the node falls through to the normal search. The qsearch is a verification, not an
assumption.

**Shipped.** `enable_razoring` (**default `true`** since v0.36.0) and `razoring_margin` (300) in
`src/config.rs`; the rule sits in `minimax` as step 0.6, after Reverse Futility Pruning and before
move generation; UCI options `EnableRazoring` / `RazoringMargin`; `razoring_margin` registered in
`tuning/parameters.json`. Seven tests in `src/search_service.rs` (`test_razoring_default_is_on`
replaces the earlier `test_razoring_default_is_off`).

**Guards**: `depth == 1`, `!is_pv`, not in check, and a mate-score bound on both `alpha` and
`beta`. The early return **bypasses the transposition table write**, exactly as Null Move Pruning
and Reverse Futility Pruning do — 8.1 cost roughly two hundred Elo by writing a fail-soft score
into the table, and a rule that returns before the store cannot repeat it.

### 3.1 What it is worth on a fixed-depth corpus

14 positions at fixed depth 11, driven through `scripts/measure_razoring.py`. Node counts are the
sum over every iteration of the iterative deepening; times are the engine's own cumulative
`info ... time`, so process start-up is not counted.

| `razoring_margin` | searched nodes | search time | positions whose move or score changed |
| ---: | ---: | ---: | ---: |
| 150 | **-20.1%** | -15.5% | 9 of 14 |
| 300 | -7.7% | -16.0% | 8 of 14 |
| 500 | -11.4% | **-16.3%** | 6 of 14 |
| 800 | -10.1% | -14.1% | 3 of 14 |

Three things to read out of that table, and one not to.

* **Time falls by about 15% at every margin tested**, including the conservative end where the
  rule changes only 3 of 14 best moves. That is the number that turns into depth in a game.
* **Node count is not monotone in the margin** — 300 saves less than both 150 and 500. Same
  mechanism as LMP (section 1): cutting a node short turns a cutting node into a fail-low one,
  PVS widens the parent's window and re-searches. Do not tune on it.
* **The per-position spread is enormous**: -53% on `Closed Centre`, +49% on `Sharp Tactical`. The
  totals are carried by a few large positions.
* **Do not read a default out of this table.** It is the exact shape of evidence that made the
  Check Extension frontier restriction look like the best of four axes before it measured -26.8
  Elo (rule 1). `razoring_margin = 300` is a literature-consistent starting point, not a finding.

### 3.2 The cost: a mate delivered on the razored horizon

Razoring loses Philidor's Legacy at depth 5, and the margin is not the lever — the loss is
identical at 200, 300, 500, 900 and 1500, because after the queen sacrifice White is a queen down
and no usable margin covers that.

The mechanism is in the Quiescence Search, not in razoring. `minimax` at `depth <= 0` calls
`generate_valid_moves_list_capture` when not in check: **Suprah's qsearch generates captures only
and never a quiet checking move.** `Nf7#` is quiet, so the verification search cannot see the
refutation and returns the fail-low it was given.

**Depth is the lever, and one iteration is enough.** At depth 6, 7 and 8 the mate comes back, and
the tree is smaller than without the rule. Both facts are pinned:
`test_razoring_finds_the_smothered_mate_once_it_is_inside_the_search` and
`test_razoring_loses_a_mate_delivered_on_the_razored_horizon`.

This is materially different from the SEE material filter rejected in 8.2, which deleted the queen
sacrifice at *every* depth. It is still a real cost, and it is the reason the depth-1 restriction
is load-bearing: **if razoring is ever wanted at depth >= 2, the qsearch has to generate checking
moves at its first ply first.**

### 3.3 The gauntlet, and the result

**The decision run: round robin over `ab-razor` and `suprah-0.35.2` on host C**, `--elo0 -10
--elo1 0` (*does it hurt*), run under `scripts/run_sprt_match.sh` so the sequential stopping rule
ended it the moment the pairing decided rather than at the planned ceiling of 2600 rounds
(5200 games). It stopped at **517 pairs (1034 games)**: LLR **+2.991** against the H1 bound of
**+2.944**, **H1 accepted** — razoring is not the harmful side of the *does it hurt* question.

| | |
| :--- | :--- |
| Pair outcomes | `0-2:34  0.5-1.5:115  1-1:186  1.5-0.5:139  2-0:43` |
| Score | 52.03% |
| Paired Elo | **+14.1**, 95% CI **[-1, +30]** (517 pairs) |
| Unpaired Elo | -14.1 for the reference build, 95% CI [-30, +2] — same magnitude, opposite sign, from `scripts/pairing_elo.py` |

**Read the interval correctly.** This was a *does it hurt* run, not a *does it gain* one: the SPRT
decision (H1 accepted) is the thing that governs the default, and it is robust — the CI crossing
zero does not undo it, because the test was never trying to establish that the point estimate
clears zero, only that a -10 Elo regression is not what the data show. Section 2's SEE pruning
result is the precedent for this reading; unlike that one, this run's own LLR crossed a bound
rather than being left to settle from hand comparison.

**The default is set**: `enable_razoring` in `src/config.rs` and the UCI literal in
`src/threads.rs` both now default `true` — the fourth release in a row that had to fix one of
those two by hand (1.1). `razoring_margin` is not yet SPSA-tuned; tuning it is the next open item
for this rule specifically, now that the default it shapes is shipping.

**Infrastructure note for host C.** The Matt-Magie manager deployed in `<mm>` (source in
`<mm>/src`, not tracked by this repository's git history) did not support opening lines at all —
`mm.sh` silently ignored the `openings` key and every game started from `startpos`, so the only
variance between paired games came from search timing jitter. This was patched directly in
`<mm>/src/main.rs` (a 14th CLI argument carrying a space-separated UCI move list, replayed onto
the game and PGN before the loop starts) and `<mm>/mm.sh` (loads `openings_lines` from the file
named by the `openings` key and passes one per round, cycling). A pre-existing, unrelated build
break in `<mm>/src/zobrist.rs` (`RngExt` does not exist in `rand` 0.9; the trait is `Rng`) had to
be fixed first to get any build at all. None of this travels with the repository — if `<mm>`'s
manager is redeployed or replaced on a future host, the same gap will need patching again.

The 100-game health check from before the gauntlet, **on host A**, is `<mm>/razor_health.trn` /
`razor_health.pgn`: `+17 =54 -29` for the razoring build, **+41.9 Elo, paired 95% CI [+1, +84]**,
zero losses on time, zero duplicate games, design effect 1.00. It correctly signalled "probably
not harmful" ahead of the real decision; it was never a substitute for the round robin above.

## 4. Singular Extensions (SE) — ✅ shipped enabled in v0.37.0 (about +5 to +10 Elo)

**What ships.** At non-root nodes with `depth >= singular_min_depth`, when the TT entry is deep
enough (`depth - singular_tt_depth_margin`), bounded in the direction that supports the move, and
the TT move is the move about to be searched: a verification search at `(depth - 1) / 2 -
singular_depth_reduction` around `tt_eval ∓ singular_margin * depth`, **excluding the TT move**.
If nothing else reaches the threshold, the TT move gets +1 ply. Exclusion is per node and never
inherited; an exclusion search is never stored in the TT and never answered from it.

Defaults: `enable_singular_extensions = true`, `singular_min_depth = 6`,
`singular_tt_depth_margin = 3`, `singular_margin = 2`, `singular_depth_reduction = 0`. None SPSA-tuned.

### 4.1 The trigger depth is 6, not the published 8

10.5 asked for this before the feature was built. A fixed-depth census at depth 9 over 24 lines of
`openings_mixed.txt`, behind the `search-diag` feature, priced every candidate in one run:

| `singular_min_depth` | verifications | extensions | verification nodes | Δ tree |
| ---: | ---: | ---: | ---: | ---: |
| 8 (published) | 349 | 47 | 0.5% | +7.4% |
| 7 | 1137 | 210 | 1.3% | +17.3% |
| **6** | **2960** | **634** | **2.2%** | **+18.0%** |
| 5 | 7112 | 1672 | 4.4% | +28.8% |
| 4 | 17744 | 4512 | 5.8% | +36.7% |

**Depth 6 dominates depth 7** — three times the extensions for the same tree cost — and 8 barely
fires at a root depth of 9 to 10. Second finding, and the one that shapes future tuning: the
verification search is only **2.2%** of the tree; the other 15.8% is the extra plies. Cheapening
verification (`singular_depth_reduction`) cannot buy much. What matters is *which* moves extend.

### 4.2 What it measured

| run | games | score | Elo |
| :--- | ---: | ---: | ---: |
| pricing round robin, stopped by the SPRT | 546 | 54.40% | +30.6 |
| pre-release cross-version smoke test | 100 | 45.50% | -31.4 |
| confirmation round robin, stopped by the SPRT | 1945 | 50.95% | +6.6 |
| **pooled** | **2591** | **51.47%** | **+10.2**, 95% CI [-1.0, +21.5] |

Both round robins accepted H1 on *does it hurt* (`--elo0 -10 --elo1 0`), independently, at
1000ms + 100ms on `openings_mixed.txt`. That is the gate and it is solid: **SE does not cost
strength.** The size is single-digit — read it as **+5 to +10 Elo**, not the +30.6 the first run
reported. A 35-position LCT II run could not distinguish the rule from off (17 → 18 of 35), which
is the correct outcome for a change this size and is not evidence in either direction.

### 4.3 A stopped SPRT is not an effect size — applies to every measurement in this document

An SPRT stops the moment evidence crosses a bound, which means it stops *when the sample happens
to be favourable*. The decision is valid; the score at the stopping moment is conditioned on
having crossed and is therefore biased away from zero. Here that was a factor of four, and only
the mandatory smoke test caught it.

* **Razoring, 3.3**, stopped at 1034 games reporting +14.1 Elo, carries the same bias. Read it as
  an upper estimate. It also explains why its CI crossed zero while its LLR crossed the bound.
* **The rule going forward**: `scripts/run_sprt_match.sh` decides *whether to ship*. It does not
  measure *how much*. When the size matters, run a fixed game count decided in advance. The
  stopping rule and the effect size cannot come from the same run.

### 4.4 Open

* `singular_margin` (2), `singular_tt_depth_margin` (3), `singular_depth_reduction` (0) are
  untuned. 4.1 argues the reduction is the least promising of the three.
* Priced at one time control only. The trigger is depth-conditional, so 10.5 applies: at a longer
  control depth 6 sits deeper in the tree and the number need not transfer.
* Single-digit Elo is a thin margin to carry a default on. It passed the gate it was asked to pass.

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
  A later measurement (8.5) sizes the same class from the other side: **5.9% of all searched
  moves give check, 3.4% of them quiet.** That bonus is also why LMR and LMP practically never
  fire on a checking move, so moving it is not the free reordering it looks like — it would hand
  those two rules a class of moves they have never actually pruned.
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

## 7. Negamax refactor — ✅ shipped in v0.37.1, node-identical, no Elo expected

`[Impact: Low]` `[Complexity: High]`

`src/search_service.rs` used an asymmetric `minimax` with parallel `if white { ... } else { ... }`
blocks across every pruning rule, PVS null-window and TT update. It is now canonical negamax on
side-to-move-relative scores. `minimax` lost its `white` parameter and 20 of its 22 colour
branches; `singular_verification` lost both of its own, including its reduced search and the bound
test that existed only to compensate for absolute scores.

The full transformation, the invariants it rests on and the sites it deliberately does not touch
are in [`task/negamax_refactor_plan.md`](task/negamax_refactor_plan.md).

| Gate | Result |
| :--- | :--- |
| `cargo test --release` and `cargo test` (debug, overflow checks on) | 176 passed, 0 failed |
| Compiler warnings, with and without `search-diag` | none |
| `scripts/verify_negamax_identity.py`, depth 8 and depth 10 | **28/28 identical** |
| Cross-version gauntlet, host C | **passed**, 2026-08-29. 1s + 100ms, 100 games per pairing, challenger first: **52.0%** against v0.37.0 (+37 =30 -33, +13.9 Elo [-37, +65] paired) and **50.5%** against v0.36.0 (+33 =35 -32, +3.5 Elo [-51, +59] paired). Zero losses on time, zero duplicate games, design effect 1.00 and 1.09 |
| Release | **v0.37.1**, patch, 2026-08-29 |

The corpus is the 14 shared positions **plus a colour-swapped copy of each**. The shared corpus is
white-to-move throughout, which is exactly the blind spot a colour-symmetry refactor has: a sign
error on the Black branch would have left all 14 of them identical.

**Re-running the gate.** Both binaries must be `--features search-diag` builds, or no `SEARCHTREE`
line is emitted and the check refuses the run rather than reporting a false pass:

```bash
git stash                      # or check out the pre-refactor commit into a worktree
cargo build --release --features search-diag && cp target/release/suprah /tmp/suprah-baseline
git stash pop
cargo build --release --features search-diag && cp target/release/suprah /tmp/suprah-candidate
scripts/verify_negamax_identity.py --baseline /tmp/suprah-baseline \
                                   --candidate /tmp/suprah-candidate --depth 8
```

It drives the engine through `scripts/uci_driver.py`, so it waits for `bestmove` rather than
sleeping; 28 positions at depth 8 take roughly eight minutes on an 8-core ARM host.

Both gauntlet intervals span zero by a wide margin, which is the only reading a node-identical
change admits. **Neither number is a measurement** and neither may be quoted as one: at 100 games
a pairing resolves to roughly +/-50 Elo, and the run existed to catch a broken build, nothing
finer. The +13.9 against v0.37.0 in particular is noise about a binary that searches the identical
tree — see 4.3.

* **Tasks**:
    - `[x]` Run the mandatory cross-version gauntlet from `skills/engine_release_procedure.md`.
      Passed on host C, 2026-08-29; the numbers are in the table above.
    - `[x]` Release as a **patch**. Shipped as **v0.37.1** on 2026-08-29 with a refactor CHANGELOG
      entry that records both open defects as known limitations.

### 7.1 One site could not be merged, and it is a defect — open

The first identity run failed 19 of 28 positions with matching scores and principal variations and
a moved tree. A counter on every early return isolated it to the Transposition Table store, and a
per-store dump named the case: **`alpha == beta` is reachable.** The root narrows
`current_alpha`/`current_beta` towards each other as it searches its move list and can hand the
next root move an empty window. At such a node both bound tests are true at once, so the order the
two comparisons are written in picks the label. On the absolute scale that broke the tie towards
`UpperBound` at a White node and `LowerBound` at a Black one; on the relative scale it breaks the
same way for both.

`Self::bound_for` reproduces the old, colour-dependent order, and restoring it made all 28
positions identical — which also proves it was the *sole* source of divergence.

**The Black half of that tie-break stores a bound the search never proved.** A Black node whose
running score is still at `beta` has established that Black cannot get *below* `beta` — a lower
bound. Labelling it `UpperBound` publishes "at most `beta`" into a table that outlives the move.
That is the shape of the defect 8.1 records at roughly two hundred Elo, and the White half of the
same tie-break is correct, so the engine stores a sound bound for one colour and an unsound one
for the other.

**How often it fires**, counted over 20 searches at depth 9 (the corpus plus six mirrors),
295,163 Transposition Table stores in total:

| | Stores | Share |
| :--- | ---: | ---: |
| On an empty window | 1,376 | 0.47% |
| ...of those, at a Black node, i.e. an unproven bound | 727 | 0.25% |
| ...of those, at depth >= 4, where the probe is most likely to trust them | 270 | 0.09% |

It is concentrated rather than spread: three positions (Kiwipete, Closed Centre, King Attack)
produce none at all, while Rook Endgame reaches 2.75% and Middlegame 2.07%. **Rarity is not an
argument for ignoring it** — 8.1 cost 168 Elo on a defect that moved only 2 of 60 positions by
more than 50cp.

**Where the empty window comes from.** No interior node can create one: `alpha` only rises, the
move loop breaks the moment `alpha >= beta`, the Transposition Table probe returns on the same
condition, and Mate Distance Pruning returns when its clamp closes the window. The one remaining
source is the root, which narrows `current_alpha`/`current_beta` towards each other as it searches
its move list — and that can only close completely when `beta` is finite, i.e. **only under an
aspiration window that is failing high**. The root does not break out of its move loop on a fail
high; it finishes the iteration with an empty window and re-searches afterwards. The entries
written during that abandoned iteration stay in the table.

**Where the code is.**

| What | Where |
| :--- | :--- |
| The tie-break itself | `SearchService::bound_for` in `src/search_service.rs` — both orders are written out side by side, so the correction is deleting the `if white` and keeping one |
| Its two call sites | the Quiescence Search Transposition Table write and the main-search one, in `minimax` |
| The behaviour is pinned by a test | `test_transposition_bound_tie_break_stays_colour_dependent_on_an_empty_window` — it will fail the moment the order is changed, which is the point |
| Where the empty window is produced | `get_moves`: `current_alpha = current_alpha.max(...)` / `current_beta = current_beta.min(...)` inside the root move loop, and the fail-high branch of the aspiration re-search below it |

**Reproducing the frequency table** takes a temporary counter, not a permanent one: call a static
`AtomicU64` bump beside `Self::bound_for` in the main-search store, keyed on
`orig_alpha == orig_beta` and `!white`, and print it where `crate::search_diag::dump()` is called
at the end of `get_moves`. The numbers above came from exactly that, over `go depth 9`.

**It is not the source of the colour asymmetry.** Measured 2026-08-29: a build with the `if
white` deleted and one order kept searches mirrored positions at a mean node ratio of 1.79 against
the baseline's 1.85, and the score gaps are unchanged. 7.1 was the last search-side suspect 7.2
had, and it is eliminated — the asymmetry is a move-ordering artefact (7.2). This defect therefore
has to stand on its own argument, which is the unproven bound, not the asymmetry.

* **Tasks**:
    - `[ ]` Run the 8.1 gate first, not a match. This defect is "an unproven bound in a table that
      outlives the move", which is 8.1's mechanism exactly, so the cold-versus-warm drift
      measurement is the cheap deterministic test and it costs no games. 8.1 records the shape:
      search a fixed 60-move sequence twice with one build, once clearing the table per position
      and once letting it accumulate, and count positions drifting more than 50cp. **Run it
      against 10.8 in the same pass** — there is a second, independent source of cold-versus-warm
      drift in the evaluation, and the gate cannot tell them apart on its own.
    - `[ ]` Then price the correction. `Self::bound_for` holds both orders side by side and the fix
      is one edit; it moves the tree, so rule 1 applies.
    - `[ ]` Decide whether the root should break out of its move loop on an aspiration fail high.
      If it should, the empty window stops existing and the tie-break becomes unreachable — which
      may be the better fix, and is a tree change in its own right.

### 7.2 The engine is not colour-symmetric — ✅ cause established 2026-08-29, one half fixed in v0.37.2

`scripts/verify_negamax_identity.py` searches every corpus position twice, once as published and
once colour-swapped. A colour-swapped position is strategically identical to its original, so a
symmetric engine would search a near-identical tree and return the same score. Reproduced on the
v0.37.1 baseline at depth 8 — the node counts match the v0.37.0 reading exactly, so the refactor
did not move them:

| Position | As published | Colour-swapped | Ratio | Score | Mirror |
| :--- | ---: | ---: | ---: | ---: | ---: |
| Pawn Endgame | 383 | 1,300 | 3.39x | +19 | +19 |
| Sharp Tactical | 12,941 | 35,542 | 2.75x | -98 | -96 |
| Sharp French | 45,742 | 124,245 | 2.72x | -23 | -39 |
| Kiwipete | 121,779 | 57,158 | 2.13x | -44 | -2 |
| Middlegame | 23,561 | 33,173 | 1.41x | +73 | +33 |

Mean ratio 1.85 over 13 positions, 7 of them above 1.5x. UCI scores are relative to the side to
move, so a symmetric engine returns the *same* number for both, not its negation.

**The cause is move ordering, and it is not a defect that can be repaired by a margin or a
constant.** Four measurements, in the order they eliminate things:

1. **Evaluation is not the cause.** `EvalService::calc_eval` was mirror-checked over 4,260
   positions replayed from the shipped opening books, at five windows each so that Lazy Evaluation
   is live. One genuine asymmetry was found and is fixed in v0.37.2 (below); with it, 0 of 21,300
   (position, window) pairs disagree.
2. **The Transposition Table tie-break is not the cause.** Making `bound_for` colour-blind — the
   7.1 correction — leaves the mean node ratio at 1.79 and the score gaps unchanged. It remains a
   defect worth pricing, but it does not produce this.
3. **SEE is not the cause.** Mirror-checked over 479 captures on the same corpus: none disagree.
4. **The move list is.** For every corpus position and its mirror the generated move *set* is the
   exact mirror image and every move's *rank* is identical — move ordering is perfectly
   colour-symmetric in what it scores. The *sequence* is not. `generate_moves_list_for_piece`
   walks piece and target bitboards by ascending square index, which a colour mirror reverses,
   and the root's selection sort keeps the earlier list position on a rank tie (`>`, not `>=`).
   Ties are everywhere: whole quiet-move blocks share rank 0 and captures share an MVV bucket.

**Why a reordered list changes the score and not just the node count.** LMP, LMR and Futility
Pruning all key on the move's *index* in the list, so a position and its mirror prune and reduce
different moves. Turning off every rule that is inexact or index-keyed — LMP, LMR, futility, RFP,
NMP, razoring, singular extensions, bad-capture pruning, Lazy Evaluation — collapses the score
gaps and leaves the trees apart, which is the signature of an ordering effect rather than an
evaluation error:

| Depth 7, 13 positions | Score gaps | Worst | Node ratio (mean / worst) |
| :--- | ---: | ---: | ---: |
| Default rules | 12 of 13 nonzero, 185 cp | 55 cp | 2.34 / 9.98 |
| Order-dependent rules off | 2 of 13 nonzero, 10 cp | 7 cp | 1.76 / 4.89 |
| Order-dependent rules off, **plus the v0.37.2 pawn fix** | **1 of 13, 7 cp** | 7 cp | **1.28 / 1.87** |

Alpha-beta returns the same value under any move order, so the residual 7 cp on one position is
what is left of the inexact rules that have no off switch — quiescence pruning and the
Transposition Table.

#### The evaluation half, fixed in v0.37.2

`white_pawn_structure_score` and `black_pawn_structure_score` counted enemy pawns on the adjacent
files with an **index** threshold where a **rank** predicate was meant. On an adjacent-files mask
an index threshold also admits the pawn's own rank — `sq + 1` for White, `sq - 1` for Black — and
a colour mirror flips the rank while leaving the file alone, so the two masks were not mirror
images.

On `r1bqkb1r/1ppp1ppp/p1n2n2/1B6/3pP3/5N2/PPP2PPP/RNBQ1RK1 w kq - 0 6` the black d4 pawn counts
one enemy on the adjacent files and the mirrored white d5 pawn counts two, because e5 sits on the
candidate's own rank and was counted for one colour only. `friendly >= enemy` then holds for one
side and not the other, and at `candidate_passed_pawn_bonus = 8` with advancement 2 that is
exactly the 8 cp middlegame / 16 cp endgame the sweep measured. It moved 13 of 381 positions.

Pinned by `test_candidate_passed_pawn_mask_is_colour_symmetric` and
`test_candidate_passed_pawn_ignores_same_rank_enemy_pawns`.

#### The ordering half — open, and probably not worth fixing

Making the tree symmetric means making move generation order mirror-invariant: iterate the piece
and target bitboards from the far rank for the side to move rather than from square zero. That is
a tree change across every node in the search, it would need pricing like any other, and **there
is no argument that symmetry itself is worth Elo** — a tie-break has to go one way or the other,
and nothing says the mirrored order is better than the published one. What the measurement does
establish is that the 2-3x figure is a tie-break artefact and not evidence of a defect, so it
should stop being carried in this document as an open bug.

* **Tasks**:
    - `[x]` Establish where the asymmetry lives. Done: move-list order, with a genuine but small
      evaluation contribution now fixed.
    - `[x]` Fix the evaluation half. Shipped in v0.37.2.
    - `[ ]` Decide whether mirror-invariant move generation is worth measuring at all. It is a
      tree change with no prior reason to gain, and this document's history (8.1, 8.2) argues for
      not building it on aesthetics.

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

Binaries and PGNs are kept outside the repository, under `<mm>/engines/` and `<mm>/`:
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

**The follow-up question this raised has since been answered, and the answer is no.** The guards
that make an extension expensive also exempt every checking move from LMR, futility, LMP and SEE
pruning, whether or not the extension runs; the obvious next thought was that the exemption, not
the extension, was the cost. It was measured on 2026-08-28 and is worth 4.5% of the tree at the
absolute most, 0.34% for the LMR half that the proposal was actually about. See 8.5.

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

### 8.5 Damping the check exemption — ⛔ measured 2026-08-28, dead before a game was played

This was section 11, a proposal rather than a backlog item, and it was written so that step 1
could kill it for the price of one diagnostic build. It did.

**The observation was correct.** A move that gives check is exempt from all four of Late Move
Reductions, Late Move Pruning, Futility Pruning and the SEE pruning of bad captures, at once:

| Rule | Guard | `src/search_service.rs` |
| :--- | :--- | ---: |
| SEE pruning of bad captures | `!turns.moves[i].gives_check` | ~1007 |
| Late Move Pruning | `!current_turn.gives_check` | ~1037 |
| Futility Pruning | `!current_turn.gives_check` | ~1053 |
| Late Move Reductions | `!current_turn.gives_check` | ~1155 |

**The inference from it was wrong.** Measured with `scripts/measure_check_exemption.py` over the
same 14-position corpus at fixed depth 10, 2,201,819 searched moves, on a **node-identical**
instrumented build:

| | share of searched moves |
| :--- | ---: |
| moves that give check | 5.9% |
| ... of which quiet | 3.4% |
| moves searched while in check (a clean partition) | 4.3% |
| tree below a checking move — **nests, so an upper bound** | 46.9% |

That last row is the number that makes the item look large, and it is the misleading one:
subtrees rooted at checking moves contain each other, so the sum counts nodes once per checking
ancestor. The quantity that decides is what each rule would actually have removed with its guard
dropped:

| Rule | moves | share of tree below them |
| :--- | ---: | ---: |
| Late Move Reductions | 310 | **0.34%** |
| Late Move Pruning | 271 | **0.02%** |
| Futility Pruning | 13,055 | 1.78% |
| SEE pruning of bad captures | 17,775 | 2.35% |
| all four | 31,411 | **4.49%** |

**The proposal was specifically `lmr_check_damping`, and LMR is the worst of the four: 310 moves
out of 130,253, 0.01% of searched moves.** The reason is the move ordering the engine already has.
A checking move carries `give_check_rank_bonus * 10000` = 50,000 and therefore sorts ahead of
every quiet move, so `turn_counter > lmr_move_threshold` is almost never true for one. The guard
is close to vacuous: removing it would change almost nothing, because the ordering already does
what the guard claims to do. Section 1 says the same thing about LMP's guard from the other
direction — it calls the guard "a property of the rule rather than of the current move ordering",
and this measurement is what that costs: 0.02%.

The two rules that *would* have removed something, Futility and SEE pruning, delete moves rather
than reduce them, and both guards exist for the documented reason in 8.2 — an SEE gate on checking
moves deletes the queen sacrifice in Philidor's Legacy. Trading a known tactical risk for at most
4.1% of the tree, which is an upper bound inflated by nesting, is not a trade worth a gauntlet.

**What to keep from it.** The instrumentation stays: `scripts/measure_check_exemption.py` and the
`SEARCHDIAGCHECK` counters in `src/search_diag.rs` are cheap, node-identical and reusable. And the
lesson generalises past this item — *the tree below a class of moves is not the prize; the prize
is what a rule would actually have removed from it, and the two differed here by a factor of ten.*


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
* The branch is level with master at **`v0.36.0-NNUE`**, ported from master v0.36.0: the razoring
  block (verbatim, same guards), the seven razoring tests, and — new for this port —
  `enablerazoring` / `razoringmargin` wiring in `src/game_handler.rs`'s `setoption` handler, which
  no earlier port had needed to touch. The decision was not re-measured on this branch; it carries
  master's SPRT result (3.3) because the razoring guards and the search parameter default are
  identical here, and only the branch's own SPSA-tuned values differ. A mandatory 100-game smoke
  gauntlet against `suprah-0.35.1-nnue` and `suprah-0.35.2-nnue` scored 50.0% and 53.5%, clear of
  the 45% floor, with no forfeits and no duplicate games — a health check on the port, not a
  pricing run. Local branch history had also silently fallen behind `origin/feature/nnue-evaluation`
  by six releases (stuck at `v0.30.0-NNUE` locally while `v0.35.2-NNUE` was already tagged on the
  remote); resolved with `git branch -f feature/nnue-evaluation origin/feature/nnue-evaluation`
  before this port started, and worth checking for on any host where the branch has not been
  touched in a while.
* **Open: no gauntlet against `suprah-0.30.0-nnue` specifically has been run.** The branch now
  plays real games again (the smoke gauntlet above), which the entries below this one could not
  say, but the three defaults that were measured on the HCE evaluation and merely assumed to
  transfer remain unconfirmed on this branch: `enable_check_extension = false` (-23.7 Elo on HCE),
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
| `scripts/uci_driver.py` | Drives a binary over UCI and **waits for `bestmove`**. See the warning below. |
| `scripts/measure_check_exemption.py` | How much of the tree does a class of moves govern, and what would a rule have removed from it? Added for 8.5. |
| `scripts/measure_razoring.py` | Searched nodes and search time for a rule toggled over UCI, on the fixed-depth corpus. Added for section 3. |

> [!WARNING]
> **A fixed `sleep` before `quit` is not a way to run a fixed-depth search.** The older scripts
> start a search, sleep, then send `quit`; a search that has not finished by then is killed
> mid-iteration and the numbers left on stderr are whichever iteration happened to complete. The
> totals then move with machine load rather than with the change under test.
> `scripts/uci_driver.py` reads stdout until `bestmove` instead, so the comparison is always the
> same depth on both sides. `measure_check_exemption.py` and `measure_razoring.py` use it;
> `measure_stage0.py` still does not.
>
> A second trap in the same place: **`search_diag::dump` runs once per iteration of the iterative
> deepening and its counters are never reset, while `Stats` is fresh on each of those calls.** The
> last `SEARCHDIAG*` line is therefore the cumulative total for the whole `go depth N`, and the
> last `SEARCHTREE` line is only its final iteration. Summing `SEARCHTREE` gives the comparable
> quantity. The two differ by more than a factor of two on the start position.

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

**Losses on time: zero** in the first 700 games at `concurrency = 9` on host B, and
zero again in all 9000 games at `concurrency = 14` on host A. Matt-Magie writes
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
work moves hosts — and
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
  It cannot be made to fire above depth 4 at all — see 10.6.
* **Razoring at depth 1** fires everywhere. Measured: -15% search time at depth 11 (section 3.1).
* **ProbCut at depth ≥ 5** reaches only the top few plies. Its trigger depth is a tunable and has
  to be set against this harness, not against the literature.
* **Singular Extensions at depth ≥ 8** fire at plies 0 to 2 and nowhere else. ✅ **Resolved
  2026-08-28**: the trigger depth came down. A fixed-depth census priced every candidate before a
  game was played and settled on 6, where the rule grants three times the extensions of 7 for the
  same tree cost (4.1). It is therefore a different feature from the published one, deliberately,
  and it measured **about +5 to +10 Elo** over 2591 games (4.2). This is the template for the rest
  of this list — the census cost one run and replaced an argument about the literature with a
  number about this harness. Read **4.3** before quoting any stopped-SPRT number in this document.

### 10.6 `lmp_max_depth` is inert above 4, and the first diagnosis was wrong

The symptom is unchanged and still matters: **`tuning/parameters.json` registers `lmp_max_depth`
with `max: 8` and the UCI facade in `src/threads.rs` advertises `max 10`, so SPSA and any GUI can
wander over a flat region from 4 upwards and tune nothing.**

The cause originally recorded here was the `searched_quiet_moves` array bound, which stopped
`quiet_count` at 64. That was real and **is fixed**: the two counters were separated in v0.35.x
and `quiet_count` is now unbounded. It was not the binding constraint.

The binding constraint is the threshold itself. `lmp_base_moves + 2 * depth^2` demands **53 quiet
moves searched at a single depth-5 node and 75 at a depth-6 node**, and no node produces that
many: a full move list is rarely over 50 moves, and beta cutoffs end most nodes long before it is
exhausted. Measured at fixed depth 8 with the transposition table on, over Kiwipete, a middlegame
and a closed centre, at `lmp_base_moves` 0 and 3: `lmp_max_depth` 4, 5, 6 and 8 all search the
**bit-identical tree**.

`test_lmp_max_depth_is_inert_above_four` pins the flat region, so a change to the growth term
fails there loudly instead of silently widening what the tuner explores.

* `[x]` Count quiet moves in a separate counter that is not bounded by the array length.
* `[x]` Add a test covering `lmp_max_depth` above 4.
* `[x]` Lower the advertised bound to 4 in both places. Done in v0.37.2: `src/threads.rs` now
  advertises `max 4` and `tuning/parameters.json` registers `max: 4`, pinned by
  `test_lmp_max_depth_advertises_only_its_live_range`.
* `[ ]` If depths above 4 are wanted, change the **growth term** — it is the quadratic that is
  dead, not the counter and not the advertised bound.

### 10.6a Dead configuration: `search_threads` — ✅ removed in v0.37.2

`Config::search_threads` defaulted to 2 and was **written in four places and read in none**. The
engine is single-threaded in search, and `src/threads.rs` says so at runtime: a `setoption name
Threads` is answered with *"Single-threaded engine. Ignoring setoption threads"*.

One thing was confirmed by it before it went: `engine_options = ... Threads=1` in every `.trn`
file does nothing, and the 25% core-headroom calculation in "How to run a measurement" is right to
assume one thread per engine.

### 10.7 Not done, and why

* **Adopting the wider pool.** `openings/book_codekiddy_10ply.txt` is built and 49 times broader
  at four plies, but nothing has been played on it. Two things are unmeasured: whether it costs
  or saves games per decision (10.4), and whether its lines are balanced enough — a line whose
  final position already favours one side is not wrong under colour-swapped pairing, but nobody
  has looked. Balance filtering needs the engine to evaluate every candidate and was not done.
* **Comparing the two pools.** One pairing, run twice, `sprt.py --plan` on each. That settles the
  cost question in one measurement and should precede adopting the new pool.

### 10.8 Lazy Evaluation compares a `cheap_eval` that is missing the pawn structure — open

`EvalService::cheap_eval` reads the pawn hash table but **never fills it**. On a miss
`struct_mg`/`struct_eg` stay at zero, so the Lazy Evaluation cutoff in `calc_eval` compares a
score with the entire pawn-structure term absent against `alpha`/`beta`, and
`lazy_eval_margin_search = 180` does not cover it. The full evaluation below computes the term and
stores it, so the *second* visit to the same pawn structure compares a different number.

Measured over the corpus at five windows, the same position evaluated with a cold and a warm pawn
table:

| Position | Window | Cold | Warm | Drift |
| :--- | :--- | ---: | ---: | ---: |
| Sharp French | (-300, -299) | 44 | 103 | 59 |
| King Attack | (200, 400) | 20 | 62 | 42 |
| Closed Centre | (-300, -299) | 64 | 102 | 38 |
| Open Sicilian | (-300, -299) | 48 | 85 | 37 |
| Kiwipete | (-300, -299) | 84 | 104 | 20 |

Six of seventy (position, window) pairs drift. The bound each call returns is still sound — the
claim "at or beyond this bound" holds either way — but the *value* is not a function of the
position alone, and `static_eval` is that value: it drives Reverse Futility Pruning
(`search_service.rs`, rule 0.5), razoring (0.6) and Futility Pruning in the move loop. All three
therefore prune on the fill state of a table rather than on the position.

This is 8.1's mechanism — a value that outlives the move — from a second direction, which is why
7.1's cold-versus-warm gate has to be run against both or it will credit 7.1 with this drift.

The likely repair is one branch: on a pawn-table miss, skip the Lazy Evaluation cutoff and fall
through to the full evaluation, which stores the term. Lazy Evaluation is only sound when the
cheap score is complete. It is a tree change, so rule 1 applies.

* **Tasks**:
    - `[ ]` Run the 8.1 cold-versus-warm gate over the 60-move sequence, against this and 7.1
      together, and attribute the drift.
    - `[ ]` Price the repair.

