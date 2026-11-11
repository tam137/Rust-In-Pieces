# NNUE Training Plan

Plan for training this engine's own NNUE networks, data generation and training included, on a
single workstation. Status: preparation. This document is the public plan; the step-by-step log
lives with the match manager (`<mm>`), which is resolved once per session as described in
`AGENTS.md`.

## 1. Goal

A network trained here end to end. The first network (**N1**) reproduces the properties of the
current one exactly - architecture, quantisation, file size and output scale - so the whole
pipeline is proven and the engine loads it without a code change. After that the layout is free:
the expected playing strength decides. Every network is accepted only by an SPRT [0, 5] against the
current NNUE release, with the usual [0, 10] guard and a control match at a longer time control
(`skills/matchplay_measurement_procedure.md`, `scripts/gate.py`).

## 2. The current network

`eval_models/quantised.bin` was trained for the engine SleepMind and is used with permission. It
was trained with the Rust trainer [bullet](https://github.com/jw1912/bullet). The specification
below is read from `src/nnue_service.rs` and from that engine's published training configuration.

| Property | Value |
| :--- | :--- |
| Inputs | 768 = 64 squares x 6 piece types x 2 colours, **10 king buckets**, horizontally mirrored (king on files e-h mirrors the board), bullet `ChessBucketsMirrored` |
| Bucket layout (half board, 32 squares, rank 1 first) | `0 1 2 3 / 4 4 5 5 / 6 6 6 6 / 7 7 7 7 / 8 8 8 8 / 8 8 8 8 / 9 9 9 9 / 9 9 9 9` |
| Hidden layer | **256** per perspective, **SCReLU**, two perspectives (side to move first) |
| Output | one value, **8 output buckets** by piece count, `(pieces - 2) / 4` (bullet `MaterialCount<8>`) |
| Factoriser | `l0f` (768 x hidden), merged into all 10 buckets when the net is saved |
| Quantisation | `l0w`, `l0b` x255 (QA); `l1w` x64 (QB, transposed); `l1b` x255*64 |
| Output scale | `NNUE_SCALE = 400` in the engine, `eval_scale = 400` in training |
| File | i16 little endian, `l0w l0b l1w l1b`, padded to 64 bytes: **3,940,928 bytes** |
| Optimiser | AdamW, `l0w` and `l0f` clipped to +-0.99 |
| Learning rate | cosine decay from 0.001 to 0.001 * 0.3^5 |
| Batches | 16,384 positions, 6,104 batches per superbatch (about 100 M positions) |
| WDL proportion | probably 0.5 (unconfirmed) |
| Superbatches, data size | unknown |

The scale is the one property that must not move silently: every search margin in centipawns
(RFP, futility, razoring, ProbCut, aspiration) is calibrated on it.

## 3. Host

16 hardware threads, x86-64 with AVX2 and AVX-512; 12 threads may be used for compute (data
generation, matches, tuning). No discrete GPU: an integrated RDNA3 GPU (gfx1103).

**ROCm on an unlisted RDNA3 iGPU** (verified 2026-11-10, ROCm 7.1 from the distribution's packages:
`rocminfo hipcc libamdhip64-dev libhipblas-dev librocblas-dev`, user in the groups `render` and
`video`). The distribution's rocBLAS ships no gfx1103 kernels; without an override a plain HIP
kernel runs but hipBLAS fails. What works: **build device code for `--offload-arch=gfx1100` and run
with `HSA_OVERRIDE_GFX_VERSION=11.0.0`**. A test kernel and an SGEMM 1024^3 against a CPU reference
were correct (max. error 2.8e-4) at about 1.1 TFLOP/s. bullet's HIP kernels have to be built for
gfx1100 the same way. bullet's CPU backend is meant for tests and small networks only.

The x86 release builds set no `target-cpu`, so they use SSE2 only, and `src/nnue_service.rs` has no
explicit SIMD paths. This matters for how wide a network can be (section 5).

## 4. Stages

### Stage 0 - Tools and compatibility
1. ROCm installed and verified (section 3).
2. **Done (2026-11-11):** bullet cloned outside the repository and pinned to
   `4e9317ffb07ee01b3ae5202d083526fc1d90fa2f` (2025-12-16), the revision the current network was
   trained with; later revisions replaced the HIP backend and changed the trainer API.
   `bullet-utils` built for the CPU. bullet's HIP kernels take their target from `GCN_ARCH_NAME`;
   built for gfx1100 and run with the override from section 3, a small example network trains on
   the iGPU at about 1.6 M positions per second.
3. **Done (2026-11-11):** trainer crate in this directory (section 7): architecture N1 as in
   section 2, command-line arguments for data, superbatches, start superbatch, resuming, name,
   WDL proportion, hidden size and scale; bullet pinned by revision and `Cargo.lock`. A smoke run
   on the iGPU writes a 3,940,928-byte network and resumes from a checkpoint.
4. Mini run on a few million positions from a throwaway data set (generated with SleepMind's
   existing generator, only to test the chain, then deleted): 1-2 superbatches; measure positions
   per second on the iGPU, memory and stability over at least an hour. The file must have
   3,940,928 bytes and load through `NNUENetwork::load_from_bytes`.
5. **Done (2026-11-11):** compatibility tests in the engine (`src/nnue_compat_tests.rs`, section
   8), green for the current network and for a trained test network.
6. Data and networks live in the match manager's working directory (`<mm>/nnue_data/`,
   `<mm>/nnue_nets/`), never in the repository.

### Stage A - Data generation mode
**Done (2026-11-11):** `src/datagen.rs` behind the feature `datagen`, start script
`scripts/run_datagen.sh`; usage in section 9. Decisions: win adjudication at 2000 cp over 4 plies,
draw adjudication from move 40 at |score| <= 10 cp over 8 plies, move limit 400, no Syzygy yet,
one single-threaded process per core, 2^20 hash entries, tables cleared per game.

- Behind a Cargo feature `datagen`; the release binary is unchanged.
- Self-play with a node limit per move given as an argument; start value **5,000 soft nodes**
  (the search stops after the iteration in which the limit is reached instead of inside it), one
  thread, small hash, tables cleared per game. 5,000 matches the data of the current network, so N1
  tests only the pipeline and the amount of data.
- More nodes give better labels but proportionally fewer positions per day; where the trade turns
  is only measurable. Later (stage F): two networks from about 100 M positions each, 5,000 against
  15,000-20,000 nodes **at equal compute**, decided by an SPRT.
- Openings: **no opening book.** The match manager's opening lists (about 1,800 lines) are far too
  few for 100 M games and stay reserved for measurements, so an SPRT does not partly measure
  memorisation. Instead 8-12 random legal plies from the start position, each ply random with
  probability 50 % and otherwise the search move; drop the game if |eval| > 400 cp after them; the
  random plies are not recorded, early positions are filtered by move number (duplicates). A large
  EPD book (also Chess960 start positions) for data generation only is an option for later.
- Optional Syzygy tablebases (3-5 pieces) for exact endgame results.
- Check the draw adjudication: the throwaway data set has 61 % draws (21 % / 18 % wins).
- End: mate, stalemate, the match manager's draw rules; adjudication of a win at |eval| >= 2000 cp
  and of a draw at |eval| <= 10 cp from move 40, each over several moves.
- Record `fen | eval | wdl`, **score and result from White's point of view** (bullet's text
  format), then `bullet-utils convert / interleave / shuffle`.
- Filter: side to move in check, best move a capture or promotion, mate scores.
- Seed per process; counters for games, dropped games, filtered positions, positions per second.
- Tests: sign with Black to move, result mapping, filters, determinism for a fixed seed.
- A start script for up to 12 processes under `scripts/`.
- Labels: first from the NNUE engine (current network), later a comparison network from the HCE
  engine.

### Stage B - Trial run
**Done (2026-11-11):** one process, 16,000 games, 1,224,181 positions at about 560 positions per
second. 37 % of the started games were dropped after the opening (cheap, about 10 plies each),
27 % of the searched positions filtered. Games: 41.5 % White wins, 19.6 % draws, 39.0 % Black wins,
75 plies on average; 11 % of the positions have six pieces or fewer. The score agrees in sign with
a material lead of three pawns or more in 98.3 % of the cases, and the mean result rises
monotonically with the score (0.08 below -600 cp, 0.92 above +600 cp). The maximum-likelihood
sigmoid scale is 417, close to 400, so the scale stays. `bullet-utils convert` accepts the file;
text takes 64 bytes per position.

About 1 M positions on one process: throughput, share filtered, game length, result
distribution, disk use; decode a sample back (legal FEN, sign, result). Fit the eval-to-result
sigmoid on the sample; if the fit is far from 400 it is recorded, the scale stays 400 for N1.

### Stage C - Data
**Running (from 2026-11-11):** twelve processes give about 5,100 positions per second (18 M per
hour). A first batch of about 100 M positions is followed by an interim network, the compatibility
tests and a short gauntlet against the current NNUE release; only if the pipeline holds in play
does generation continue to about 400 M.

First round 300-500 M positions (at least 100 M for a test network), up to 12 processes, only while
no measurement runs on the same cores.

### Stage D - Training
Configuration as in section 2, WDL 0.5, scale 400; first about one to two passes over the data,
longer while the loss still falls; checkpoints every few superbatches. Checks: size, loading, the
compatibility tests, reference positions against the old network.

### Stage E - Acceptance
Candidate with the new network embedded and otherwise identical; census, `scripts/gate.py` against
the current NNUE release. If a network ships, the NNUE search margins and the LMR divisor are
re-checked afterwards.

### Stage F - Iterate
New data with the new network, old plus new data; only then wider networks.

## 5. Network choice

Width is a trade: more neurons evaluate more precisely but cost time per node (accumulator updates
and the output grow linearly with the width) and need more data. Two limits apply here: the data
twelve threads can produce, and inference without a SIMD target. Reference point: SleepMind went
from 256 to 768 neurons and measured +39 Elo at 60s + 0.6s, together with about 1.5 B positions,
WDL 0.5 and scale 295 - not the width alone.

| Net | Layout | Data (rough) | Inference cost | Purpose |
| :--- | :--- | :--- | :--- | :--- |
| **N1** | (768x10 mirrored -> 256)x2 -> 1x8, SCReLU, scale 400 | 300-500 M | as today | baseline; proves the chain, no code change |
| **N2** | the same with **512** hidden | >= 1 B | about 2x accumulator and output | the expected main gain once inference is faster |
| N3 | 768 or 1024 hidden | >= 1.5-2 B | about 3-4x | only if N2 clearly gains |
| later | a second layer, more output buckets | much more | code and data | separate plan |

N1 comes first and unchanged: a pipeline error (sign, mirroring, scale) is otherwise
indistinguishable from a poor choice of network. Two engine steps come before N2, independent of
training:

- **I1 - SIMD in inference:** `target-cpu` (for example `x86-64-v3`) for the x86 builds or explicit
  AVX2 paths for the accumulator and the SCReLU output; ARM builds have NEON anyway. Tree-identical,
  nodes per second measured, own SPRT; the target has to match every machine the builds run on.
- **I2 - Cost before training:** builds with 512 and 768 hidden and random weights, nodes per
  second against 256, so the Elo a wider net has to recover is known before it is trained.
  Needs `NNUE_HIDDEN_SIZE` per build and a size check on load; measured after I1.

## 6. Risks

- Format or perspective errors make a network worthless without an obvious symptom: stage 0 and
  stage B are mandatory before compute goes into data.
- The iGPU path is unofficial; if it fails, training needs a separate plan (for example a CPU
  trainer using AVX-512) before data is produced.
- Too little data gives a weaker first network: add data before changing the architecture.
- Data generation shares cores with measurements and must not run beside an SPRT.
- Nothing in this repository names hosts, absolute paths or credentials.

## 7. Trainer

`nnue_training/` is a Cargo workspace of its own, so the engine build never compiles bullet or a
GPU backend. Exactly one backend feature has to be selected:

```bash
cd nnue_training
cargo test --features cpu                                     # unit tests, no GPU needed
HIP_PATH=/usr GCN_ARCH_NAME=gfx1100 cargo build --release --features hip
```

**Only one GPU process at a time.** On the RDNA3 iGPU a second compute process (even a short
evaluation) next to a running training hung the GPU scheduler; the driver reset the GPU and the
training aborted with `hipErrorLaunchFailure` (2026-11-11). Evaluations beside a training run use a
CPU build: `cargo build --release --features cpu --target-dir target/cpu`.

`HIP_PATH` is the ROCm prefix (`/usr` for the distribution's packages) and `GCN_ARCH_NAME` the
device-code target; on the RDNA3 iGPU of section 3 the binary runs with
`HSA_OVERRIDE_GFX_VERSION=11.0.0`.

Data goes in as shuffled bullet binary files (`bullet-utils convert --from text`, `interleave`,
`shuffle`). A run of eight superbatches of about 100 M positions each:

```bash
HSA_OVERRIDE_GFX_VERSION=11.0.0 target/release/suprah-nnue-trainer \
    --data <mm>/nnue_data/n1.data --superbatches 8 --name n1 --output-dir <mm>/nnue_nets
```

Checkpoints land in `<output-dir>/<name>-<superbatch>/` (`quantised.bin` for the engine,
`raw.bin`, `optimiser_state/`); `--resume <checkpoint> --start <next superbatch>` continues a run.
After the last superbatch the trainer checks that `quantised.bin` has the size the engine expects
for the chosen hidden size and exits with an error otherwise. `--help` lists all options. The
defaults are N1: 256 hidden neurons, scale 400, WDL 0.5, cosine learning rate from 0.001 to
0.001 * 0.3^5. A different hidden size or scale trains, but the engine loads it only after the
code change of section 5.

## 8. Compatibility tests

The engine checks every network against the trainer before it is embedded
(`src/nnue_compat_tests.rs`; the network is the embedded one or the file named by `NNUE_TEST_NET`):

| Test | Checks | Catches |
| :--- | :--- | :--- |
| colour flip | a position and its colour mirror give exactly the negated score | engine feature transform (holds for any weights) |
| horizontal mirror | a position without castling rights and its file mirror give exactly the same score | engine king-bucket mirroring (holds for any weights) |
| material sign | extra material is positive for its owner, both sides to move, output buckets 0-7; +300 cp where pawns or pieces remain, the sign only for bare-king endings (rare in the data) | networks trained with a wrong perspective, square order or output layout |
| output bucket | the engine's bucket equals bullet's `MaterialCount<8>` | bucket formula drift |
| trainer reference | the engine's quantised score of `nnue_training/reference_fens.txt` equals the trainer's floating-point score within 40 cp + 5 % (rounding the output weights to 1/64 alone moves scores by tens of centipawns; pipeline errors move them by hundreds) | any mismatch between trainer and engine: layout, quantisation, transposition, scale |

The reference comparison needs the trainer's output for the same checkpoint and is therefore
`#[ignore]`d by default:

```bash
nnue_training/target/cpu/release/suprah-nnue-trainer \
    --eval-fens nnue_training/reference_fens.txt --resume <checkpoint> > reference.txt
NNUE_TEST_NET=<checkpoint>/quantised.bin NNUE_TEST_REFERENCE=reference.txt \
    cargo test --release nnue_compat -- --include-ignored
```

Test networks trained on 5 M throwaway positions (1 B and 3 B positions seen) passed all of them;
the largest differences to the trainer were 23.4 and 22.4 cp. A reference with negated scores
fails, and a barely trained network fails the material test. The overtrained network valued
K+Q v K at +215 and K+R v K at +50 - the trainer gives the same numbers, so this is the data, not
the pipeline.

## 9. Data generation

```bash
cargo build --release --features datagen --target-dir target/datagen
target/datagen/release/suprah datagen --help
scripts/run_datagen.sh target/datagen/release/suprah <mm>/nnue_data/<batch> 12 <games> <base seed>
```

The feature build gets its own target directory so the release binary in `target/release` is
never replaced by it; without the feature the binary ignores `datagen` and speaks UCI. Each
process appends whole games to `part_<i>.txt` (`fen | score | result`, both from White's view)
and reports games, dropped games, positions, the share of filtered positions, the result
distribution, plies per game and positions per second. Equal seeds give equal data; give every
batch a new base seed. The text files go through `bullet-utils convert --from text`,
`interleave` and `shuffle` before training.
