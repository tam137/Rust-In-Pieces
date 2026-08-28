# Suprah Engine Strength Enhancement Roadmap (`task.md`)

What to build next in **Suprah**, and the record of what has already been tried and failed.
Read "Negative results" before proposing anything: five of the ideas in this document were built,
measured and reversed, and two of them looked excellent on every metric except games won.

---

## 🧭 Start Here

### Where the engine is

| | |
| :--- | :--- |
| Released | **v0.35.2** on `master` (HCE), **v0.35.2-NNUE** on `feature/nnue-evaluation` |
| Throughput | **1.80x** over v0.30.3, from two measured changes on bit-identical search trees |
| Matchplay resolution | **+/-23 Elo at 500 games**, **+/-13 at 3000**, per pairing — measured on host A |
| Uncommitted | nothing. Razoring (section 3) is committed and **off by default** |
| Blocked on | one gauntlet, for the razoring default. Nothing else is waiting on a measurement. |
| Next session runs on | **host C (ARM)** — resolve `<mm>` and rebuild the binaries there; nothing from host A runs or transfers |

The engine searches at roughly 6.5 M nodes/s **on host A**, and reaches **depth 9 to 10** at the
1s + 0.1s match time control there. Both figures are host-dependent and unmeasured on host C. Sections 1 and 2 are built, measured and released. Section 3 is built and
measured on a corpus but not in games; sections 4 to 7 are not started. Section 10 is the
measurement infrastructure.

> [!IMPORTANT]
> **Elo numbers do not carry across hosts**, and the work has moved three times. Runs are labelled
> by host throughout this file:
>
> | Host | What | When |
> | :--- | :--- | :--- |
> | **host A** | 20-core x86-64 | up to 2026-08-27, and again from 2026-08-28 |
> | **host B** | 12-core x86-64 | the deciding LMP/SEE gauntlet, 2026-08-27 evening |
> | **host C** | ARM | from 2026-08-29, the razoring gauntlet onward |
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

Sections 1 and 2 carry the numbers and the reasoning. Two lessons from that round were general
enough to become rules below rather than history: **a run must pair the configuration it exists to
qualify** (rule 2), and **no intermediate value of a difference-of-differences is a result,
however stable it looks** — `both - lmp` read +12.0 [+2, +22] over three stable checkpoints at 78%
of a 9000-game run and finished at +3.9.

### The next action — run the razoring gauntlet on host C

**This is the one thing blocking the next release.** Section 3 is written, guarded, unit-tested
and measured on a fixed-depth corpus: **-7.7% to -20.1% searched nodes and about -15% search
time** at depth 11, depending on the margin. It ships **off**, because a shipped default is a
search parameter default and rule 2 binds that to a cross-version gauntlet. Only the default is
open — the code is free to keep either way.

> [!IMPORTANT]
> **The gauntlet runs on host C (ARM), in a fresh session, and nothing measured on host A carries
> over to it.** Resolve `<mm>` first (see below); it is not where it was on host A. Re-measure the
> two per-host numbers before trusting them — the concurrency ceiling and games/minute in "How to
> run a measurement" are host A's. Existing PGNs do not travel between hosts and must not be read
> together with the new one.

Step by step:

1. **Resolve `<mm>`** and confirm `mm.sh`, `engines/` and `openings_mixed.txt` are there. Copy
   `openings/book_mixed.txt` to `<mm>/openings_mixed.txt` if it is missing.
2. **Rebuild every binary the run needs, natively on host C.** Everything in `<mm>/engines/` was
   built on an x86-64 host and will not run. That is `ab-razor` (suffix `Cargo.toml` to
   `0.35.2-RAZOR` and set `enable_razoring = true` in `src/config.rs`), and the reference
   `suprah-0.35.2` from the released tag. **Restore `Cargo.toml` and `src/config.rs` afterwards.**
   The recipe is in "How to run a measurement"; do not use `./build_and_release.sh` for a
   throwaway variant, it rewrites `CHANGELOG.md` and `Cargo.toml`.
3. **Verify the binaries actually differ** before spending hours on them. `scripts/uci_driver.py`
   does this in seconds and does not need the harness:
   ```bash
   python3 -c "import sys; sys.path.insert(0,'scripts'); from uci_driver import search; \
     [print(e, search('<mm>/engines/'+e, \
       'r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1', 11).time_ms) \
      for e in ['suprah-0.35.2','ab-razor']]"
   ```
   On host A the equivalent pair read 337 ms against 286 ms. The absolute numbers will differ on
   ARM; what must hold is that `ab-razor` is the faster of the two. If they are equal, the default
   did not take effect and the run would have measured nothing.
4. **Calibrate concurrency on host C.** `floor(nproc * 0.75) - 1`, then 120 games at that setting
   and `scripts/match_health.py` on the result. A non-zero forfeit count means step down, not
   correct.
5. **Plan the run before starting it.** `scripts/sprt.py --plan --elo0 -10 --elo1 0` — the *does it
   hurt* framing, which is the right one when the code exists and only the default is open. On
   host A's pair variance that costs about 800 games at a true +20 and 1100 at +13, against 1340
   and 2490 for *does it gain*, and about 3900 either way if the rule is truly neutral. Re-run
   `--plan` with host C's own variance rather than trusting those rows.
6. **Run it as a round robin over `ab-razor` and `suprah-0.35.2`**, not a gauntlet — rule 2: in
   gauntlet mode only the challenger plays everyone, and the pairing that decides has to exist.
   `suprah-0.35.2` is the released reference, so this is a cross-version check and not a self-A/B;
   `ab-noraz` is the same code as the reference and is only worth adding as a third engine if a
   same-build control is wanted.
7. **Read it per pairing** with `scripts/pairing_elo.py`, never off the scoreboard, and check
   `scripts/match_health.py` before believing any of it.
8. **Then set the default** in `src/config.rs` **and** the UCI literal in `src/threads.rs`, and
   only then consider tuning `razoring_margin` by SPSA.

A 100-game health check has already been run on host A (`<mm>/razor_health.pgn`): clean, no
forfeits, no duplicate games, design effect 1.00, and razoring ahead at **+41.9 Elo, 95% CI
[+1, +84]**. At this harness's resolution 100 games are worth about +/-50 Elo, so that interval
barely clearing zero says the rule is probably not harmful and nothing more. **It is a sign, not
a result, and it does not substitute for step 5 to 7.**

### The backlog after that, in order

| # | Item | Where | Why this order |
| ---: | :--- | :--- | :--- |
| 1 | Razoring: the gauntlet, then the default | 3 | Code is done. One measurement away from shipping |
| 2 | Singular Extensions | 4 | Largest search item; needs TT-move exclusion. Read 10.5 first — at depth 9 to 10 it fires at plies 0 to 2 and nowhere else |
| 3 | `MovePicker` stages 1-3 | 5 | The throughput prize, but read 5.2 and 8.3 before starting |
| 4 | NNUE incremental accumulator | 6 | Only worth it once `use_nnue` is the default path |
| 5 | Negamax refactor | 7 | Pure refactor, no expected Elo, high blast radius. Last. |

The proposal that used to be section 11 — damping the check exemption — was measured on
2026-08-28 and is dead. It is written up as a negative result in 8.5.

### Everything still open, in one place

A new session can start from this table; each row says where the detail is.

| Open | Where | Kind |
| :--- | :--- | :--- |
| **Razoring gauntlet on host C, then set the default** | 3.3 | blocking the next release |
| Advertised UCI defaults drift from `Config::default()` — twelve stale literals | 1.1 | correctness of the facade only |
| `lmp_max_depth` is inert above 4; the tuner and UCI still advertise a dead range | 10.6 | dead tuning range |
| `Config::search_threads` is written in four places and read in none | 10.6a | dead code |
| Singular Extensions are close to untestable at depth 9 to 10 — decide trigger depth or time control **before** building | 4, 10.5 | open design question |
| `MovePicker` stages 1-3: needs an entry-time history snapshot, and five `#[allow(dead_code)]` attributes to resolve | 5, 5.2 | large item, constraints known |
| NNUE incremental accumulator, and making `use_nnue` the default | 6 | large item |
| The NNUE branch has not played a game since v0.30.0-nnue | 9 | unverified defaults |
| Whether the wider opening pool costs or saves games per decision, and whether its lines are balanced | 10.7 | open measurement |
| `scripts/measure_stage0.py` still uses a fixed `sleep` instead of `uci_driver.py` | 10.1 | unsafe measurement |
| Negamax refactor | 7 | last, no expected Elo |

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

## 3. Razoring at depth 1 — built, measured on a corpus, **off by default pending a gauntlet**

`[Impact: unmeasured in games]` `[Complexity: Medium]`

At depth 1, when the static evaluation trails the window by more than `razoring_margin`, one ply
is unlikely to recover the gap and the node's whole move loop is spent proving a fail-low. The
rule runs a Quiescence Search directly and returns that score **only if it confirms the fail-low**;
otherwise the node falls through to the normal search. The qsearch is a verification, not an
assumption.

**Built.** `enable_razoring` (**default `false`**) and `razoring_margin` (300) in `src/config.rs`;
the rule sits in `minimax` as step 0.6, after Reverse Futility Pruning and before move generation;
UCI options `EnableRazoring` / `RazoringMargin`; `razoring_margin` registered in
`tuning/parameters.json`. Six tests in `src/search_service.rs`.

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

### 3.3 What is still owed

* `[ ]` A cross-version gauntlet against `suprah-0.35.2`, framed as *does it hurt*
  (`--elo0 -10 --elo1 0`), run as a **round robin** so both configurations get a direct pairing.
  Build `ab-razor` and `ab-noraz` natively on the host that runs it, from v0.35.2 with the version
  suffixes `-RAZOR` / `-NORAZ` so the PGN separates them into two rows. The step-by-step is in
  "The next action".
* `[ ]` Set `enable_razoring` from that result, in `src/config.rs` **and** in the UCI literal in
  `src/threads.rs` — the third release in a row had to fix one of those by hand (1.1).
* `[ ]` Only then tune `razoring_margin` by SPSA. Tuning a parameter of a rule that is off is
  tuning nothing.

The 100-game health check already run **on host A** is `<mm>/razor_health.trn` /
`razor_health.pgn`: `+17 =54 -29` for the razoring build, **+41.9 Elo, paired 95% CI [+1, +84]**,
zero losses on time, zero duplicate games, design effect 1.00. At this harness's resolution 100
games are worth about +/-50 Elo, so the interval barely clearing zero is a sign that the rule is
not harmful, and nothing more. The `.trn` travels to another host; the PGN and the binaries do
not.

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
* **Singular Extensions at depth ≥ 8** fire at plies 0 to 2 and nowhere else. The feature as
  specified in section 4 is close to untestable here. Either its trigger depth comes down — which
  makes it a different feature from the published one — or the time control goes up, and one ply
  costs a factor of ten in wall time. **Decide which before building it.**

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
* `[ ]` If depths above 4 are wanted, change the **growth term** — it is the quadratic that is
  dead, not the counter and not the advertised bound. Until then, lowering the advertised `max`
  to 4 in both places would be honest, and is a one-line change in each.

### 10.6a Dead configuration: `search_threads`

`Config::search_threads` defaults to 2 and is **written in four places and read in none** —
`grep search_threads src/*.rs` finds only `src/config.rs`. The engine is single-threaded in
search, and `src/threads.rs` says so at runtime: a `setoption name Threads` is answered with
*"Single-threaded engine. Ignoring setoption threads"*.

Nothing is broken by it, and one thing is confirmed by it: `engine_options = ... Threads=1` in
every `.trn` file does nothing, and the 25% core-headroom calculation in "How to run a
measurement" is right to assume one thread per engine. Delete the field when `src/config.rs` is
next touched.

### 10.7 Not done, and why

* **Adopting the wider pool.** `openings/book_codekiddy_10ply.txt` is built and 49 times broader
  at four plies, but nothing has been played on it. Two things are unmeasured: whether it costs
  or saves games per decision (10.4), and whether its lines are balanced enough — a line whose
  final position already favours one side is not wrong under colour-swapped pairing, but nobody
  has looked. Balance filtering needs the engine to evaluate every candidate and was not done.
* **Comparing the two pools.** One pairing, run twice, `sprt.py --plan` on each. That settles the
  cost question in one measurement and should precede adopting the new pool.

