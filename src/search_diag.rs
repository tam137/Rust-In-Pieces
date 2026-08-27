//! Stage-0 opportunity measurement for `task.md` specification 1.2.2.
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
pub const RANK_STAGE0_FLOOR: i32 = 150_000;

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
    }
}
