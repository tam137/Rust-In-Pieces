---
name: Matchplay Measurement Procedure
description: Standard Operating Procedure for matchplay, SPRT, and strength measurement in Suprah.
---

# Matchplay Measurement Procedure

How to price a search change in games, at the lowest game count that can honestly answer the
question asked. Rule 1 of `task.md` makes matchplay the only currency; this document is about
making it affordable, reliable, and statistically sound.

Read `task.md` "Rules that are not optional" first. Nothing here overrides them.

---

## 1. Resolving the Match Manager Location (`<mm>`) & Host Isolation Rules

`<mm>` throughout this repository, its skills, and documentation refers to the **Matt-Magie working directory**:
where engine binaries, tournament `.trn` files, opening pools, and PGN logs reside.

Because `<mm>` is a sibling of this repository on some hosts and located elsewhere on others, you MUST
resolve it once at the start of a session:

```bash
# From the repository root: take the first match containing mm.sh and engines/
find .. ~ -maxdepth 3 -name mm.sh -type f 2>/dev/null
```

> [!IMPORTANT]
> **Never hardcode or write the resolved path into committed files.**
> Per `AGENTS.md`, the resolved absolute path must never be written into `task.md`, `CHANGELOG.md`,
> or skill documents. Only `build_and_release.sh` and `scripts/run_sprt_match.sh` carry a default
> and allow an `MM_DIR` override.
>
> **Elo numbers do not carry across hosts and architectures.**
> Nothing in `<mm>` crosses between hosts: `*.pgn` and `books/` are gitignored, so only `.trn` files
> and the repository itself travel.
> - **Compare ratings only within one run, and never read two PGNs from different hosts together.**
> - **Engine binaries do not travel between hosts.** Anything already in `<mm>/engines/` was built
>   for whatever architecture put it there; rebuild natively before using it.
> - **Per-host figures must be re-measured on each host, never carried over.** This includes
>   concurrency caps (`floor(nproc * 0.75) - 1`) and games per minute.

---

## 2. The Measurement Tooling Infrastructure

The engine repository provides dedicated Python tools under `scripts/` to drive matches, compute
statistics, and verify health.

| Tool | Purpose |
| :--- | :--- |
| [`scripts/sprt.py`](../scripts/sprt.py) | Sequential Probability Ratio Test (pentanomial GSPRT) over paired openings. Also supports `--plan` and `--trajectory`. |
| [`scripts/run_sprt_match.sh`](../scripts/run_sprt_match.sh) | Match runner wrapper that stops the tournament the moment the SPRT decision bound is reached. |
| [`scripts/match_health.py`](../scripts/match_health.py) | Quality audit of PGNs: verifies forfeits, duplicate games, White/Black win balance, and design effect. |
| [`scripts/pairing_elo.py`](../scripts/pairing_elo.py) | Computes honest paired intervals per pairing from a PGN. |
| [`scripts/version_curve.py`](../scripts/version_curve.py) | Connects multiple runs onto a single rating scale anchored to a frozen reference binary. |
| [`scripts/uci_driver.py`](../scripts/uci_driver.py) | Deterministic UCI harness that **waits for `bestmove`**. Prohibits fixed `sleep` timeouts. |
| [`scripts/measure_cold_warm_drift.py`](../scripts/measure_cold_warm_drift.py) | Measures Transposition Table and pawn hash table stability between cold and warm states. |
| [`scripts/book_lines.py`](../scripts/book_lines.py) | Direct PolyGlot book parser and sampler for creating wide, balanced opening pools. |

> [!WARNING]
> **Never use arbitrary `sleep` timeouts for benchmarking or fixed-depth searches.**
> A fixed `sleep` terminates searches mid-iteration depending on host CPU load. Always use
> [`scripts/uci_driver.py`](../scripts/uci_driver.py) to await `bestmove` tokens.

---

## 3. The Question Decides the Cost

Before any binary is built, write down the decision the run has to support. There are three
shapes, and they cost very different amounts.

| Shape | Hypotheses | Typical cost |
| :--- | :--- | :--- |
| **Does it gain?** A new feature, default off, that has to earn its place. | `--elo0 0 --elo1 10` | High. Rejecting a null feature is the expensive direction. |
| **Does it hurt?** A rule that is already written and free to keep; only a regression would stop it shipping. | `--elo0 -10 --elo1 0` | Roughly half of the above when the feature is genuinely good. |
| **Is it a regression?** A cross-version check against a released engine. | `--elo0 -10 --elo1 0` | Cheap when the answer is no. |

**Price the run before starting it.** From any earlier PGN of the same engine family:

```bash
python3 scripts/sprt.py <any-earlier.pgn> --engines <A> <B> --plan
```

This reads the distribution of paired outcomes the harness actually produces — not an assumed
draw rate — and prints, for a range of true effects, how many games a sequential test needs and
what a fixed run of the same length would resolve. If the effect you are hunting is smaller than
the half-width in the last column, the run cannot answer the question and must be resized or
restated before it is worth starting.

Suprah is a **sharp** engine: only about a third of its pairs come back level, and the pair
variance is near 0.065. That is what makes small effects expensive here.

---

## 4. Stopped SPRT vs. Fixed-N Effect Size

> [!IMPORTANT]
> **A stopped SPRT decides *whether to ship*, never *how much was gained*.**
> A sequential test stops the moment evidence crosses a bound, which means it stops on favorable
> statistical fluctuations. This biases point estimates upwards by a factor of 3 to 4.

* **Gating / Release Decision:** Use `scripts/run_sprt_match.sh` with `--elo0 -10 --elo1 0` or
  `--elo0 0 --elo1 10`.
* **Effect Size Estimation (`CHANGELOG.md`):** Use a fixed game count (`rounds` in `.trn` run with
  `./mm.sh -t`), decided in advance before the match starts. The stopping rule and the effect size
  must never come from the same run.

---

## 5. Build the Variants

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

**Verify before spending hours:**

```bash
# 1. Verify id name differs (avoids collapsing into one PGN row)
(echo uci; sleep 0.6) | <mm>/engines/ab-<tag> | grep '^id name'

# 2. Verify default took effect via Kiwipete node count
python3 -c '
import subprocess
p = subprocess.Popen(["<mm>/engines/ab-<tag>"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
p.stdin.write("uci\nsetoption name OwnBook value false\nucinewgame\nposition fen r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1\ngo depth 9\n")
p.stdin.flush()
for line in p.stdout:
    if line.startswith("bestmove"): break
    if "nodes" in line: print(line.strip())
p.stdin.write("quit\n")
p.stdin.flush()
'
```

---

## 6. Always Include the Permanent Anchor

`suprah-0.34.0` is the **permanent anchor**. Every measurement run includes it, whether or not
the question needs it.

It costs one extra opponent in a gauntlet and buys two things:
1. The cross-version check rule 2 mandates for changes in search/eval logic.
2. A place on a single rating scale connected via `scripts/version_curve.py`.

```bash
python3 scripts/version_curve.py <mm>/*.pgn --anchor "Rust-In-Pieces V0.34.0"
```

The scale is machine-bound: when work moves to a different host architecture, the rating curve
re-anchors natively.

---

## 7. Execution & Concurrency

### Hardware Concurrency Cap
At least 25% of CPU cores must remain free to guarantee zero forfeits on time:

$$\text{concurrency} = \lfloor \text{nproc} \times 0.75 \rfloor - 1$$

* On an 8-core host: `concurrency = 5`
* On a 12-core host: `concurrency = 8`
* On a 20-core host: `concurrency = 14`

Any run with non-zero forfeits on time is **discarded immediately**. Do not compile or run test
suites on the host while a match is running.

### Tournament Configuration (`<mm>/<name>.trn`)
```ini
engines = ab-candidate, ab-baseline
time_control = 1000
increment = 100
rounds = 5000
pgn = <name>.pgn
engine_options = OwnBook=false, Hash=64, Threads=1
concurrency = 5
openings = book_width.txt
mode = gauntlet
```

Run sequentially:
```bash
scripts/run_sprt_match.sh <name>.trn <candidate> <baseline> --elo0 -0 --elo1 10
```

---

## 8. The Opening Pool

The standard opening pool is [`book_width.txt`](../openings/book_dclarge_10ply.txt) (1,200 lines,
613 distinct 4-ply starts, generated from `DCbook_large.bin` with uniform sampling).

Install once on the host:
```bash
cp openings/book_width.txt <mm>/book_width.txt
```

### Opening Pool Generation Rules:
- Always use **even ply lengths** (e.g. 10 plies) so that White is on move at handover.
- Sampling with `--temperature 0.25` balances prefix breadth with book path validity.
- Survey available books using `python3 scripts/book_lines.py --survey`.

---

## 9. Audit the Run Before Believing It

```bash
python3 scripts/match_health.py <mm>/<name>.pgn
```

| Metric | Threshold / Action |
| :--- | :--- |
| **Losses on time** | **Must be 0.** Any forfeit invalidates the run. Lower concurrency and discard. |
| **Identical games** | Must be negligible. If duplicate games occur frequently, pool is too narrow. |
| **White's score** | Must be below ~60%. If higher, pool is biased and inflates pair variance. |
| **Design effect ($D_{\text{eff}}$)** | Must be close to 1.0. If $D_{\text{eff}} > 1.3$, opening clustering is degrading effective sample size. |

Then calculate paired Elo:
```bash
python3 scripts/pairing_elo.py <mm>/<name>.pgn
python3 scripts/sprt.py <mm>/<name>.pgn --engines <A> <B> --trajectory
```
