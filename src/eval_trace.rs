//! The evaluation trace of the Texel tuner (`task.md` 29.5 E2).
//!
//! Every term of the hand-crafted evaluation reports the parameter it read and how far one unit of
//! that parameter moves the middlegame and the endgame sum of its unit (a unit is one call of
//! `calculate_weighted_eval`, or a value added to the evaluation untapered). Constants that are not
//! parameters report themselves as constants. The tuner rebuilds the evaluation from these
//! coefficients for any parameter vector, which is what makes a gradient over hundreds of weights
//! affordable.
//!
//! Everything here is compiled only with the `texel` feature. Without it the macros expand to
//! empty blocks, `tracing()` is the constant `false`, and the evaluation is the shipped one.

/// A term: `tr!(Param, c_mg, c_eg)` for a scalar of [`P`], read through `Config`.
macro_rules! tr {
    ($param:ident, $c_mg:expr, $c_eg:expr) => {{
        #[cfg(feature = "texel")]
        {
            if crate::eval_trace::active() {
                crate::eval_trace::term(crate::eval_trace::P::$param as usize, ($c_mg) as f64, ($c_eg) as f64);
            }
        }
    }};
}

/// A term whose parameter index is computed: `tr_param!(index_expression, c_mg, c_eg)`.
macro_rules! tr_param {
    ($index:expr, $c_mg:expr, $c_eg:expr) => {{
        #[cfg(feature = "texel")]
        {
            if crate::eval_trace::active() {
                crate::eval_trace::term($index, ($c_mg) as f64, ($c_eg) as f64);
            }
        }
    }};
}

/// A constant of the current unit that no parameter scales.
macro_rules! tr_const {
    ($k_mg:expr, $k_eg:expr) => {{
        #[cfg(feature = "texel")]
        {
            if crate::eval_trace::active() {
                crate::eval_trace::constant(($k_mg) as f64, ($k_eg) as f64);
            }
        }
    }};
}

/// Closes the current unit with the sums it actually produced and their tapered value.
macro_rules! tr_unit {
    ($o:expr, $e:expr, $tapered:expr) => {{
        #[cfg(feature = "texel")]
        {
            if crate::eval_trace::active() {
                crate::eval_trace::unit_end(($o) as f64, ($e) as f64, ($tapered) as i32);
            }
        }
    }};
}

/// Closes the current unit for a value added to the evaluation untapered.
macro_rules! tr_direct {
    ($value:expr) => {{
        #[cfg(feature = "texel")]
        {
            if crate::eval_trace::active() {
                crate::eval_trace::unit_end(($value) as f64, ($value) as f64, ($value) as i32);
            }
        }
    }};
}

/// One feature of a king's safety: `tr_safety!(king, mg_index, eg_index, value)`, `king` 0 for
/// White's king, 1 for Black's. The unit it belongs to adds `-max(u_mg, 0)^2 / 720` (mg) and
/// `-max(u_eg, 0) / 20` (eg) for that king, with `u` the weighted sum of its features
/// ([`SAFETY_MG_DIVISOR`], [`SAFETY_EG_DIVISOR`]).
macro_rules! tr_safety {
    ($king:expr, $mg:expr, $eg:expr, $value:expr) => {{
        #[cfg(feature = "texel")]
        {
            if crate::eval_trace::active() {
                crate::eval_trace::safety($king, $mg, $eg, ($value) as f64);
            }
        }
    }};
}

/// The material and piece-square terms of every piece on the board, ahead of their unit.
macro_rules! tr_pst {
    ($board:expr) => {{
        #[cfg(feature = "texel")]
        {
            if crate::eval_trace::active() {
                crate::eval_trace::pieces($board);
            }
        }
    }};
}

/// The evaluation as it enters the positional cap, and as it leaves the function.
macro_rules! tr_mark {
    ($which:ident, $value:expr) => {{
        #[cfg(feature = "texel")]
        {
            if crate::eval_trace::active() {
                crate::eval_trace::$which(($value) as i32);
            }
        }
    }};
}

pub(crate) use {tr, tr_const, tr_direct, tr_mark, tr_param, tr_pst, tr_safety, tr_unit};

/// The divisors of the quadratic middlegame and the linear endgame king-safety penalty.
pub const SAFETY_MG_DIVISOR: i32 = 720;
pub const SAFETY_EG_DIVISOR: i32 = 20;

/// The `(mg, eg)` penalty of one king for its safety sums `u_mg`, `u_eg`, as the engine and the
/// model compute it (the engine on integers, the model on reals).
#[cfg(feature = "texel")]
#[inline(always)]
pub fn safety_penalty(u_mg: f64, u_eg: f64) -> (f64, f64) {
    let (m, e) = (u_mg.max(0.0), u_eg.max(0.0));
    (m * m / SAFETY_MG_DIVISOR as f64, e / SAFETY_EG_DIVISOR as f64)
}

/// Whether a trace is being recorded; the constant `false` without the `texel` feature, so the
/// evaluation's one branch on it disappears from the shipped build.
#[inline(always)]
pub fn tracing() -> bool {
    #[cfg(feature = "texel")]
    {
        active()
    }
    #[cfg(not(feature = "texel"))]
    {
        false
    }
}

#[cfg(feature = "texel")]
pub use imp::*;

#[cfg(feature = "texel")]
mod imp {
    use crate::config::Config;
    use crate::model::Board;
    use std::cell::{Cell, RefCell};

    macro_rules! scalar_params {
        ($($id:ident => $field:ident),* $(,)?) => {
            /// The scalar evaluation weights of `Config` the tuner fits, in a fixed order.
            #[derive(Clone, Copy, Debug, PartialEq, Eq)]
            #[repr(usize)]
            pub enum P { $($id),* }

            /// `Config` field name, getter and setter of every [`P`], in the same order.
            pub const SCALARS: &[(&str, fn(&Config) -> i16, fn(&mut Config, i16))] = &[
                $( (stringify!($field), |c: &Config| c.$field, |c: &mut Config, v: i16| c.$field = v) ),*
            ];
        };
    }

    scalar_params! {
        YourTurnBonus => your_turn_bonus,
        UndevelopedKnightMalus => undeveloped_knight_malus,
        UndevelopedBishopMalus => undeveloped_bishop_malus,
        UndevelopedKingMalus => undeveloped_king_malus,
        RookOpenFile => rook_open_file,
        RookHalfOpenFile => rook_half_open_file,
        BishopPairBonus => bishop_pair_bonus,
        RookDoubledBonus => rook_doubled_bonus,
        RookBehindPassedPawnMiddlegame => rook_behind_passed_pawn_middlegame,
        RookBehindPassedPawnEndgame => rook_behind_passed_pawn_endgame,
        ProtectedPassedPawnMiddlegame => protected_passed_pawn_middlegame,
        ProtectedPassedPawnEndgame => protected_passed_pawn_endgame,
        KingOppositionBonus => king_opposition_bonus,
        KingOpenFileMalus => king_open_file_malus,
        KingHalfOpenFileMalus => king_half_open_file_malus,
        PawnStructure => pawn_structure,
        PawnSupportsKnightOutpost => pawn_supports_knight_outpost,
        PawnCentered => pawn_centered,
        PawnUndevelopedMalus => pawn_undeveloped_malus,
        PawnOnLastRankBonus => pawn_on_last_rank_bonus,
        PawnOnBeforeLastRankBonus => pawn_on_before_last_rank_bonus,
        PawnOnBeforeBeforeLastRankBonus => pawn_on_before_before_last_rank_bonus,
        PawnDefendsBishop => pawn_defends_bishop,
        PawnDoubleMalus => pawn_double_malus,
        KnightOnRimMalus => knight_on_rim_malus,
        KnightCentered => knight_centered,
        KnightBlockesPawn => knight_blockes_pawn,
        BishopTrappedAtRimMalus => bishop_trapped_at_rim_malus,
        PawnAttacksOpponentFig => pawn_attacks_opponent_fig,
        PawnAttacksOpponentFigWithTempo => pawn_attacks_opponent_fig_with_tempo,
        QueenInAttack => queen_in_attack,
        QueenInAttackWithTempo => queen_in_attack_with_tempo,
        KnightAttacksBishop => knight_attacks_bishop,
        KnightAttacksRook => knight_attacks_rook,
        KnightAttacksBishopTempo => knight_attacks_bishop_tempo,
        KnightAttacksRookTempo => knight_attacks_rook_tempo,
        ThreatMinorAttacksRook => threat_minor_attacks_rook,
        ThreatMinorAttacksQueen => threat_minor_attacks_queen,
        ThreatRookAttacksQueen => threat_rook_attacks_queen,
        KingPawnShield => king_pawn_shield,
        KingPieceShield => king_piece_shield,
        KingPawnShieldKingside => king_pawn_shield_kingside,
        KingPawnShieldQueenside => king_pawn_shield_queenside,
        KingPieceShieldKingside => king_piece_shield_kingside,
        KingPieceShieldQueenside => king_piece_shield_queenside,
        ConnectedPassedPawnMg => connected_passed_pawn_mg,
        ConnectedPassedPawnEg => connected_passed_pawn_eg,
        KnightOutpostTrueMg => knight_outpost_true_mg,
        KnightOutpostTrueEg => knight_outpost_true_eg,
        BishopOutpostTrueMg => bishop_outpost_true_mg,
        BishopOutpostTrueEg => bishop_outpost_true_eg,
        RookBehindEnemyPassedPawnMg => rook_behind_enemy_passed_pawn_mg,
        RookBehindEnemyPassedPawnEg => rook_behind_enemy_passed_pawn_eg,
        KingTrappAtBaselineMalus => king_trapp_at_baseline_malus,
        RookOnSeventh => rook_on_seventh,
        KingDangerWeight1 => king_danger_weight_1,
        KingDangerWeight2 => king_danger_weight_2,
        KingDangerWeight3 => king_danger_weight_3,
        KingDangerWeight4 => king_danger_weight_4,
        KingDangerWeight5 => king_danger_weight_5,
        PawnIsolatedMalus => pawn_isolated_malus,
        PawnBackwardMalus => pawn_backward_malus,
        KnightMobilityFactor => knight_mobility_factor,
        BishopMobilityFactor => bishop_mobility_factor,
        RookMobilityFactor => rook_mobility_factor,
        QueenMobilityFactor => queen_mobility_factor,
        KingPasserDistWeight => king_passer_dist_weight,
        KingOpenFileHeavyThreatMalus => king_open_file_heavy_threat_malus,
        RookOpenFileAttacksKing => rook_open_file_attacks_king,
        RookOpenFileAttacksQueen => rook_open_file_attacks_queen,
        PawnPhalanxMg => pawn_phalanx_mg,
        PawnPhalanxEg => pawn_phalanx_eg,
        BishopDiagonalAttacksKing => bishop_diagonal_attacks_king,
        BishopDiagonalAttacksQueen => bishop_diagonal_attacks_queen,
        RookOnSeventhKingCutoff => rook_on_seventh_king_cutoff,
        RooksDoubledOnSeventh => rooks_doubled_on_seventh,
        PassedPawnBlockadedMalus => passed_pawn_blockaded_malus,
        CandidatePassedPawnBonus => candidate_passed_pawn_bonus,
        PawnStormBonus => pawn_storm_bonus,
        KingSafetyKnightAttackerMg => king_safety_knight_attacker_mg,
        KingSafetyKnightAttackerEg => king_safety_knight_attacker_eg,
        KingSafetyBishopAttackerMg => king_safety_bishop_attacker_mg,
        KingSafetyBishopAttackerEg => king_safety_bishop_attacker_eg,
        KingSafetyRookAttackerMg => king_safety_rook_attacker_mg,
        KingSafetyRookAttackerEg => king_safety_rook_attacker_eg,
        KingSafetyQueenAttackerMg => king_safety_queen_attacker_mg,
        KingSafetyQueenAttackerEg => king_safety_queen_attacker_eg,
        KingSafetyRingAttacksMg => king_safety_ring_attacks_mg,
        KingSafetyRingAttacksEg => king_safety_ring_attacks_eg,
        KingSafetyWeakSquaresMg => king_safety_weak_squares_mg,
        KingSafetyWeakSquaresEg => king_safety_weak_squares_eg,
        KingSafetyNoQueenMg => king_safety_no_queen_mg,
        KingSafetyNoQueenEg => king_safety_no_queen_eg,
        KingSafetySafeKnightCheckMg => king_safety_safe_knight_check_mg,
        KingSafetySafeKnightCheckEg => king_safety_safe_knight_check_eg,
        KingSafetySafeBishopCheckMg => king_safety_safe_bishop_check_mg,
        KingSafetySafeBishopCheckEg => king_safety_safe_bishop_check_eg,
        KingSafetySafeRookCheckMg => king_safety_safe_rook_check_mg,
        KingSafetySafeRookCheckEg => king_safety_safe_rook_check_eg,
        KingSafetySafeQueenCheckMg => king_safety_safe_queen_check_mg,
        KingSafetySafeQueenCheckEg => king_safety_safe_queen_check_eg,
        KingSafetyAdjustmentMg => king_safety_adjustment_mg,
        KingSafetyAdjustmentEg => king_safety_adjustment_eg,
        ThreatHangingMg => threat_hanging_mg,
        ThreatHangingEg => threat_hanging_eg,
        ThreatLesserOnMinorMg => threat_lesser_on_minor_mg,
        ThreatLesserOnMinorEg => threat_lesser_on_minor_eg,
        ThreatLesserOnRookMg => threat_lesser_on_rook_mg,
        ThreatLesserOnRookEg => threat_lesser_on_rook_eg,
        ThreatLesserOnQueenMg => threat_lesser_on_queen_mg,
        ThreatLesserOnQueenEg => threat_lesser_on_queen_eg,
        ThreatSafePawnPushMg => threat_safe_pawn_push_mg,
        ThreatSafePawnPushEg => threat_safe_pawn_push_eg,
        // The endgame scales of `task.md` 31.3: not summed into the units but applied by the
        // pipeline to the finished evaluation, so no trace term carries them.
        EndgameScaleDrawish => endgame_scale_drawish,
        EndgameScalePawnEnding => endgame_scale_pawn_ending,
    }

    /// Layout of the parameter vector: scalars, the passed-pawn table (mg, eg), material (mg, eg),
    /// piece-square tables (mg, eg; piece * 64 + square from White's side, rank 8 first).
    pub const N_SCALARS: usize = SCALARS.len();
    pub const PASSER_MG: usize = N_SCALARS;
    pub const PASSER_EG: usize = PASSER_MG + 8;
    pub const MATERIAL_MG: usize = PASSER_EG + 8;
    pub const MATERIAL_EG: usize = MATERIAL_MG + 6;
    pub const TABLE_MG: usize = MATERIAL_EG + 6;
    pub const TABLE_EG: usize = TABLE_MG + 6 * 64;
    pub const N_PARAMS: usize = TABLE_EG + 6 * 64;

    pub fn passer_mg(relative_rank: usize) -> usize { PASSER_MG + relative_rank }
    pub fn passer_eg(relative_rank: usize) -> usize { PASSER_EG + relative_rank }

    /// The `(mg, eg)` weights of the king-safety features, in the order `EvalService::king_safety`
    /// computes them.
    pub const KING_SAFETY: [(P, P); 12] = [
        (P::KingSafetyKnightAttackerMg, P::KingSafetyKnightAttackerEg),
        (P::KingSafetyBishopAttackerMg, P::KingSafetyBishopAttackerEg),
        (P::KingSafetyRookAttackerMg, P::KingSafetyRookAttackerEg),
        (P::KingSafetyQueenAttackerMg, P::KingSafetyQueenAttackerEg),
        (P::KingSafetyRingAttacksMg, P::KingSafetyRingAttacksEg),
        (P::KingSafetyWeakSquaresMg, P::KingSafetyWeakSquaresEg),
        (P::KingSafetyNoQueenMg, P::KingSafetyNoQueenEg),
        (P::KingSafetySafeKnightCheckMg, P::KingSafetySafeKnightCheckEg),
        (P::KingSafetySafeBishopCheckMg, P::KingSafetySafeBishopCheckEg),
        (P::KingSafetySafeRookCheckMg, P::KingSafetySafeRookCheckEg),
        (P::KingSafetySafeQueenCheckMg, P::KingSafetySafeQueenCheckEg),
        (P::KingSafetyAdjustmentMg, P::KingSafetyAdjustmentEg),
    ];

    /// The `(mg, eg)` weights of the threats on the attack maps, in the order
    /// `EvalService::threats` computes them.
    pub const THREATS: [(P, P); 5] = [
        (P::ThreatHangingMg, P::ThreatHangingEg),
        (P::ThreatLesserOnMinorMg, P::ThreatLesserOnMinorEg),
        (P::ThreatLesserOnRookMg, P::ThreatLesserOnRookEg),
        (P::ThreatLesserOnQueenMg, P::ThreatLesserOnQueenEg),
        (P::ThreatSafePawnPushMg, P::ThreatSafePawnPushEg),
    ];

    /// The king-danger weight a given effective attacker count reads; index 0 is the constant 0.
    pub fn king_danger(index: usize) -> usize {
        [P::KingDangerWeight1, P::KingDangerWeight2, P::KingDangerWeight3, P::KingDangerWeight4,
            P::KingDangerWeight5][index - 1] as usize
    }

    /// The pawn and piece shield weights the king's file reads, as `white_king` chooses them.
    pub fn king_shield(file: i32) -> (usize, usize) {
        if file <= 2 {
            (P::KingPawnShieldQueenside as usize, P::KingPieceShieldQueenside as usize)
        } else if file >= 5 {
            (P::KingPawnShieldKingside as usize, P::KingPieceShieldKingside as usize)
        } else {
            (P::KingPawnShield as usize, P::KingPieceShield as usize)
        }
    }

    /// The shipped parameter vector: `config` for the scalars and the passed-pawn table, `pst.rs`
    /// for material and tables.
    pub fn theta_of(config: &Config) -> Vec<f64> {
        let mut theta = vec![0.0; N_PARAMS];
        for (i, (_, get, _)) in SCALARS.iter().enumerate() {
            theta[i] = get(config) as f64;
        }
        for r in 0..8 {
            theta[PASSER_MG + r] = config.passed_pawn_bonus_mg[r] as f64;
            theta[PASSER_EG + r] = config.passed_pawn_bonus_eg[r] as f64;
        }
        for p in 0..6 {
            theta[MATERIAL_MG + p] = crate::pst::MATERIAL_MG[p] as f64;
            theta[MATERIAL_EG + p] = crate::pst::MATERIAL_EG[p] as f64;
            for sq in 0..64 {
                theta[TABLE_MG + p * 64 + sq] = crate::pst::TABLE_MG[p][sq] as f64;
                theta[TABLE_EG + p * 64 + sq] = crate::pst::TABLE_EG[p][sq] as f64;
            }
        }
        theta
    }

    /// One closed unit, kept for `texel-check`: what the coefficients predict at the shipped
    /// parameters against what the evaluation produced.
    #[derive(Clone, Debug)]
    pub struct UnitCheck {
        pub predicted: (f64, f64),
        pub actual: (f64, f64),
    }

    /// The trace of one evaluation.
    #[derive(Clone, Debug, Default)]
    pub struct Trace {
        /// Merged coefficients of the scalar and passed-pawn parameters, `(index, c_mg, c_eg)`.
        pub terms: Vec<(usize, f64, f64)>,
        /// The pieces: `(piece * 64 + square from White's side, +1 White / -1 Black)`.
        pub pieces: Vec<(u16, i8)>,
        /// The king-safety features: `(king, mg index, eg index, value)`, king 0 White, 1 Black.
        pub safety: Vec<(u8, u16, u16, f64)>,
        /// Merged constants plus the rounding residual of every unit at the shipped parameters.
        pub constant: (f64, f64),
        /// Sum of the units' tapered values as the evaluation computed them.
        pub tapered_sum: i32,
        pub units: Vec<UnitCheck>,
        pub pre_cap: Option<i32>,
        pub final_eval: Option<i32>,
    }

    struct State {
        theta0: Vec<f64>,
        trace: Trace,
        dense: Vec<(f64, f64)>,
        seen: Vec<bool>,
        touched: Vec<usize>,
        unit_terms: Vec<(usize, f64, f64)>,
        unit_const: (f64, f64),
        unit_pieces: (f64, f64),
        unit_safety: Vec<(u8, u16, u16, f64)>,
    }

    thread_local! {
        static ACTIVE: Cell<bool> = const { Cell::new(false) };
        static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    }

    #[inline(always)]
    pub fn active() -> bool {
        ACTIVE.with(|a| a.get())
    }

    /// Starts a trace; `theta0` is the vector the evaluation runs with ([`theta_of`]).
    pub fn begin(theta0: &[f64]) {
        STATE.with(|s| {
            *s.borrow_mut() = Some(State {
                theta0: theta0.to_vec(),
                trace: Trace::default(),
                dense: vec![(0.0, 0.0); N_PARAMS],
                seen: vec![false; N_PARAMS],
                touched: Vec::new(),
                unit_terms: Vec::new(),
                unit_const: (0.0, 0.0),
                unit_pieces: (0.0, 0.0),
                unit_safety: Vec::new(),
            })
        });
        ACTIVE.with(|a| a.set(true));
    }

    /// Ends the trace and returns it.
    pub fn finish() -> Trace {
        ACTIVE.with(|a| a.set(false));
        STATE.with(|s| {
            let mut state = s.borrow_mut().take().expect("no trace running");
            for &i in &state.touched {
                let (m, e) = state.dense[i];
                if m != 0.0 || e != 0.0 {
                    state.trace.terms.push((i, m, e));
                }
            }
            state.trace
        })
    }

    pub fn term(index: usize, c_mg: f64, c_eg: f64) {
        STATE.with(|s| {
            if let Some(state) = s.borrow_mut().as_mut() {
                state.unit_terms.push((index, c_mg, c_eg));
            }
        });
    }

    pub fn safety(king: usize, mg_index: usize, eg_index: usize, value: f64) {
        STATE.with(|s| {
            if let Some(state) = s.borrow_mut().as_mut() {
                state.unit_safety.push((king as u8, mg_index as u16, eg_index as u16, value));
            }
        });
    }

    /// The `(mg, eg)` contribution of `features` at `theta`, White-positive: each king's penalty
    /// counts against its own side.
    pub fn safety_contribution(features: &[(u8, u16, u16, f64)], theta: &[f64]) -> (f64, f64) {
        let mut u = [(0.0, 0.0); 2];
        for &(king, mg, eg, value) in features {
            u[king as usize].0 += theta[mg as usize] * value;
            u[king as usize].1 += theta[eg as usize] * value;
        }
        let (wm, we) = super::safety_penalty(u[0].0, u[0].1);
        let (bm, be) = super::safety_penalty(u[1].0, u[1].1);
        (bm - wm, be - we)
    }

    pub fn constant(k_mg: f64, k_eg: f64) {
        STATE.with(|s| {
            if let Some(state) = s.borrow_mut().as_mut() {
                state.unit_const.0 += k_mg;
                state.unit_const.1 += k_eg;
            }
        });
    }

    pub fn pieces(board: &Board) {
        STATE.with(|s| {
            if let Some(state) = s.borrow_mut().as_mut() {
                for (bb_idx, &bb) in board.bitboards.iter().enumerate() {
                    let white = bb_idx < 6;
                    let piece = bb_idx % 6;
                    let mut b = bb;
                    while b != 0 {
                        let sq = b.trailing_zeros() as usize;
                        let visual = if white { sq ^ 56 } else { sq };
                        let sign = if white { 1.0 } else { -1.0 };
                        state.trace.pieces.push(((piece * 64 + visual) as u16, sign as i8));
                        let t = &state.theta0;
                        state.unit_pieces.0 += sign * (t[MATERIAL_MG + piece] + t[TABLE_MG + piece * 64 + visual]);
                        state.unit_pieces.1 += sign * (t[MATERIAL_EG + piece] + t[TABLE_EG + piece * 64 + visual]);
                        b &= b - 1;
                    }
                }
            }
        });
    }

    pub fn unit_end(actual_mg: f64, actual_eg: f64, tapered: i32) {
        STATE.with(|s| {
            if let Some(state) = s.borrow_mut().as_mut() {
                let mut predicted = (state.unit_const.0 + state.unit_pieces.0, state.unit_const.1 + state.unit_pieces.1);
                if !state.unit_safety.is_empty() {
                    let (m, e) = safety_contribution(&state.unit_safety, &state.theta0);
                    predicted.0 += m;
                    predicted.1 += e;
                    let features = std::mem::take(&mut state.unit_safety);
                    state.trace.safety.extend(features);
                }
                for &(i, cm, ce) in &state.unit_terms {
                    predicted.0 += state.theta0[i] * cm;
                    predicted.1 += state.theta0[i] * ce;
                    if !state.seen[i] {
                        state.seen[i] = true;
                        state.touched.push(i);
                    }
                    state.dense[i].0 += cm;
                    state.dense[i].1 += ce;
                }
                // The constant absorbs what integer arithmetic inside the unit rounded away, so
                // the shipped parameters rebuild the unit's sums exactly.
                state.trace.constant.0 += state.unit_const.0 + (actual_mg - predicted.0);
                state.trace.constant.1 += state.unit_const.1 + (actual_eg - predicted.1);
                state.trace.tapered_sum += tapered;
                state.trace.units.push(UnitCheck { predicted, actual: (actual_mg, actual_eg) });
                state.unit_terms.clear();
                state.unit_const = (0.0, 0.0);
                state.unit_pieces = (0.0, 0.0);
            }
        });
    }

    pub fn pre_cap(value: i32) {
        STATE.with(|s| {
            if let Some(state) = s.borrow_mut().as_mut() {
                state.trace.pre_cap = Some(value);
            }
        });
    }

    pub fn final_eval(value: i32) {
        STATE.with(|s| {
            if let Some(state) = s.borrow_mut().as_mut() {
                state.trace.final_eval = Some(value);
            }
        });
    }

    /// `(delta * rank_factor) / 8` of `king_passer_proximity_score`, before the weight.
    pub fn king_passer_coefficient(sq: u8, is_white: bool, wk_sq: u8, bk_sq: u8) -> f64 {
        let (p_rank, p_file) = ((sq / 8) as i32, (sq % 8) as i32);
        let w_dist = (p_rank - (wk_sq / 8) as i32).abs().max((p_file - (wk_sq % 8) as i32).abs());
        let b_dist = (p_rank - (bk_sq / 8) as i32).abs().max((p_file - (bk_sq % 8) as i32).abs());
        let rank_factor = if is_white { p_rank + 1 } else { 8 - p_rank };
        let delta = if is_white { b_dist - w_dist } else { w_dist - b_dist };
        (delta * rank_factor) as f64 / 8.0
    }
}
