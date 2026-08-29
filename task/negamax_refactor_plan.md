# Negamax Refactor — Implementation Plan (`task.md` section 7)

`[Impact: Low]` `[Complexity: High]` — a **pure refactor with no expected Elo**, in the file whose
last large change cost 209 Elo (`task.md` 8.1). The whole plan is therefore built around one
question: *can the change be proven to leave the search tree bit-identical before a single game is
played?* Per `task.md` rule 5, a node-identity check is preferred to a match wherever one exists,
and this item is exactly that case.

> Priority order and the record of what has been tried and failed live in [`task.md`](../task.md).

---

## 1. The current state

`SearchService::minimax` is an **asymmetric minimax on absolute, White-positive scores**. Every
pruning rule, the PVS null window, the fail-hard running score and the Quiescence stand-pat
carry parallel `if white { ... } else { ... }` blocks, because a White node maximises against
`beta` while a Black node minimises against `alpha`.

| | |
| :--- | :--- |
| `minimax` | `src/search_service.rs`, roughly 1170 lines in one function |
| Colour branches | 20 `if white` sites inside `minimax`, 2 more in `singular_verification` |
| Score scale | absolute, White-positive. `EvalService::calc_eval` and the NNUE wrapper both return it |
| Boundary | `SearchResult` is absolute; `uci_parser_service.rs` negates by `is_white_move` |

### Three invariants the refactor rests on, each verified rather than assumed

1. **`white == board.white_to_move` at every entry to `minimax`.** A `debug_assert_eq!` on that
   equality survived the full 174-test suite in a debug build and four hand-run searches
   (start position, Kiwipete from either side, and a pawn endgame) before the parameter was
   removed. It is now structural rather than asserted: the value is read off the board.
2. **`minimax` is never entered at `ply == 0`.** `get_moves` enters at `ply = 1`; the three
   same-node re-entries pass `ply` unchanged. Verified by `debug_assert!(ply > 0)` over the same
   corpus.
3. **Consequently Mate Distance Pruning always runs**, and it clamps the window into
   `[-(MATE_SCORE - ply), MATE_SCORE - ply]` before any other code reads `alpha` or `beta`.
   Every negation inside `minimax` is therefore exact — `-i16::MIN` is unreachable there.

Invariant 3 also settles a question that looks like a defect and is not: `alpha.abs() < 20000`
guards LMP, Futility Pruning, razoring and SEE pruning, and `i16::MIN.abs()` panics in a debug
build. It never fires, because Mate Distance Pruning has already replaced an `i16::MIN` alpha.

---

## 2. The transformation, site by site

Negamax expresses every node as a maximiser over side-to-move-relative scores. The mapping from
this engine's absolute scale is `score_rel = score_abs` at a White node and `score_rel =
-score_abs` at a Black node; a window maps as `(alpha_rel, beta_rel) = (-beta_abs, -alpha_abs)`
at a Black node.

Each row below is the identity argument for one site: the Black branch, rewritten through that
mapping, becomes the White branch character for character. That is what makes the merge a
refactor rather than a change.

| # | Site | Today | After | Note |
| ---: | :--- | :--- | :--- | :--- |
| 1 | Mate Distance Pruning | already colour-neutral | unchanged | maps to itself |
| 2 | TT probe | `alpha.max` / `beta.min` | unchanged | the stored scale becomes relative; the probe is already written in terms of the node's own window |
| 2a | TT write | classified against `orig_alpha`/`orig_beta` | `Self::bound_for`, **still colour-dependent** | the one site the conversion could not merge — see section 7 |
| 3 | `static_eval` | absolute | negated once at its single assignment | every consumer then loses its colour branch |
| 4 | Null Move Pruning | `(beta-1, beta)` / `(alpha, alpha+1)` | `(-beta, -beta+1)`, result negated | the side to move flips although no move was played |
| 5 | NMP verification search | `(alpha, beta)`, same `white` | `(alpha, beta)`, **not** negated | same node |
| 6 | Reverse Futility Pruning | two branches | one | exact mirror |
| 7 | Razoring | two branches | one, **not** negated | same node |
| 8 | Quiescence stand-pat + its two TT stores | two ~35-line branches | one | exact mirror |
| 9 | Delta pruning | two branches | one | exact mirror |
| 10 | Terminal mate score | `WhiteWin` / `BlackWin` split | `-(MATE_SCORE - ply)` | the winner is never the side to move at a terminal node |
| 11 | Fail-hard running score | `if white { alpha } else { beta }` | `alpha` | **`task.md` 8.1 — must stay fail-hard** |
| 12 | Futility Pruning | two branches | one | exact mirror |
| 13 | Late Move Reductions | `(alpha, alpha+1)` / `(beta-1, beta)` | `(-alpha-1, -alpha)`, result negated | exact mirror |
| 14 | Principal Variation Search | two branches, both windows and both re-search tests | one, result negated | exact mirror |
| 15 | Score update, killers, history | two branches | one | exact mirror |
| 16 | `singular_verification` bound test | `LowerBound => white`, `UpperBound => !white` | `LowerBound => true`, `UpperBound => false` | the compensation for absoluteness disappears |
| 17 | `singular_verification` threshold | two branches | one, **not** negated | same node |
| 18 | `get_moves` root call | absolute throughout | one adapter: window negated in, score negated out | the root keeps the absolute scale |

### The single highest-risk rule

**Three of the ten recursive calls re-enter the same node and must not flip the sign**: the Null
Move verification search, the razoring Quiescence Search, and the singular verification. Each runs
on the same board, the same side to move and the same `ply` — only the move list or the depth
differs. Every other recursion follows a `do_move` or a null move and must negate both its window
and its result. Getting one of these backwards produces a search that still compiles, still
returns plausible scores, and is wrong by a mirror.

---

## 3. Scope, and what is deliberately left alone

**In scope**: `SearchService::minimax`, `singular_verification`, `is_singular`, and one adapter in
`get_moves`. The `white` parameter is dropped from all three.

**Out of scope, deliberately**:

* **`get_moves` keeps absolute scores.** Converting the root as well would reach into
  `SearchResult`, the aspiration window, `uci_parser_service.rs`, `game_handler.rs` and their
  tests. One adapter line at the single call site contains the blast radius to one file.
* **`EvalService::calc_eval` keeps returning absolute scores.** Its Lazy Evaluation cutoffs
  compare an absolute `cheap` score against the window, so the window is converted back to
  absolute at the three call sites instead. Making evaluation side-relative is a 3100-line change
  with its own test surface and belongs to a separate item.
* **Mate Distance Pruning is not upgraded.** Negamax makes the canonical, stronger form
  (`beta = min(beta, MATE - ply - 1)`) natural, and it would change the tree. A refactor that
  changes the tree cannot be verified by node identity, which is the whole gate here.
* **The `turn` aliasing under Null Move Pruning is not fixed.** The null child receives the
  *parent's* `turn`, so `turn.gives_check` and the counter-move index describe the move before the
  null rather than the null itself. It is pre-existing, it is load-bearing for the current tree,
  and correcting it here would destroy the identity proof. Record it, do not touch it.

---

## 4. Side effects to watch

1. **The Transposition Table's score convention flips** from absolute to side-to-move-relative,
   and a Black node's `LowerBound` becomes an `UpperBound`. This is contained: `search_service.rs`
   is the only reader of `TranspositionEntry::eval`, and `move_gen_service.rs` reads only
   `best_move`. Nothing persists a table across builds, so no mixed-convention table can exist.
   Mate-score normalisation is unaffected — mate scores are symmetric under the mapping.
2. **`-i16::MIN` overflows.** `i16::MIN` is the root's negative infinity and negating it panics in
   a debug build. It is negated exactly once, in the `get_moves` adapter, with `saturating_neg`.
   The resulting `i16::MIN + 1` is erased by the child's own Mate Distance Pruning before any code
   reads it, so the tree does not move. Inside `minimax` the sentinel becomes `-i16::MAX` for the
   same reason; both sentinel sites are provably dead stores.
3. **The fail-hard running score must survive the merge.** `eval` starts *at the window bound*.
   Starting it outside is fail-soft, which `task.md` 8.1 measured at roughly -200 Elo.
4. **`search-diag` counters must keep firing at the same points**, or `measure_stage0.py`,
   `measure_razoring.py` and `measure_check_exemption.py` silently change meaning.
5. **The NNUE branch will conflict hard.** `feature/nnue-evaluation` is one release behind at
   v0.36.0-NNUE and carries its own copy of this file. `task.md` section 9 already records that
   the branch has not played a game since v0.30.0-nnue.
6. **A colour asymmetry exists in the engine today** — see section 6. It is not created by this
   refactor, and the identity check is what localises it.

---

## 5. Verification gates, in order

1. `cargo test` green in **both** debug and release. Debug matters: it has overflow checks, and
   this refactor is entirely about signed negation.
2. `scripts/verify_negamax_identity.py --baseline <pre> --candidate <post>` reports **28/28**, at
   depth 8 and again at depth 10. The corpus is the 14 shared positions **plus a colour-swapped
   copy of each**. The shared corpus is white-to-move throughout, which is precisely the blind
   spot a colour-symmetry refactor has: a sign error on the Black branch would leave all 14 shared
   positions identical.
3. The two `debug_assert`s of section 1 stay in the code, so the invariants are re-proven by every
   debug test run rather than by this document.
4. Release gate unchanged: `skills/engine_release_procedure.md` mandates a cross-version gauntlet
   for any change to `search_service.rs`, at 1s + 100ms and 100 games per pairing, as a smoke test.
   Node identity does not waive it — it is the control for the class of defect a self-A/B cannot
   see (`task.md` rule 2).

---

## 6. Result

| Gate | Result |
| :--- | :--- |
| `cargo test --release` | 176 passed, 0 failed |
| `cargo test` (debug, overflow checks on) | 176 passed, 0 failed |
| Compiler warnings, with and without `search-diag` | none |
| `verify_negamax_identity.py --depth 8` | **28/28 identical** |
| `verify_negamax_identity.py --depth 10` | **28/28 identical** |
| Cross-version gauntlet | **not run — required before any release** |

`minimax` lost its `white` parameter and 20 of its 22 colour branches; `singular_verification`
lost both of its own, including the reduced search and the bound test. The two that remain are
`Self::absolute_window` / `Self::relative_score`, which exist only because evaluation stays on
the absolute scale, and `Self::bound_for`, which is section 7.

---

## 7. The one site that could not be merged, and why

The first identity run failed on 19 of 28 positions. The scores and the principal variations
matched everywhere; only the tree size moved. Instrumenting both binaries with a counter on every
early return isolated it to a single counter — `store_upper`, 60 against 43 — and dumping every
Transposition Table store in walk order named the case exactly:

```
base (black node, ply 2, depth 1, eval=-34, orig_alpha=-34, orig_beta=-34) -> UpperBound
cand (black node, ply 2, depth 1, eval=-34, orig_alpha=-34, orig_beta=-34) -> LowerBound
```

**`alpha == beta` is reachable.** The root narrows `current_alpha`/`current_beta` towards each
other as it searches its move list, and the next root move can be handed an empty window. At such
a node both bound tests are true at once, and the order the two comparisons are written in decides
the label. Written on the absolute scale, that order broke the tie towards `UpperBound` at a White
node and towards `LowerBound` at a Black one; written on the relative scale it breaks the same way
for both, which is a different label at every Black node.

Restoring the colour-dependent order in `Self::bound_for` made all 28 positions identical, which
also confirms it was the **sole** source of divergence.

**The Black half of that tie-break is a defect.** A Black node whose running score is still at
`beta` has proved that Black cannot get *below* `beta` — the true score is at least `beta`, a
lower bound. Labelling it `UpperBound` publishes "the score is at most `beta`", which the search
never established, into a table that outlives the move. That is the same shape as the failure
`task.md` 8.1 records at roughly two hundred Elo, and the White half of the same tie-break is
correct, so the engine has been storing a sound bound for one colour and an unsound one for the
other.

It is **deliberately not fixed here**. Correcting it moves the search tree, which would forfeit
the identity gate that makes this refactor safe, and `task.md` rule 1 prices every search change
by matchplay. It is now a separate backlog item, and it is a cheap one: `Self::bound_for` is a
single function with the two orders side by side, and the alternative is one edit away.

---

## 8. A finding the harness turned up

Running the baseline against itself over the mirrored corpus shows that **the engine is not
colour-symmetric today**. A colour-swapped position is strategically identical to its original, so
a symmetric engine would search a near-identical tree and return the same score. It does not:

| Position | White to move | Colour-swapped | |
| :--- | ---: | ---: | :--- |
| Kiwipete | 121,779 nodes | 57,158 | 2.1x |
| Sharp French | 45,742 | 124,245 | 2.7x |
| Sharp Tactical | 12,941 | 35,542 | 2.7x |
| Middlegame | +34 cp | +68 cp | 34 cp apart |
| Rook Endgame | +165 cp | +136 cp | 29 cp apart |

The cause is not established. It is either the asymmetric search branches this refactor removes,
or an asymmetry in `eval_service.rs` / `move_gen_service.rs`. **The identity check separates the
two**: if the refactored build is node-identical on all 28 positions, the asymmetry cannot live in
the search's colour branches, and the remaining suspects are evaluation and move ordering. That is
worth an item of its own either way, and it is the one reason to doubt the `[Impact: Low]` label
this refactor carries in `task.md`.

The identity result now answers half of that question: the refactored build is node-identical on
all 28 positions, so **the asymmetry does not live in the search's colour branches**. The
remaining suspects are `eval_service.rs` and move ordering — and the empty-window bound defect of
section 7, which is colour-dependent by construction and is the one search-side candidate that
survives.
