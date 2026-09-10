//! Search diagnostics, off by default behind the `search-diag` Cargo feature.
//!
//! Two measurements live here. The first, and the reason the module exists, is the Stage-0
//! opportunity measurement for the staged `MovePicker`. The second is the size of the class of
//! moves that give check, for `task.md` section 11 — see [`record_searched_move`].
//!
//! The staged `MovePicker` of 1.2.2 reorders moves and therefore changes the search tree by
//! construction, which removes the node-identity gate that verified v0.31.0 and v0.32.0. Exactly
//! one part of it is provably order-preserving: the PV/TT move always sorts first, so searching
//! it before generating anything is a pure throughput change and stays node-identical.
//!
//! Whether that short-circuit is worth the correctness risk it carries — a Transposition Table
//! move played without ever being matched against a generated move list — depends entirely on how
//! many nodes actually cut on it. This module counts that, so the decision rests on a measurement
//! rather than on the literature's 85-90% figure for cutoffs on the first move.
//!
//! Everything here is behind the `search-diag` Cargo feature, which is off by default. In the
//! shipped build the recording functions have empty bodies and the call sites are `cfg`-gated, so
//! the search's codegen is untouched and the measurement cannot perturb what it measures.

/// Rank floor that identifies the PV/TT move, and nothing else, in a generated move list.
///
/// `get_valid_moves_from_move_list` gives the PV or TT move `is_pv_node_rank_bonus * 10000`
/// = 180,000 and then adds the ordinary MVV-LVA terms on top, so its worst case is a queen
/// capturing a pawn at 180,000 + 20,000 - 30,000 = 170,000. Every other move is bounded above by
/// capturing a queen (90,000) while giving check (`give_check_rank_bonus * 10000` = 50,000), i.e.
/// 140,000. The gap between 140,000 and 170,000 makes this floor an exact discriminator rather
/// than a heuristic one.
#[allow(dead_code)]
pub const RANK_STAGE0_FLOOR: i32 = crate::model::BAND_TT << crate::model::RANK_TIEBREAK_BITS;

/// What kind of move produced a cutoff, in the order a staged `MovePicker` would yield them.
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub enum MoveClass {
    /// The PV or Transposition Table move — Stage 0, servable without generating anything.
    PvOrTt = 0,
    /// A capture — Stage 1, needs capture generation only.
    Capture = 1,
    /// A quiet move that gives check. It carries `give_check_rank_bonus`, which is why it can
    /// outrank captures today, and it is the reason a lazy picker cannot keep the current order.
    QuietCheck = 2,
    /// A killer or counter move — Stage 2, servable by validating two or three remembered moves.
    KillerOrCounter = 3,
    /// An ordinary quiet move — Stage 3, needs full quiet generation.
    Quiet = 4,
}

#[cfg(feature = "search-diag")]
mod counters {
    use std::sync::atomic::{AtomicU64, Ordering};

    /// Nodes that reached the standard move loop with a non-empty move list.
    pub static INTERIOR_NODES: AtomicU64 = AtomicU64::new(0);
    /// Of those, nodes whose move list contained a PV/TT move.
    pub static STAGE0_AVAILABLE: AtomicU64 = AtomicU64::new(0);
    /// Nodes that produced a beta cutoff on the first searched move, whichever move that was.
    pub static FIRST_MOVE_CUTOFF: AtomicU64 = AtomicU64::new(0);
    /// Nodes that produced a beta cutoff on the first searched move *and* that move was the
    /// PV/TT move — the nodes a Stage-0 short-circuit would serve without generating anything.
    pub static STAGE0_CUTOFF: AtomicU64 = AtomicU64::new(0);
    /// Nodes where the search's own Transposition Table probe yielded a move at all. This is the
    /// ceiling on Stage-0 availability.
    pub static TT_MOVE_PRESENT: AtomicU64 = AtomicU64::new(0);
    /// Nodes where a Transposition Table move existed but no move in the list carried the PV/TT
    /// rank — the move was shadowed by a `pv_nodes` entry, or it was not legal here at all. The
    /// gap between this and zero is availability that a short-circuit could recover.
    pub static TT_MOVE_UNRANKED: AtomicU64 = AtomicU64::new(0);
    /// First-move cutoffs broken down by what kind of move actually cut, indexed by
    /// [`super::MoveClass`]. This sizes each stage of a `MovePicker`: a stage only pays for
    /// itself if cutoffs are waiting behind it.
    pub static CUTOFF_BY_CLASS: [AtomicU64; 5] = [
        AtomicU64::new(0),
        AtomicU64::new(0),
        AtomicU64::new(0),
        AtomicU64::new(0),
        AtomicU64::new(0),
    ];

    // ---------------------------------------------------------------------------------------
    // `task.md` section 11: how large is the class of moves that give check?
    //
    // A move that gives check is exempt from Late Move Reductions, Late Move Pruning, Futility
    // Pruning and the SEE pruning of bad captures, all at once. Nothing has ever measured what
    // that exemption costs. These counters size it directly: how many searched moves are in the
    // class, how much of the searched tree hangs under them, and how much of that tree only
    // exists because one of the four guards fired.
    // ---------------------------------------------------------------------------------------

    /// Moves that survived every pruning rule and were actually searched.
    pub static SEARCHED_MOVES: AtomicU64 = AtomicU64::new(0);
    /// Of those, moves that give check.
    pub static SEARCHED_CHECKS: AtomicU64 = AtomicU64::new(0);
    /// Of those, moves that give check and are not captures — the class section 11 is about.
    pub static SEARCHED_QUIET_CHECKS: AtomicU64 = AtomicU64::new(0);
    /// Searched moves whose own parent move gave check, i.e. moves made while in check. Unlike
    /// the subtree sums below this is a clean partition of the tree — every searched move is
    /// counted at most once — so it is the honest lower bound on how much of the search sits
    /// inside the exempt class.
    pub static MOVES_IN_CHECK: AtomicU64 = AtomicU64::new(0);

    /// Interior nodes searched below all searched moves, i.e. the size of the searched tree
    /// counted once per subtree. Used as the denominator for the three shares below; it is a
    /// deterministic proxy for time and, unlike a wall clock, cannot perturb the tree it counts.
    pub static SUBTREE_TOTAL: AtomicU64 = AtomicU64::new(0);
    /// Of that tree, the part below a move that gives check.
    pub static SUBTREE_CHECK: AtomicU64 = AtomicU64::new(0);

    /// Checking moves that met every Late Move Reduction condition except the `gives_check`
    /// guard, and whose damped reduction would have been positive. These were searched at full
    /// depth solely because of the guard.
    pub static LMR_BLOCKED: AtomicU64 = AtomicU64::new(0);
    /// The tree below them. This is what a reduction would have shrunk rather than deleted.
    pub static SUBTREE_LMR_BLOCKED: AtomicU64 = AtomicU64::new(0);
    /// Checking moves that met every Late Move Pruning condition except the `gives_check` guard.
    pub static LMP_BLOCKED: AtomicU64 = AtomicU64::new(0);
    /// The tree below them. This is what the rule would have deleted outright.
    pub static SUBTREE_LMP_BLOCKED: AtomicU64 = AtomicU64::new(0);
    /// Checking moves that met every Futility Pruning condition except the `gives_check` guard.
    pub static FP_BLOCKED: AtomicU64 = AtomicU64::new(0);
    /// The tree below them.
    pub static SUBTREE_FP_BLOCKED: AtomicU64 = AtomicU64::new(0);
    /// Checking captures that met every condition of the SEE pruning of bad captures except the
    /// `gives_check` guard. This is the fourth and last rule the exemption switches off.
    pub static SEE_BLOCKED: AtomicU64 = AtomicU64::new(0);
    /// The tree below them.
    pub static SUBTREE_SEE_BLOCKED: AtomicU64 = AtomicU64::new(0);

    // ---------------------------------------------------------------------------------------
    // `task.md` section 4: how often can a Singular Extension fire, and what does asking cost?
    //
    // The rule triggers on remaining depth, and this engine reaches a root depth of 9 to 10 at
    // the match time control. Whether a given `singular_min_depth` fires often enough to be worth
    // measuring in games — and how much of the tree the verification searches add — is exactly
    // what these counters answer. See `task.md` 10.5.
    // ---------------------------------------------------------------------------------------

    /// Nodes that reached `is_singular`, i.e. every cheap guard passed and the Transposition
    /// Table move was about to be searched.
    pub static SINGULAR_CANDIDATES: AtomicU64 = AtomicU64::new(0);
    /// Of those, nodes where the table entry supported the question and a verification search
    /// actually ran. The gap to the line above is candidates lost to a shallow or wrongly bounded
    /// entry.
    pub static SINGULAR_VERIFICATIONS: AtomicU64 = AtomicU64::new(0);
    /// Verification searches that concluded the Transposition Table move is singular, i.e.
    /// extensions actually granted.
    pub static SINGULAR_EXTENSIONS: AtomicU64 = AtomicU64::new(0);
    /// Interior nodes spent inside verification searches. This is the price of the rule, in the
    /// same unit as `SUBTREE_TOTAL`.
    pub static SINGULAR_VERIFY_NODES: AtomicU64 = AtomicU64::new(0);
    /// Verification searches by the remaining depth of the node that ran them, so one run reports
    /// what every candidate `singular_min_depth` would have cost.
    pub static SINGULAR_BY_DEPTH: [AtomicU64; 32] = [const { AtomicU64::new(0) }; 32];
    /// Extensions granted, by the same depth index.
    pub static SINGULAR_EXT_BY_DEPTH: [AtomicU64; 32] = [const { AtomicU64::new(0) }; 32];

    // ---------------------------------------------------------------------------------------
    // Null Move Pruning, for `task.md` 20.1 and 20.2.
    //
    // Rule 6: before pricing a depth-gated rule, check it fires at the depth of play. Both new
    // guards only ever *remove* null searches, so what decides whether they are worth a 6000-game
    // run is how many they remove and at what depth. These counters answer that from one
    // `search-diag` build, before the run is started.
    // ---------------------------------------------------------------------------------------

    /// Nodes that pass every guard Null Move Pruning carried *before* 20.1 and 20.2, i.e. the
    /// candidate population both new guards are subtracted from.
    pub static NMP_CANDIDATES: AtomicU64 = AtomicU64::new(0);
    /// Of those, nodes the `!is_pv` guard of 20.2 refuses.
    pub static NMP_PV_BLOCKED: AtomicU64 = AtomicU64::new(0);
    /// Of those, nodes the `static_eval >= beta` gate of 20.1 refuses. A node can be refused by
    /// both guards, so this and the line above overlap by construction.
    pub static NMP_GATE_BLOCKED: AtomicU64 = AtomicU64::new(0);
    /// Null searches that actually ran, i.e. candidates minus everything both guards refused.
    pub static NMP_SEARCHES: AtomicU64 = AtomicU64::new(0);
    /// Of those, the ones that produced a cutoff — after the verification search where one ran.
    pub static NMP_CUTS: AtomicU64 = AtomicU64::new(0);
    /// Candidates by remaining depth, so one run reports what the guards cost at the root depth
    /// the time control actually reaches rather than at the depth a test happens to use.
    pub static NMP_BY_DEPTH: [AtomicU64; 32] = [const { AtomicU64::new(0) }; 32];
    /// Nodes both guards let through, by the same depth index.
    pub static NMP_SEARCH_BY_DEPTH: [AtomicU64; 32] = [const { AtomicU64::new(0) }; 32];

    // ---------------------------------------------------------------------------------------
    // `task.md` 22: how much of the tree does Internal Iterative Reduction reach?
    //
    // Rule 6 again. The rule fires on nodes at `depth >= iir_min_depth` that have no
    // Transposition Table move, and nothing measured says how large that class is in an engine
    // whose killers, history and counter moves have persisted across the iterative deepening
    // loop since v0.42.0. The share `applied / eligible` is what decides whether the rule is
    // worth a six-hour run.
    // ---------------------------------------------------------------------------------------

    /// Nodes that reached the rule at or above `iir_min_depth`, i.e. its candidate population.
    pub static IIR_ELIGIBLE: AtomicU64 = AtomicU64::new(0);
    /// Of those, the nodes with no Transposition Table move — the ones actually reduced.
    pub static IIR_APPLIED: AtomicU64 = AtomicU64::new(0);
    /// Candidates by remaining depth, so the share is readable at the depth of play and not only
    /// in aggregate.
    pub static IIR_BY_DEPTH: [AtomicU64; 32] = [const { AtomicU64::new(0) }; 32];
    /// Reduced nodes, by the same depth index.
    pub static IIR_APPLIED_BY_DEPTH: [AtomicU64; 32] = [const { AtomicU64::new(0) }; 32];

    // `task.md` 23.3: what does the Late Move Reduction actually read?
    //
    // `lmr_history_good_threshold` and `lmr_history_bad_threshold` were calibrated against an
    // unsigned table that saturated at zero and was rescaled whenever an entry passed 9000. The
    // table is now signed, bounded by `model::MAX_HISTORY` and updated by gravity, so both
    // thresholds sit on a scale that no longer exists. These counters are what replaces guessing
    // at the new ones: they say how often each branch fires today and, through the histogram,
    // where the population actually sits.
    // ---------------------------------------------------------------------------------------

    /// Every quiet move that reached the Late Move Reduction and had its history read.
    pub static LMR_HIST_DECISIONS: AtomicU64 = AtomicU64::new(0);
    /// Of those, the ones above `lmr_history_good_threshold`, which are reduced one ply less.
    pub static LMR_HIST_GOOD: AtomicU64 = AtomicU64::new(0);
    /// Of those, the ones below `lmr_history_bad_threshold`, which are reduced one ply more.
    pub static LMR_HIST_BAD: AtomicU64 = AtomicU64::new(0);
    /// Entries reading exactly zero: the moves this search has never seen refuted or rewarded.
    /// Against the unsigned table this group and the refuted one were indistinguishable, which
    /// is the defect 23.3 repairs — the split between this counter and the next one is the
    /// measurement that says how much of the "bad" branch was landing on the wrong moves.
    pub static LMR_HIST_ZERO: AtomicU64 = AtomicU64::new(0);
    /// Entries below zero: the moves that were actually refuted.
    pub static LMR_HIST_NEGATIVE: AtomicU64 = AtomicU64::new(0);
    /// The distribution, by signed magnitude. See [`super::history_bucket`] for the mapping.
    pub static LMR_HIST_BUCKETS: [AtomicU64; 32] = [const { AtomicU64::new(0) }; 32];

    pub fn add(counter: &AtomicU64, value: u64) {
        counter.fetch_add(value, Ordering::Relaxed);
    }

    pub fn bump(counter: &AtomicU64) {
        counter.fetch_add(1, Ordering::Relaxed);
    }

    pub fn read(counter: &AtomicU64) -> u64 {
        counter.load(Ordering::Relaxed)
    }
}

/// Records one interior node.
///
/// `stage0_available` is whether the generated list actually carries a move at the PV/TT rank,
/// i.e. what a short-circuit could use today. `tt_move_present` is whether the search's own
/// Transposition Table probe produced a move at all, which is the ceiling that availability
/// would reach if nothing shadowed or discarded it.
#[inline(always)]
#[allow(unused_variables, dead_code)]
pub fn record_interior_node(stage0_available: bool, tt_move_present: bool) {
    #[cfg(feature = "search-diag")]
    {
        counters::bump(&counters::INTERIOR_NODES);
        if stage0_available {
            counters::bump(&counters::STAGE0_AVAILABLE);
        }
        if tt_move_present {
            counters::bump(&counters::TT_MOVE_PRESENT);
            if !stage0_available {
                counters::bump(&counters::TT_MOVE_UNRANKED);
            }
        }
    }
}

/// Records a beta cutoff. `turn_counter` is the 1-based index of the move that caused it,
/// `first_searched_rank` the rank of the move searched first at this node and `first_class`
/// which stage of a `MovePicker` would have had to produce it.
#[inline(always)]
#[allow(unused_variables, dead_code)]
pub fn record_cutoff(turn_counter: i32, first_searched_rank: i32, first_class: MoveClass) {
    #[cfg(feature = "search-diag")]
    {
        if turn_counter == 1 {
            counters::bump(&counters::FIRST_MOVE_CUTOFF);
            if first_searched_rank >= RANK_STAGE0_FLOOR {
                counters::bump(&counters::STAGE0_CUTOFF);
            }
            counters::bump(&counters::CUTOFF_BY_CLASS[first_class as usize]);
        }
    }
}

/// Records one move that was actually searched, for `task.md` section 11.
///
/// `subtree_nodes` is the number of interior nodes the search visited below this move, taken as
/// the delta of `Stats::calculated_nodes` across `do_move`/`undo_move`. It is used instead of a
/// wall clock because it is deterministic and free: a timer around every move would perturb the
/// tree it is measuring, and the node-identity gate in `scripts/measure_stage0.py` would then be
/// unable to certify the measurement.
///
/// The three `*_blocked` flags mean "this move met every condition of that rule except the
/// `gives_check` guard". They are the actionable quantity: the tree below a blocked move exists
/// only because a move that gives check is currently exempt from every reduction and every
/// pruning rule in the engine.
#[inline(always)]
#[allow(unused_variables, dead_code)]
pub fn record_searched_move(
    gives_check: bool,
    is_capture: bool,
    parent_gives_check: bool,
    lmr_blocked: bool,
    lmp_blocked: bool,
    fp_blocked: bool,
    see_blocked: bool,
    subtree_nodes: u64,
) {
    #[cfg(feature = "search-diag")]
    {
        counters::bump(&counters::SEARCHED_MOVES);
        counters::add(&counters::SUBTREE_TOTAL, subtree_nodes);
        if parent_gives_check {
            counters::bump(&counters::MOVES_IN_CHECK);
        }
        if gives_check {
            counters::bump(&counters::SEARCHED_CHECKS);
            counters::add(&counters::SUBTREE_CHECK, subtree_nodes);
            if !is_capture {
                counters::bump(&counters::SEARCHED_QUIET_CHECKS);
            }
        }
        if lmr_blocked {
            counters::bump(&counters::LMR_BLOCKED);
            counters::add(&counters::SUBTREE_LMR_BLOCKED, subtree_nodes);
        }
        if lmp_blocked {
            counters::bump(&counters::LMP_BLOCKED);
            counters::add(&counters::SUBTREE_LMP_BLOCKED, subtree_nodes);
        }
        if fp_blocked {
            counters::bump(&counters::FP_BLOCKED);
            counters::add(&counters::SUBTREE_FP_BLOCKED, subtree_nodes);
        }
        if see_blocked {
            counters::bump(&counters::SEE_BLOCKED);
            counters::add(&counters::SUBTREE_SEE_BLOCKED, subtree_nodes);
        }
    }
}

/// Records one node that reached the Singular Extension verification, for `task.md` section 4.
///
/// `verified` separates a node whose Transposition Table entry could support the question from
/// one where it could not; `extended` is the outcome. `verify_nodes` is the interior tree the
/// verification search walked, taken as a `Stats::calculated_nodes` delta, so the cost of the
/// rule is reported in the same unit as the tree it is added to.
#[inline(always)]
#[allow(unused_variables, dead_code)]
pub fn record_singular(depth: i32, verified: bool, extended: bool, verify_nodes: u64) {
    #[cfg(feature = "search-diag")]
    {
        counters::bump(&counters::SINGULAR_CANDIDATES);
        if !verified {
            return;
        }
        let bucket = (depth.max(0) as usize).min(31);
        counters::bump(&counters::SINGULAR_VERIFICATIONS);
        counters::bump(&counters::SINGULAR_BY_DEPTH[bucket]);
        counters::add(&counters::SINGULAR_VERIFY_NODES, verify_nodes);
        if extended {
            counters::bump(&counters::SINGULAR_EXTENSIONS);
            counters::bump(&counters::SINGULAR_EXT_BY_DEPTH[bucket]);
        }
    }
}

/// Records one node that reached Null Move Pruning's pre-20.1 guards, for `task.md` 20.1 and 20.2.
///
/// `pv_blocked` and `gate_blocked` are the two new guards' verdicts on this node, taken
/// independently, so a node refused by both is counted in both — the run reports what each guard
/// is worth on its own, which is the question the tree census asks in four configurations.
#[inline(always)]
#[allow(unused_variables, dead_code)]
pub fn record_nmp_candidate(depth: i32, pv_blocked: bool, gate_blocked: bool) {
    #[cfg(feature = "search-diag")]
    {
        let bucket = (depth.max(0) as usize).min(31);
        counters::bump(&counters::NMP_CANDIDATES);
        counters::bump(&counters::NMP_BY_DEPTH[bucket]);
        if pv_blocked {
            counters::bump(&counters::NMP_PV_BLOCKED);
        }
        if gate_blocked {
            counters::bump(&counters::NMP_GATE_BLOCKED);
        }
        if !pv_blocked && !gate_blocked {
            counters::bump(&counters::NMP_SEARCHES);
            counters::bump(&counters::NMP_SEARCH_BY_DEPTH[bucket]);
        }
    }
}

/// Records one node that reached Internal Iterative Reduction's depth gate, for `task.md` 22.
///
/// `applied` is the rule's verdict: true where the Transposition Table probe yielded no move and
/// the node is therefore searched one ply shallower. The call site carries the same guards as the
/// rule apart from `tt_move`, so `applied / eligible` is the share of the candidate population
/// the rule actually reduces.
#[inline(always)]
#[allow(unused_variables, dead_code)]
pub fn record_iir(depth: i32, applied: bool) {
    #[cfg(feature = "search-diag")]
    {
        let bucket = (depth.max(0) as usize).min(31);
        counters::bump(&counters::IIR_ELIGIBLE);
        counters::bump(&counters::IIR_BY_DEPTH[bucket]);
        if applied {
            counters::bump(&counters::IIR_APPLIED);
            counters::bump(&counters::IIR_APPLIED_BY_DEPTH[bucket]);
        }
    }
}

/// The histogram slot a history entry falls in, by sign and by binary magnitude.
///
/// Slot 16 is exactly zero. Slots 17 to 31 are the positive entries, one per power of two, so
/// slot `17 + k` holds the magnitudes in `[2^k, 2^(k+1))` and slot 31 holds everything from
/// 2^14 = 16,384 up, which is `model::MAX_HISTORY` and therefore its own edge. Slots 15 down to
/// 1 mirror that for the negative entries. Slot 0 is never used.
#[allow(dead_code)]
pub fn history_bucket(entry: i32) -> usize {
    if entry == 0 {
        return 16;
    }
    let decade = ((31 - entry.unsigned_abs().leading_zeros()) as usize).min(14);
    if entry > 0 { 17 + decade } else { 15 - decade }
}

/// Records one Late Move Reduction history read, `task.md` 23.3.
///
/// The thresholds are passed in rather than read from a `Config` here, so the comparison lives in
/// exactly one place and cannot drift from the one `lmr_reduction` makes. Called from the rule's
/// own site only: the diagnostic call in the checking-move census above runs on a population the
/// rule excludes, and counting it would put moves in the sample that never reach the decision.
#[inline(always)]
#[allow(unused_variables, dead_code)]
pub fn record_lmr_history(hist_val: i32, good_threshold: i32, bad_threshold: i32) {
    #[cfg(feature = "search-diag")]
    {
        counters::bump(&counters::LMR_HIST_DECISIONS);
        // The same `else if` the rule uses: a move above the good threshold never reaches the
        // bad one, which matters at any calibration where the two could overlap.
        if hist_val > good_threshold {
            counters::bump(&counters::LMR_HIST_GOOD);
        } else if hist_val < bad_threshold {
            counters::bump(&counters::LMR_HIST_BAD);
        }
        if hist_val == 0 {
            counters::bump(&counters::LMR_HIST_ZERO);
        }
        if hist_val < 0 {
            counters::bump(&counters::LMR_HIST_NEGATIVE);
        }
        counters::bump(&counters::LMR_HIST_BUCKETS[history_bucket(hist_val)]);
    }
}

/// Records a Null Move Pruning cutoff, taken at the point the rule returns `beta`, i.e. after the
/// verification search at the depths that run one.
#[inline(always)]
#[allow(dead_code)]
pub fn record_nmp_cut() {
    #[cfg(feature = "search-diag")]
    counters::bump(&counters::NMP_CUTS);
}

/// Writes the size of the tree the search actually walked.
///
/// The UCI `nodes` field reports `Stats::created_nodes`, i.e. the number of *generated* moves.
/// Stage 0 of `task.md` 1.2.2 skips generation entirely at a cutoff, so that field legitimately
/// falls while the engine gets faster and is therefore useless as an identity criterion. The two
/// counters here are the searched tree: interior moves actually played, and Quiescence entries.
#[allow(unused_variables, dead_code)]
pub fn dump_tree(calculated_nodes: usize, eval_nodes: usize) {
    #[cfg(feature = "search-diag")]
    eprintln!("SEARCHTREE calculated={} eval={}", calculated_nodes, eval_nodes);
}

/// Writes the cumulative counters to stderr. Called at the end of every search, so the final
/// line before `bestmove` carries the totals for the whole run.
pub fn dump() {
    #[cfg(feature = "search-diag")]
    {
        use std::sync::atomic::AtomicU64;

        let interior = counters::read(&counters::INTERIOR_NODES);
        if interior == 0 {
            return;
        }
        let available = counters::read(&counters::STAGE0_AVAILABLE);
        let first_cut = counters::read(&counters::FIRST_MOVE_CUTOFF);
        let stage0_cut = counters::read(&counters::STAGE0_CUTOFF);
        let tt_present = counters::read(&counters::TT_MOVE_PRESENT);
        let tt_unranked = counters::read(&counters::TT_MOVE_UNRANKED);
        let pct = |value: u64| (value as f64) * 100.0 / (interior as f64);

        eprintln!(
            "SEARCHDIAG interior={} available={} ({:.1}%) first_cut={} ({:.1}%) \
             stage0_cut={} ({:.1}%) wasted_validation={} ({:.1}%) \
             tt_present={} ({:.1}%) tt_unranked={} ({:.1}%)",
            interior,
            available,
            pct(available),
            first_cut,
            pct(first_cut),
            stage0_cut,
            pct(stage0_cut),
            available.saturating_sub(stage0_cut),
            pct(available.saturating_sub(stage0_cut)),
            tt_present,
            pct(tt_present),
            tt_unranked,
            pct(tt_unranked),
        );
        let by_class: Vec<u64> = counters::CUTOFF_BY_CLASS.iter().map(counters::read).collect();
        eprintln!(
            "SEARCHDIAGCLASS pv_tt={} capture={} quiet_check={} killer_counter={} quiet={}",
            by_class[0], by_class[1], by_class[2], by_class[3], by_class[4],
        );

        eprintln!(
            "SEARCHDIAGCHECK searched={} checks={} quiet_checks={} in_check={} subtree_total={} \
             subtree_check={} lmr_blocked={} subtree_lmr={} lmp_blocked={} subtree_lmp={} \
             fp_blocked={} subtree_fp={} see_blocked={} subtree_see={}",
            counters::read(&counters::SEARCHED_MOVES),
            counters::read(&counters::SEARCHED_CHECKS),
            counters::read(&counters::SEARCHED_QUIET_CHECKS),
            counters::read(&counters::MOVES_IN_CHECK),
            counters::read(&counters::SUBTREE_TOTAL),
            counters::read(&counters::SUBTREE_CHECK),
            counters::read(&counters::LMR_BLOCKED),
            counters::read(&counters::SUBTREE_LMR_BLOCKED),
            counters::read(&counters::LMP_BLOCKED),
            counters::read(&counters::SUBTREE_LMP_BLOCKED),
            counters::read(&counters::FP_BLOCKED),
            counters::read(&counters::SUBTREE_FP_BLOCKED),
            counters::read(&counters::SEE_BLOCKED),
            counters::read(&counters::SUBTREE_SEE_BLOCKED),
        );

        let by_depth = |table: &[AtomicU64; 32]| -> String {
            table
                .iter()
                .enumerate()
                .map(|(depth, slot)| (depth, counters::read(slot)))
                .filter(|(_, count)| *count > 0)
                .map(|(depth, count)| format!("{}:{}", depth, count))
                .collect::<Vec<_>>()
                .join(",")
        };
        eprintln!(
            "SEARCHDIAGSINGULAR candidates={} verifications={} extensions={} verify_nodes={} \
             by_depth={} ext_by_depth={}",
            counters::read(&counters::SINGULAR_CANDIDATES),
            counters::read(&counters::SINGULAR_VERIFICATIONS),
            counters::read(&counters::SINGULAR_EXTENSIONS),
            counters::read(&counters::SINGULAR_VERIFY_NODES),
            by_depth(&counters::SINGULAR_BY_DEPTH),
            by_depth(&counters::SINGULAR_EXT_BY_DEPTH),
        );
        eprintln!(
            "SEARCHDIAGNMP candidates={} pv_blocked={} gate_blocked={} searches={} cuts={} \
             by_depth={} search_by_depth={}",
            counters::read(&counters::NMP_CANDIDATES),
            counters::read(&counters::NMP_PV_BLOCKED),
            counters::read(&counters::NMP_GATE_BLOCKED),
            counters::read(&counters::NMP_SEARCHES),
            counters::read(&counters::NMP_CUTS),
            by_depth(&counters::NMP_BY_DEPTH),
            by_depth(&counters::NMP_SEARCH_BY_DEPTH),
        );
        let buckets = |table: &[AtomicU64; 32]| -> String {
            table
                .iter()
                .enumerate()
                .map(|(slot, count)| (slot, counters::read(count)))
                .filter(|(_, count)| *count > 0)
                .map(|(slot, count)| {
                    let label = match slot {
                        16 => "0".to_string(),
                        s if s > 16 => format!("+2^{}", s - 17),
                        s => format!("-2^{}", 15 - s),
                    };
                    format!("{}:{}", label, count)
                })
                .collect::<Vec<_>>()
                .join(",")
        };
        eprintln!(
            "SEARCHDIAGHIST decisions={} good={} bad={} zero={} negative={} buckets={}",
            counters::read(&counters::LMR_HIST_DECISIONS),
            counters::read(&counters::LMR_HIST_GOOD),
            counters::read(&counters::LMR_HIST_BAD),
            counters::read(&counters::LMR_HIST_ZERO),
            counters::read(&counters::LMR_HIST_NEGATIVE),
            buckets(&counters::LMR_HIST_BUCKETS),
        );
        eprintln!(
            "SEARCHDIAGIIR eligible={} applied={} by_depth={} applied_by_depth={}",
            counters::read(&counters::IIR_ELIGIBLE),
            counters::read(&counters::IIR_APPLIED),
            by_depth(&counters::IIR_BY_DEPTH),
            by_depth(&counters::IIR_APPLIED_BY_DEPTH),
        );
    }
}
