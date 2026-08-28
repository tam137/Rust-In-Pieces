# Matchplay Measurement Procedure

How to price a search change in games, at the lowest game count that can honestly answer the
question asked. Rule 1 of `task.md` makes matchplay the only currency; this document is about
making it affordable.

Read `task.md` "Rules that are not optional" first. Nothing here overrides them.

> [!IMPORTANT]
> **`<mm>` is the Matt-Magie working directory and is not the same on every host.** It is a
> sibling of this repository on some and elsewhere on others. Resolve it once at the start of a
> session — `find .. ~ -maxdepth 3 -name mm.sh -type f` — and substitute the result into every
> command below. Per `AGENTS.md` the resolved value is never written back into a committed file.
>
> **Engine binaries do not travel between hosts.** Anything already in `<mm>/engines/` was built
> for whatever architecture put it there; rebuild natively before using it. So do per-host figures
> like the concurrency ceiling and games/minute: re-measure, never carry over.

---

## 1. The question decides the cost, not the feature

Before any binary is built, write down the decision the run has to support. There are three
shapes, and they cost very different amounts.

| Shape | Hypotheses | Typical cost |
| :--- | :--- | :--- |
| **Does it gain?** A new feature, default off, that has to earn its place. | `--elo0 0 --elo1 10` | High. Rejecting a null feature is the expensive direction. |
| **Does it hurt?** A rule that is already written and free to keep; only a regression would stop it shipping. | `--elo0 -10 --elo1 0` | Roughly half of the above when the feature is genuinely good. |
| **Is it a regression?** A cross-version check against a released engine. | `--elo0 -10 --elo1 0` | Cheap when the answer is no. |

The middle row is the one that gets forgotten. `task.md` 8.2 measured the Check Extension as a
gainer and it was not; the LMP/SEE gauntlet asks whether the second of two written rules is worth
keeping, which is a *does it hurt* question and costs less stated that way.

**Price the run before starting it.** From any earlier PGN of the same engine family:

```bash
python3 scripts/sprt.py <any-earlier.pgn> --engines <A> <B> --plan
```

This reads the distribution of paired outcomes the harness actually produces - not an assumed
draw rate - and prints, for a range of true effects, how many games a sequential test needs and
what a fixed run of the same length would resolve. If the effect you are hunting is smaller than
the half-width in the last column, the run cannot answer the question and must be resized or
restated before it is worth starting.

Suprah is a **sharp** engine: only about a third of its pairs come back level, and the pair
variance is near 0.065. That is what makes small effects expensive here, and it is a property of
the engine and the opening pool rather than of the harness.

---

## 2. Build the variants

Matt-Magie sends one `engine_options` string to both engines, so an A/B of two configurations
needs two **binaries**, not two option sets.

```bash
# per variant
sed -i 's/^version = "X.Y.Z"$/version = "X.Y.Z-TAG"/' Cargo.toml
sed -i 's/^            <param>: <old>,$/            <param>: <new>,/' src/config.rs
cargo build --release
cp target/release/suprah <mm>/engines/ab-<tag>
git checkout -- Cargo.toml src/config.rs
```

`build_and_release.sh` is the release pipeline and rewrites `CHANGELOG.md`; never use it for a
throwaway variant. Plain `cargo build --release` is correct here.

**Verify before spending hours.** Two checks, both seconds:

```bash
# 1. The id name really differs, or the variants collapse into one PGN row.
(echo uci; sleep 0.6) | <mm>/engines/ab-<tag> | grep '^id name'

# 2. The default really took effect. A fixed-depth node count on Kiwipete separates them.
KIWI="r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1"
{ echo uci; sleep 0.3; echo "setoption name OwnBook value false"; echo ucinewgame;
  echo "position fen $KIWI"; echo "go depth 9"; sleep 25; echo quit; } \
  | <mm>/engines/ab-<tag> | grep -o 'nodes [0-9]*' | tail -1
```

A rebuilt release must reproduce its recorded node count exactly. v0.34.0 is 1,192,961 at depth 9
and v0.33.1 is 2,047,451; a mismatch means the build is not the version it claims to be.

---

## 3. Always include the anchor

`suprah-0.34.0` is the **permanent anchor**. Every measurement run includes it, whether or not
the question needs it.

It costs one extra opponent in a gauntlet and buys two things that cannot be recovered afterwards:

* the cross-version check rule 2 mandates for any change to `search_service.rs`,
  `eval_service.rs`, `move_gen_service.rs` or a search parameter default, and
* a place on a single rating scale, so the project has a curve over versions instead of a pile of
  incomparable pairings.

```bash
python3 scripts/version_curve.py <mm>/*.pgn --anchor "Rust-In-Pieces V0.34.0"
```

Runs connect through the engines they share. An engine that never played the anchor, directly or
through a chain, is reported as unplaceable rather than given a number.

**Retiring the anchor** takes one run containing both the old and the new anchor; that pairing
bridges the two scales and `version_curve.py` follows the chain on its own. Do not retire it for
any other reason - the value is entirely in it not moving.

**The scale is machine-bound.** A frozen binary plays differently on a faster host, so ratings
carry across runs on one machine and not across machines. When the work moves hosts, the curve
starts again.

---

## 4. Run it sequentially

```bash
# openings, once per machine
cp openings/book_mixed.txt <mm>/openings_mixed.txt
```

Write `<mm>/<name>.trn` with `rounds` set **generously** - the stopping rule, not the
round count, ends the run:

```
engines = ab-both, ab-lmp, suprah-0.34.0
time_control = 1000
increment = 100
rounds = 2000
pgn = <name>.pgn
engine_options = OwnBook=false, Hash=64, Threads=1
concurrency = 9
openings = openings_mixed.txt
mode = gauntlet
```

`OwnBook=false` matters: the engine carries a 93,000-entry book compiled into the binary and
would otherwise play it on top of the manager's opening line.

```bash
scripts/run_sprt_match.sh <name>.trn <A> <B> --elo0 -10 --elo1 0
```

The wrapper starts the tournament in its own process group, polls the PGN once a minute, and ends
every game the moment the named pairing decides. The other pairings in a gauntlet keep playing
until then, which is what makes the anchor comparison free. The LLR trace is written to
`<mm>/<name>.sprt.log`.

A fixed-length run is still correct when the question is "how big is it" rather than "which way is
it" - a number for `CHANGELOG.md`, say. Use `mm.sh -t` directly for that.

---

## 5. Check the run before believing it

```bash
python3 scripts/match_health.py <mm>/<name>.pgn
```

| What it reports | Act when |
| :--- | :--- |
| Losses on time | **Any at all.** Lower `concurrency` and re-run; forfeits do not fall evenly on the two engines. |
| Identical games | More than a handful. The opening pool is being replayed and duplicate games are counted as observations without carrying new information. |
| White's score | Above roughly 60%. The pairing cancels it, but it inflates pair variance and therefore the cost of every comparison. |
| Distinct openings | Far below the line count of the pool. `openings/book_mixed.txt` has 598 lines but only 17 distinct four-ply starts, so a run samples far fewer opening families than lines. |
| Pair outcome shape | Feeds `sprt.py --plan` for the next run. |

Then read the Elo per pairing:

```bash
python3 scripts/pairing_elo.py <mm>/<name>.pgn      # paired interval is the honest one
python3 scripts/sprt.py <mm>/<name>.pgn --engines <A> <B> --trajectory
```

`--trajectory` replays the run and shows where the test would have stopped. Run it on every
fixed-length match afterwards: it is how the saving from sequential testing gets recorded instead
of assumed.

---

## 6. The opening pool

> [!NOTE]
> **`books/` is in `.gitignore` and is not in version control** — thirteen PolyGlot books, 58 MB
> of binaries. A fresh clone has none of them and every command below fails with a missing file.
> Only `Performance.bin` in the repository root is tracked, and it is the book compiled into the
> engine, not a good sampler. Copy `books/` across by hand when the work moves machines.

`books/` holds thirteen PolyGlot books and they differ enormously in how wide a pool they can
produce. `scripts/book_lines.py --survey` measures it directly - it walks the book file itself,
needs no engine, and takes about a second for all of them.

Measured over 400 sampled 10-ply lines, distinct four-ply starts:

| Book | root moves | distinct @4 | distinct @8 |
| :--- | ---: | ---: | ---: |
| `codekiddy.bin` | 10 | **307** | 396 |
| `DCbook_large.bin` | 6 | 284 | 395 |
| `Elo2400.bin` | 13 | 279 | 389 |
| `final-book.bin` | 11 | 200 | 262 |
| `KomodoVariety.bin` | 17 | 147 | 177 |
| `Performance.bin` | 3 | **17** | 105 |
| `komodo.bin` | 2 | 3 | 48 |

`Performance.bin` is the book compiled into the engine, and it is close to the worst of them for
this purpose: it is a popularity book with three root moves. `komodo.bin` is worse still - a deep
best-play book with two. Neither is a defect; they are built to play well, not to spread.

`gavibook.bin` and `gavibook-small.bin` are not PolyGlot books - their size is not a multiple of
16 bytes - and the survey says so rather than reading garbage.

### Building a pool

```bash
scripts/book_lines.py --self-test        # nine published key vectors, plus castling
scripts/book_lines.py --book books/codekiddy.bin --plies 10 --count 2000 \
    --temperature 0.25 --out openings/book_codekiddy_10ply.txt
cp openings/book_codekiddy_10ply.txt <mm>/openings_mixed.txt
```

`--temperature` is the one parameter that matters. At 1 moves are drawn by book weight, which is
what the engine's own sampler does and what leaves a pool seventeen openings wide. At 0 they are
drawn uniformly. **0.25 is better than either**: it produces more distinct prefixes *and* more
usable lines than uniform sampling, because following weight keeps the walk inside the book
instead of running it off the edge.

The resulting pool against the one in use:

| Pool | lines | @2 | @4 | @6 | @8 |
| :--- | ---: | ---: | ---: | ---: | ---: |
| `book_mixed.txt` (Performance.bin) | 598 | 4 | 17 | 49 | 121 |
| `book_codekiddy_10ply.txt` | 2000 | 83 | **831** | 1621 | 1892 |

Use **even ply lengths only**, so White is on move at handover as in every earlier measurement.

### What this does and does not fix

It fixes **external validity**: a result measured on 17 opening families is Elo over those
families, and a change that helps in sharp open positions and not in closed ones is mis-priced by
such a pool. It is the most likely mechanism behind the disagreements `task.md` records between
runs, including the Check Extension "deep only" axis reversing from +34.2 to -16.0.

It does **not** fix an interval that was already honest. The intraclass correlation of pair scores
within an opening family measures 0.00 to 0.03 on the narrow pool, i.e. a design effect of 1.00 to
1.27. The old pool was not inflating anyone's confidence.

**Whether a wider pool costs or saves games is an open question, not a known win.** Broader
openings are sharper, sharper openings raise the pair variance, and higher variance means more
games per decision - but sharper positions also convert small strength differences into results
instead of drawing them away. The two effects work against each other and which wins is empirical.
Measure it, do not argue it: run the same pairing on both pools and compare
`scripts/sprt.py --plan`. The pool with the lower expected game count is the cheaper one.

Balance filtering - discarding lines whose final position already favours one side - is the other
untested knob. It needs the engine to evaluate every candidate line and has not been done.

## 7. Throughput of the harness

Roughly 40 games per minute at `concurrency = 9` on a 12-core host, so a 2000-game pairing is
about 50 minutes. Do not compile or run tests while a match is running: the games are
time-controlled and a busy core shows up as a forfeit.

`match_health.py` reporting zero losses on time is the evidence that the concurrency is safe.
Raising it is a legitimate way to buy throughput, but it has to be re-checked at the new setting,
and a run whose forfeits are non-zero is discarded rather than corrected.
