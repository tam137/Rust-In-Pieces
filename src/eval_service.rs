use crate::config::Config;
use crate::model::Board;
use crate::move_gen_service::MoveGenService;
use crate::eval_trace::{tr, tr_const, tr_direct, tr_mark, tr_param, tr_pst, tr_safety, tr_unit};


const ADJACENT_FILES_MASK: [u64; 8] = [
    0x0202020202020202u64,
    0x0505050505050505u64,
    0x0a0a0a0a0a0a0a0au64,
    0x1414141414141414u64,
    0x2828282828282828u64,
    0x5050505050505050u64,
    0xa0a0a0a0a0a0a0a0u64,
    0x4040404040404040u64,
];

const KING_RING_MASKS: [u64; 64] = {
    let mut masks = [0; 64];
    let mut sq = 0;
    while sq < 64 {
        let file = sq % 8;
        let rank = sq / 8;
        let mut mask = 0u64;
        let mut df = -1;
        while df <= 1 {
            let mut dr = -1;
            while dr <= 1 {
                if !(df == 0 && dr == 0) {
                    let f = file + df;
                    let r = rank + dr;
                    if f >= 0 && f < 8 && r >= 0 && r < 8 {
                        mask |= 1u64 << (r * 8 + f);
                    }
                }
                dr += 1;
            }
            df += 1;
        }
        masks[sq as usize] = mask;
        sq += 1;
    }
    masks
};

const WHITE_PASSED_PAWN_MASKS: [u64; 64] = {
    let mut masks = [0; 64];
    let mut sq = 0;
    while sq < 64 {
        let file = sq % 8;
        let rank = sq / 8;
        let mut mask = 0u64;
        let mut r = rank + 1;
        while r < 8 {
            mask |= 1u64 << (r * 8 + file);
            if file > 0 {
                mask |= 1u64 << (r * 8 + file - 1);
            }
            if file < 7 {
                mask |= 1u64 << (r * 8 + file + 1);
            }
            r += 1;
        }
        masks[sq as usize] = mask;
        sq += 1;
    }
    masks
};

const BLACK_PASSED_PAWN_MASKS: [u64; 64] = {
    let mut masks = [0; 64];
    let mut sq = 0;
    while sq < 64 {
        let file = sq % 8;
        let rank = sq / 8;
        let mut mask = 0u64;
        let mut r = 0;
        while r < rank {
            mask |= 1u64 << (r * 8 + file);
            if file > 0 {
                mask |= 1u64 << (r * 8 + file - 1);
            }
            if file < 7 {
                mask |= 1u64 << (r * 8 + file + 1);
            }
            r += 1;
        }
        masks[sq as usize] = mask;
        sq += 1;
    }
    masks
};

/// The squares each side attacks, built once per evaluation from the attack sets the piece terms
/// compute anyway: `by[side][piece]` per piece type (`WHITE_PAWN..=WHITE_KING` indexing, side 0
/// White, 1 Black), `all[side]` their union, `two[side]` the squares attacked at least twice.
/// Pseudo-attacks: pins are ignored and sliders stop at the first piece, as the mobility terms
/// count them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct AttackMaps {
    pub by: [[u64; 6]; 2],
    pub all: [u64; 2],
    pub two: [u64; 2],
}

impl AttackMaps {
    /// The maps after the pawns: a square two pawns attack is attacked twice.
    #[inline(always)]
    fn new(white_pawns: u64, black_pawns: u64, white_pawn_attacks: u64, black_pawn_attacks: u64) -> Self {
        const FILE_A: u64 = 0x0101010101010101;
        const FILE_H: u64 = 0x8080808080808080;
        let white_double = ((white_pawns & !FILE_A) << 7) & ((white_pawns & !FILE_H) << 9);
        let black_double = ((black_pawns & !FILE_H) >> 7) & ((black_pawns & !FILE_A) >> 9);
        let mut maps = AttackMaps::default();
        maps.by[0][crate::model::WHITE_PAWN] = white_pawn_attacks;
        maps.by[1][crate::model::WHITE_PAWN] = black_pawn_attacks;
        maps.all = [white_pawn_attacks, black_pawn_attacks];
        maps.two = [white_double, black_double];
        maps
    }

    /// Adds the attack set of one piece of `side`; `piece` is its type as a White index.
    #[inline(always)]
    fn add(&mut self, side: usize, piece: usize, attacks: u64) {
        self.two[side] |= self.all[side] & attacks;
        self.all[side] |= attacks;
        self.by[side][piece] |= attacks;
    }
}

/// What one side's pieces aim at the enemy king's ring: attackers per type (knight, bishop, rook,
/// queen), and the ring squares they hit, counted once per attacker.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RingAttack {
    pub by_type: [i32; 4],
    pub count: i32,
    pub hits: i32,
}

impl RingAttack {
    /// One piece of type `slot` with `on_ring` its attacks on the ring.
    #[inline(always)]
    fn add(&mut self, slot: usize, on_ring: u64) {
        if on_ring != 0 {
            self.by_type[slot] += 1;
            self.count += 1;
            self.hits += on_ring.count_ones() as i32;
        }
    }
}

// The attack maps of the last full evaluation on this thread, for the tests.
#[cfg(test)]
thread_local! {
    static LAST_MAPS: std::cell::Cell<AttackMaps> = std::cell::Cell::new(AttackMaps::default());
}

pub struct EvalService {
    _knight_moves: [i16; 8],
    pub nnue_net: crate::nnue_service::NNUENetwork,
}

impl EvalService {

    pub fn new(config: &Config) -> Self {
        let nnue_net = if config.use_nnue {
            match crate::nnue_service::NNUENetwork::load_from_file(&config.nnue_model_path) {
                Ok(net) => net,
                Err(e) => {
                    let msg = format!("RIP Critical Error: Failed to load NNUE network file '{}': {}", config.nnue_model_path, e);
                    eprintln!("{}", msg);
                    std::process::exit(1);
                }
            }
        } else {
            crate::nnue_service::NNUENetwork::new_empty()
        };

        Self {
            _knight_moves: [-21, -19, -12, -8, 21, 19, 12, 8],
            nnue_net,
        }
    }

    pub fn cheap_eval(&self, board: &Board, config: &Config, pawn_table: &crate::pawn_hash::PawnHashTable) -> i16 {
        if Self::is_insufficient_material(board) {
            return 0;
        }

        let game_phase = self.get_game_phase(board) as i16;
        let mut eval: i16 = self.calculate_weighted_eval(board.pst_mg, board.pst_eg, game_phase);

        // Add pawn structure from table if present
        let mut struct_mg = 0;
        let mut struct_eg = 0;
        if let Some((cached_mg, cached_eg)) = pawn_table.get(board.pawn_key) {
            struct_mg = cached_mg;
            struct_eg = cached_eg;
        }
        eval += self.calculate_weighted_eval(struct_mg, struct_eg, game_phase);

        // Apply bishop pair scaling if opposite colored bishops
        if Self::is_opposite_colored_bishops_endgame(board) {
            eval = Self::scale_opposite_bishops(eval, config);
        }

        // Apply endgame scaling
        eval = self.adjust_eval(eval, game_phase, config);
        eval
    }

    /// `eval * opposite_bishops_draw_scale / 100` in `i32`. In `i16` the product wrapped above
    /// 642 centipawns at the default scale of 51 and flipped the sign of a winning evaluation.
    #[inline(always)]
    fn scale_opposite_bishops(eval: i16, config: &Config) -> i16 {
        (eval as i32 * config.opposite_bishops_draw_scale as i32 / 100) as i16
    }

    /// The static evaluation, White-positive. It finds out for itself whether the side to move is
    /// in check and counts the attackers of both kings, which is the reference behaviour the tests
    /// and diagnostics use. The search calls [`Self::calc_eval_known_check`].
    pub fn calc_eval(&self, board: &Board, config: &Config, movegen: &MoveGenService, pawn_table: &crate::pawn_hash::PawnHashTable, alpha: i16, beta: i16, margin: i16) -> i16 {
        let in_check = movegen.is_in_check(board);
        self.evaluate(board, config, movegen, pawn_table, alpha, beta, margin, in_check, true)
    }

    /// [`Self::calc_eval`] for a caller that already knows whether the side to move is in check.
    /// The search always does - it is the `gives_check` of the move that led to the node - and it
    /// evaluates a node in check only at its ply ceiling. In a legal position only the side to
    /// move can be in check, so the king-safety terms count the attackers of that king alone, and
    /// of neither king when it is not in check: the counts `calc_eval` computes, without the check
    /// test and the two attacker scans it pays at every leaf.
    pub fn calc_eval_known_check(&self, board: &Board, config: &Config, movegen: &MoveGenService, pawn_table: &crate::pawn_hash::PawnHashTable, alpha: i16, beta: i16, margin: i16, in_check: bool) -> i16 {
        self.evaluate(board, config, movegen, pawn_table, alpha, beta, margin, in_check, false)
    }

    fn evaluate(&self, board: &Board, config: &Config, movegen: &MoveGenService, pawn_table: &crate::pawn_hash::PawnHashTable, alpha: i16, beta: i16, margin: i16, in_check: bool, count_both_kings: bool) -> i16 {
        if config.use_nnue && self.nnue_net.loaded {
            return crate::nnue_service::NNUEService::evaluate(board, &self.nnue_net);
        }
        if Self::is_insufficient_material(board) {
            return 0;
        }
        let game_phase = self.get_game_phase(board);

        // Skip Lazy Eval if disabled, if in check, or in deep endgame (game_phase < min_game_phase)
        // where positional opposition & passed pawn dynamics dominate. Neither return can fire
        // when both bounds are open, so `cheap_eval` is computed only when one of them is not.
        if config.enable_lazy_eval && !in_check && game_phase >= config.lazy_eval_min_game_phase
            && (alpha > i16::MIN + 2000 || beta < i16::MAX - 2000)
        {
            let cheap = self.cheap_eval(board, config, pawn_table);
            if alpha > i16::MIN + 2000 && cheap + margin <= alpha {
                return cheap;
            }
            if beta < i16::MAX - 2000 && cheap - margin >= beta {
                return cheap;
            }
        }

        let game_phase = self.get_game_phase(board) as i16;
        tr_pst!(board);
        let mut eval: i16 = self.calculate_weighted_eval(board.pst_mg, board.pst_eg, game_phase);

        // Pawn structure evaluation and caching
        let mut white_passed_pawns = 0u64;
        let mut black_passed_pawns = 0u64;

        let mut temp_w = board.bitboards[crate::model::WHITE_PAWN];
        while temp_w != 0 {
            let sq = temp_w.trailing_zeros() as u8;
            if self.is_white_passed_pawn(sq, board) {
                white_passed_pawns |= 1u64 << sq;
            }
            temp_w &= temp_w - 1;
        }
        let mut temp_b = board.bitboards[crate::model::BLACK_PAWN];
        while temp_b != 0 {
            let sq = temp_b.trailing_zeros() as u8;
            if self.is_black_passed_pawn(sq, board) {
                black_passed_pawns |= 1u64 << sq;
            }
            temp_b &= temp_b - 1;
        }

        let mut struct_mg = 0;
        let mut struct_eg = 0;

        // A trace needs every pawn term to report itself, so it computes the structure afresh.
        let cached = if crate::eval_trace::tracing() { None } else { pawn_table.get(board.pawn_key) };
        if let Some((cached_mg, cached_eg)) = cached {
            struct_mg = cached_mg;
            struct_eg = cached_eg;
        } else {
            let mut temp_w = board.bitboards[crate::model::WHITE_PAWN];
            while temp_w != 0 {
                let sq = temp_w.trailing_zeros() as u8;
                let (mg, eg) = self.white_pawn_structure_score(sq, board, config, white_passed_pawns);
                struct_mg += mg;
                struct_eg += eg;
                temp_w &= temp_w - 1;
            }
            let mut temp_b = board.bitboards[crate::model::BLACK_PAWN];
            while temp_b != 0 {
                let sq = temp_b.trailing_zeros() as u8;
                let (mg, eg) = self.black_pawn_structure_score(sq, board, config, black_passed_pawns);
                struct_mg += mg;
                struct_eg += eg;
                temp_b &= temp_b - 1;
            }
            pawn_table.store(board.pawn_key, struct_mg, struct_eg);
        }

        // Add dynamic pawn scores (King proximity, attacks/defenses)
        let (dyn_mg, dyn_eg) = Self::pawn_dynamic_scores(board, config, white_passed_pawns, black_passed_pawns);

        eval += self.calculate_weighted_eval(struct_mg + dyn_mg, struct_eg + dyn_eg, game_phase);

        // Precalculate true outpost squares using efficient candidate bitboards
        let mut white_true_outposts = 0u64;
        let mut black_true_outposts = 0u64;

        let white_pawns = board.bitboards[crate::model::WHITE_PAWN];
        let black_pawns = board.bitboards[crate::model::BLACK_PAWN];

        if (board.bitboards[crate::model::WHITE_KNIGHT] | board.bitboards[crate::model::WHITE_BISHOP]) != 0 {
            white_true_outposts = Self::true_outposts(white_pawns, black_pawns, true);
        }
        if (board.bitboards[crate::model::BLACK_KNIGHT] | board.bitboards[crate::model::BLACK_BISHOP]) != 0 {
            black_true_outposts = Self::true_outposts(white_pawns, black_pawns, false);
        }

        let white_pawn_attacks = Self::get_white_pawn_attacks(white_pawns);
        let black_pawn_attacks = Self::get_black_pawn_attacks(black_pawns);

        let white_king_sq = board.bitboards[crate::model::WHITE_KING].trailing_zeros() as u8;
        let black_king_sq = board.bitboards[crate::model::BLACK_KING].trailing_zeros() as u8;
        let white_king_ring = self.get_king_ring(white_king_sq);
        let black_king_ring = self.get_king_ring(black_king_sq);

        let mut white_attackers = 0;
        let mut black_attackers = 0;
        let mut white_king_danger = 0;
        let mut black_king_danger = 0;
        // Minor pieces that attack their own king's ring: the defenders the king danger subtracts.
        let mut white_defenders = 0;
        let mut black_defenders = 0;
        let mut maps = AttackMaps::new(white_pawns, black_pawns, white_pawn_attacks, black_pawn_attacks);
        // What each side's pieces aim at the enemy king's ring, for the king safety.
        let mut white_on_ring = RingAttack::default();
        let mut black_on_ring = RingAttack::default();

        let mut temp_w_rook = board.bitboards[crate::model::WHITE_ROOK];
        while temp_w_rook != 0 {
            let sq = temp_w_rook.trailing_zeros() as u8;
            let (eval_for_piece, attackers, danger, attacks) = self.white_rook(sq, board, config, game_phase, movegen, black_king_ring, black_pawn_attacks, board.white_pieces);
            if config.print_eval_per_figure { println!("{},\t11,\t{}", sq, eval_for_piece); }
            eval += eval_for_piece;
            white_attackers += attackers;
            white_king_danger += danger;
            maps.add(0, crate::model::WHITE_ROOK, attacks);
            white_on_ring.add(2, attacks & black_king_ring);
            temp_w_rook &= temp_w_rook - 1;
        }
        let mut temp_w_knight = board.bitboards[crate::model::WHITE_KNIGHT];
        while temp_w_knight != 0 {
            let sq = temp_w_knight.trailing_zeros() as u8;
            let (eval_for_piece, attackers, danger, attacks) = self.white_knight(sq, board, config, game_phase, movegen, black_king_ring, white_true_outposts, black_pawn_attacks, board.white_pieces);
            if config.print_eval_per_figure { println!("{},\t12,\t{}", sq, eval_for_piece); }
            eval += eval_for_piece;
            white_attackers += attackers;
            white_king_danger += danger;
            maps.add(0, crate::model::WHITE_KNIGHT, attacks);
            white_on_ring.add(0, attacks & black_king_ring);
            if attacks & white_king_ring != 0 { white_defenders += 1; }
            temp_w_knight &= temp_w_knight - 1;
        }
        let mut temp_w_bishop = board.bitboards[crate::model::WHITE_BISHOP];
        while temp_w_bishop != 0 {
            let sq = temp_w_bishop.trailing_zeros() as u8;
            let (eval_for_piece, attackers, danger, attacks) = self.white_bishop(sq, board, config, game_phase, movegen, black_king_ring, white_true_outposts, black_pawn_attacks, board.white_pieces);
            if config.print_eval_per_figure { println!("{},\t13,\t{}", sq, eval_for_piece); }
            eval += eval_for_piece;
            white_attackers += attackers;
            white_king_danger += danger;
            maps.add(0, crate::model::WHITE_BISHOP, attacks);
            white_on_ring.add(1, attacks & black_king_ring);
            if attacks & white_king_ring != 0 { white_defenders += 1; }
            temp_w_bishop &= temp_w_bishop - 1;
        }
        let mut temp_w_queen = board.bitboards[crate::model::WHITE_QUEEN];
        while temp_w_queen != 0 {
            let sq = temp_w_queen.trailing_zeros() as u8;
            let (eval_for_piece, attackers, danger, attacks) = self.white_queen(sq, board, config, game_phase, movegen, black_king_ring, black_pawn_attacks, board.white_pieces);
            if config.print_eval_per_figure { println!("{},\t14,\t{}", sq, eval_for_piece); }
            eval += eval_for_piece;
            white_attackers += attackers;
            white_king_danger += danger;
            maps.add(0, crate::model::WHITE_QUEEN, attacks);
            white_on_ring.add(3, attacks & black_king_ring);
            temp_w_queen &= temp_w_queen - 1;
        }
        let mut temp_w_king = board.bitboards[crate::model::WHITE_KING];
        while temp_w_king != 0 {
            let sq = temp_w_king.trailing_zeros() as u8;
            let (eval_for_piece, _attackers, _danger) = self.white_king(sq, board, config, game_phase, movegen,
                count_both_kings || (in_check && board.white_to_move));
            if config.print_eval_per_figure { println!("{},\t15,\t{}", sq, eval_for_piece); }
            eval += eval_for_piece;
            maps.add(0, crate::model::WHITE_KING, movegen.get_king_attacks(sq as usize));
            temp_w_king &= temp_w_king - 1;
        }

        let mut temp_b_rook = board.bitboards[crate::model::BLACK_ROOK];
        while temp_b_rook != 0 {
            let sq = temp_b_rook.trailing_zeros() as u8;
            let (eval_for_piece, attackers, danger, attacks) = self.black_rook(sq, board, config, game_phase, movegen, white_king_ring, white_pawn_attacks, board.black_pieces);
            if config.print_eval_per_figure { println!("{},\t21,\t{}", sq, eval_for_piece); }
            eval += eval_for_piece;
            black_attackers += attackers;
            black_king_danger += danger;
            maps.add(1, crate::model::WHITE_ROOK, attacks);
            black_on_ring.add(2, attacks & white_king_ring);
            temp_b_rook &= temp_b_rook - 1;
        }
        let mut temp_b_knight = board.bitboards[crate::model::BLACK_KNIGHT];
        while temp_b_knight != 0 {
            let sq = temp_b_knight.trailing_zeros() as u8;
            let (eval_for_piece, attackers, danger, attacks) = self.black_knight(sq, board, config, game_phase, movegen, white_king_ring, black_true_outposts, white_pawn_attacks, board.black_pieces);
            if config.print_eval_per_figure { println!("{},\t22,\t{}", sq, eval_for_piece); }
            eval += eval_for_piece;
            black_attackers += attackers;
            black_king_danger += danger;
            maps.add(1, crate::model::WHITE_KNIGHT, attacks);
            black_on_ring.add(0, attacks & white_king_ring);
            if attacks & black_king_ring != 0 { black_defenders += 1; }
            temp_b_knight &= temp_b_knight - 1;
        }
        let mut temp_b_bishop = board.bitboards[crate::model::BLACK_BISHOP];
        while temp_b_bishop != 0 {
            let sq = temp_b_bishop.trailing_zeros() as u8;
            let (eval_for_piece, attackers, danger, attacks) = self.black_bishop(sq, board, config, game_phase, movegen, white_king_ring, black_true_outposts, white_pawn_attacks, board.black_pieces);
            if config.print_eval_per_figure { println!("{},\t23,\t{}", sq, eval_for_piece); }
            eval += eval_for_piece;
            black_attackers += attackers;
            black_king_danger += danger;
            maps.add(1, crate::model::WHITE_BISHOP, attacks);
            black_on_ring.add(1, attacks & white_king_ring);
            if attacks & black_king_ring != 0 { black_defenders += 1; }
            temp_b_bishop &= temp_b_bishop - 1;
        }
        let mut temp_b_queen = board.bitboards[crate::model::BLACK_QUEEN];
        while temp_b_queen != 0 {
            let sq = temp_b_queen.trailing_zeros() as u8;
            let (eval_for_piece, attackers, danger, attacks) = self.black_queen(sq, board, config, game_phase, movegen, white_king_ring, white_pawn_attacks, board.black_pieces);
            if config.print_eval_per_figure { println!("{},\t24,\t{}", sq, eval_for_piece); }
            eval += eval_for_piece;
            black_attackers += attackers;
            black_king_danger += danger;
            maps.add(1, crate::model::WHITE_QUEEN, attacks);
            black_on_ring.add(3, attacks & white_king_ring);
            temp_b_queen &= temp_b_queen - 1;
        }
        let mut temp_b_king = board.bitboards[crate::model::BLACK_KING];
        while temp_b_king != 0 {
            let sq = temp_b_king.trailing_zeros() as u8;
            let (eval_for_piece, _attackers, _danger) = self.black_king(sq, board, config, game_phase, movegen,
                count_both_kings || (in_check && !board.white_to_move));
            if config.print_eval_per_figure { println!("{},\t25,\t{}", sq, eval_for_piece); }
            eval += eval_for_piece;
            maps.add(1, crate::model::WHITE_KING, movegen.get_king_attacks(sq as usize));
            temp_b_king &= temp_b_king - 1;
        }

        #[cfg(test)]
        LAST_MAPS.with(|m| m.set(maps));

        // Apply King Danger Weights (Task 1.4)
        let danger_weights = [
            0,
            config.king_danger_weight_1,
            config.king_danger_weight_2,
            config.king_danger_weight_3,
            config.king_danger_weight_4,
            config.king_danger_weight_5,
        ];

        if white_attackers > 0 {
            let effective_attackers = (white_attackers as i16 - (black_defenders as i16 * config.king_ring_defender_value)).max(0) as usize;
            let idx = std::cmp::min(effective_attackers, 5);
            let danger = (white_king_danger * danger_weights[idx]) / 100;
            if idx > 0 { tr_param!(crate::eval_trace::king_danger(idx), white_king_danger as f64 / 100.0, white_king_danger as f64 / 100.0); }
            tr_direct!(danger);
            eval += danger;
        }
        if black_attackers > 0 {
            let effective_attackers = (black_attackers as i16 - (white_defenders as i16 * config.king_ring_defender_value)).max(0) as usize;
            let idx = std::cmp::min(effective_attackers, 5);
            let danger = (black_king_danger * danger_weights[idx]) / 100;
            if idx > 0 { tr_param!(crate::eval_trace::king_danger(idx), -(black_king_danger as f64) / 100.0, -(black_king_danger as f64) / 100.0); }
            tr_direct!(-danger);
            eval -= danger;
        }

        let white_king_safety = self.king_safety(board, config, movegen, &maps, 0, white_king_sq, white_king_ring, &black_on_ring);
        let black_king_safety = self.king_safety(board, config, movegen, &maps, 1, black_king_sq, black_king_ring, &white_on_ring);
        let king_safety = self.calculate_weighted_eval(black_king_safety.0 - white_king_safety.0,
            black_king_safety.1 - white_king_safety.1, game_phase);
        eval += king_safety;

        eval += self.threats(board, config, &maps, game_phase);

        // Connected passed pawns bonus
        let mut white_connected_passed_pawns = 0;
        let mut temp_w_passed = white_passed_pawns;
        while temp_w_passed != 0 {
            let sq1 = temp_w_passed.trailing_zeros() as usize;
            let file1 = sq1 & 7;
            let file_mask = 0x0101010101010101u64 << file1;
            let adjacent_king_attacks = movegen.get_king_attacks(sq1) & !file_mask;
            if (adjacent_king_attacks & white_passed_pawns) != 0 {
                white_connected_passed_pawns += 1;
            }
            temp_w_passed &= temp_w_passed - 1;
        }
        
        let mut black_connected_passed_pawns = 0;
        let mut temp_b_passed = black_passed_pawns;
        while temp_b_passed != 0 {
            let sq1 = temp_b_passed.trailing_zeros() as usize;
            let file1 = sq1 & 7;
            let file_mask = 0x0101010101010101u64 << file1;
            let adjacent_king_attacks = movegen.get_king_attacks(sq1) & !file_mask;
            if (adjacent_king_attacks & black_passed_pawns) != 0 {
                black_connected_passed_pawns += 1;
            }
            temp_b_passed &= temp_b_passed - 1;
        }

        if white_connected_passed_pawns > 0 || black_connected_passed_pawns > 0 {
            tr!(ConnectedPassedPawnMg, white_connected_passed_pawns - black_connected_passed_pawns, 0);
            tr!(ConnectedPassedPawnEg, 0, white_connected_passed_pawns - black_connected_passed_pawns);
            let o_bonus = (white_connected_passed_pawns - black_connected_passed_pawns) as i16 * config.connected_passed_pawn_mg;
            let e_bonus = (white_connected_passed_pawns - black_connected_passed_pawns) as i16 * config.connected_passed_pawn_eg;
            eval += self.calculate_weighted_eval(o_bonus, e_bonus, game_phase);
        }

        let mut o_bishop_pair = 0;
        let mut e_bishop_pair = 0;
        if board.bitboards[crate::model::WHITE_BISHOP].count_ones() >= 2 {
            tr!(BishopPairBonus, 1, 1);
            tr_const!(0, 15);
            o_bishop_pair += config.bishop_pair_bonus;
            e_bishop_pair += config.bishop_pair_bonus + 15;
        }
        if board.bitboards[crate::model::BLACK_BISHOP].count_ones() >= 2 {
            tr!(BishopPairBonus, -1, -1);
            tr_const!(0, -15);
            o_bishop_pair -= config.bishop_pair_bonus;
            e_bishop_pair -= config.bishop_pair_bonus + 15;
        }
        eval += self.calculate_weighted_eval(o_bishop_pair, e_bishop_pair, game_phase);



        // Opposition in Endgames
        if game_phase < 40 {
            let wk_rank = (white_king_sq / 8) as i32;
            let wk_file = (white_king_sq % 8) as i32;
            let bk_rank = (black_king_sq / 8) as i32;
            let bk_file = (black_king_sq % 8) as i32;
            
            let has_vertical_opp = wk_file == bk_file && (wk_rank - bk_rank).abs() == 2;
            let has_horizontal_opp = wk_rank == bk_rank && (wk_file - bk_file).abs() == 2;
            
            if has_vertical_opp || has_horizontal_opp {
                if board.white_to_move {
                    // Black holds opposition
                    tr!(KingOppositionBonus, -1, -1);
                    tr_direct!(-config.king_opposition_bonus);
                    eval -= config.king_opposition_bonus;
                } else {
                    // White holds opposition
                    tr!(KingOppositionBonus, 1, 1);
                    tr_direct!(config.king_opposition_bonus);
                    eval += config.king_opposition_bonus;
                }
            }
        }

        tr!(YourTurnBonus, if board.white_to_move { 1 } else { -1 }, if board.white_to_move { 1 } else { -1 });
        tr_direct!(if board.white_to_move { config.your_turn_bonus } else { -config.your_turn_bonus });
        eval += if board.white_to_move { config.your_turn_bonus } else { -config.your_turn_bonus };

        tr_mark!(pre_cap, eval);
        if config.enable_positional_cap {
            let material_eval = Self::tapered_material(board, game_phase);
            let outside = if config.king_safety_outside_cap { king_safety } else { 0 };
            let positional_eval = eval - material_eval - outside;

            let cap = match config.aggressiveness {
                crate::config::Aggressiveness::Normal => 150,
                crate::config::Aggressiveness::Aggressive => 250,
                crate::config::Aggressiveness::HighAggressive => 400,
            };

            let capped_positional = if positional_eval.abs() <= cap {
                positional_eval
            } else {
                let excess = positional_eval.abs() - cap;
                let damping = if config.positional_cap_damping > 0 { config.positional_cap_damping } else { 1 };
                let capped_excess = excess / damping; // soft compression on excess positional values to avoid Saturation Blindness
                let sign = if positional_eval >= 0 { 1 } else { -1 };
                sign * (cap + capped_excess)
            };
            eval = material_eval + capped_positional + outside;
        }

        if Self::is_opposite_colored_bishops_endgame(board) {
            eval = Self::scale_opposite_bishops(eval, config);
        }

        eval = self.adjust_eval(eval, game_phase, config);

        eval = self.apply_endgame_mopup(eval, board, game_phase, config);

        if config.print_eval_per_figure {
            println!("{}", eval);
        }
        tr_mark!(final_eval, eval);
        eval
    }

    fn white_pawn_structure_score(&self, sq: u8, board: &Board, config: &Config, precalculated_passed_pawns: u64) -> (i16, i16) {
        let mut o_eval = 0;
        let mut e_eval = 0;
        let sq = sq as i32;
        let rank = sq / 8;
        let file = sq % 8;
        
        let moves_until_promote = 7 - rank;
        let _on_rank = rank + 1;



        if sq == 27 || sq == 28 || sq == 35 || sq == 36 {
            tr!(PawnCentered, 1, 0);
            o_eval += config.pawn_centered;
        }

        match moves_until_promote {
            1 => e_eval += config.pawn_on_last_rank_bonus,
            2 => e_eval += config.pawn_on_before_last_rank_bonus,
            3 => e_eval += config.pawn_on_before_before_last_rank_bonus,
            _ => ()
        }

        match moves_until_promote {
            1 => o_eval += config.pawn_on_last_rank_bonus / 2,
            2 => o_eval += config.pawn_on_before_last_rank_bonus / 2,
            3 => o_eval += config.pawn_on_before_before_last_rank_bonus / 2,
            _ => ()
        }
        match moves_until_promote {
            1 => tr!(PawnOnLastRankBonus, 0.5, 1),
            2 => tr!(PawnOnBeforeLastRankBonus, 0.5, 1),
            3 => tr!(PawnOnBeforeBeforeLastRankBonus, 0.5, 1),
            _ => ()
        }

        let white_pawns = board.bitboards[crate::model::WHITE_PAWN];
        if (file < 7 && ((1u64 << (sq + 9)) & white_pawns) != 0) ||
           (file > 0 && ((1u64 << (sq + 7)) & white_pawns) != 0) {
            tr!(PawnStructure, 1, 0);
            o_eval += config.pawn_structure;
        }



        if moves_until_promote >= 5 && (file == 3 || file == 4) {
            tr!(PawnUndevelopedMalus, -1, 0);
            o_eval -= config.pawn_undeveloped_malus;
        }



        let front_mask = (0x0101010101010101u64 << file) & !((1u64 << (sq + 1)) - 1);
        let has_doubled_pawn = (white_pawns & front_mask) != 0;
        if has_doubled_pawn {
            tr!(PawnDoubleMalus, -1, -1);
            o_eval -= config.pawn_double_malus;
            e_eval -= config.pawn_double_malus;
        }

        let is_passed = (1u64 << sq) & precalculated_passed_pawns != 0;
        if is_passed {
            let (bonus_mg, bonus_eg) = (config.passed_pawn_bonus_mg[rank as usize], config.passed_pawn_bonus_eg[rank as usize]);
            let is_protected = (file > 0 && ((1u64 << (sq - 9)) & white_pawns) != 0) ||
                               (file < 7 && ((1u64 << (sq - 7)) & white_pawns) != 0);
            if is_protected {
                tr!(ProtectedPassedPawnMiddlegame, 1, 0);
                tr!(ProtectedPassedPawnEndgame, 0, 1);
                o_eval += config.protected_passed_pawn_middlegame;
                e_eval += config.protected_passed_pawn_endgame;
            }

            // Passed pawn blockade: enemy piece on square directly in front
            let front_sq = sq + 8;
            if front_sq < 64 && ((1u64 << front_sq) & board.black_pieces) != 0 {
                tr!(PassedPawnBlockadedMalus, -0.5, -1);
                o_eval -= config.passed_pawn_blockaded_malus / 2;
                e_eval -= config.passed_pawn_blockaded_malus;
            }

            tr_param!(crate::eval_trace::passer_mg(rank as usize), 1, 0);
            tr_param!(crate::eval_trace::passer_eg(rank as usize), 0, 1);
            e_eval += bonus_eg;
            o_eval += bonus_mg;
        } else {
            // Candidate passed pawn: no opposing pawn on this file in front, and supported by friendly adjacent pawns
            let front_file_mask = (0x0101010101010101u64 << file) & !((1u64 << (sq + 1)) - 1);
            let black_pawns = board.bitboards[crate::model::BLACK_PAWN];
            if (black_pawns & front_file_mask) == 0 && (3..=5).contains(&rank) {
                let adjacent_files = ADJACENT_FILES_MASK[file as usize];
                let friendly_adj_pawns = (white_pawns & adjacent_files).count_ones();
                // Ranks strictly ahead of this pawn. The threshold is a *rank* predicate, not
                // an index one: `adjacent_files` already spans both neighbouring files, so an
                // index threshold would additionally admit `sq + 1` - the neighbour on this
                // pawn's own rank. A colour mirror flips the rank and leaves the file alone, so
                // that leak made the White and Black masks non-mirrored and the evaluation
                // colour-asymmetric. See `task.md` 7.2.
                let ahead_of_rank = !((1u64 << ((rank + 1) * 8)) - 1);
                let enemy_adj_pawns = (black_pawns & adjacent_files & ahead_of_rank).count_ones();
                if friendly_adj_pawns >= enemy_adj_pawns {
                    let advancement = (rank - 2) as i16;
                    tr!(CandidatePassedPawnBonus, advancement as f64 / 2.0, advancement);
                    o_eval += (config.candidate_passed_pawn_bonus * advancement) / 2;
                    e_eval += config.candidate_passed_pawn_bonus * advancement;
                }
            }
        }

        let adjacent_files = ADJACENT_FILES_MASK[file as usize];
        let is_isolated = (white_pawns & adjacent_files) == 0;
        if is_isolated {
            tr!(PawnIsolatedMalus, -1, -1);
            tr_const!(0, -15);
            o_eval -= config.pawn_isolated_malus;
            e_eval -= config.pawn_isolated_malus + 15;
        }
        
        let behind_and_same_rank_mask = (1u64 << (rank * 8 + 8)) - 1;
        let is_backward = !is_isolated && (white_pawns & adjacent_files & behind_and_same_rank_mask) == 0;
        if is_backward {
            tr!(PawnBackwardMalus, -1, -1);
            tr_const!(0, -10);
            o_eval -= config.pawn_backward_malus;
            e_eval -= config.pawn_backward_malus + 10;
        }

        let has_phalanx = (file > 0 && ((1u64 << (sq - 1)) & white_pawns) != 0) ||
                          (file < 7 && ((1u64 << (sq + 1)) & white_pawns) != 0);
        if has_phalanx && (3..=5).contains(&rank) {
            let advancement = (rank - 2) as i16;
            tr!(PawnPhalanxMg, advancement, 0);
            tr!(PawnPhalanxEg, 0, advancement);
            o_eval += config.pawn_phalanx_mg * advancement;
            e_eval += config.pawn_phalanx_eg * advancement;
        }

        (o_eval, e_eval)
    }

    fn black_pawn_structure_score(&self, sq: u8, board: &Board, config: &Config, precalculated_passed_pawns: u64) -> (i16, i16) {
        let mut o_eval = 0;
        let mut e_eval = 0;
        let sq = sq as i32;
        let rank = sq / 8;
        let file = sq % 8;
        
        let moves_until_promote = rank;



        if sq == 27 || sq == 28 || sq == 35 || sq == 36 {
            tr!(PawnCentered, -1, 0);
            o_eval -= config.pawn_centered;
        }

        match moves_until_promote {
            1 => e_eval -= config.pawn_on_last_rank_bonus,
            2 => e_eval -= config.pawn_on_before_last_rank_bonus,
            3 => e_eval -= config.pawn_on_before_before_last_rank_bonus,
            _ => ()
        }

        match moves_until_promote {
            1 => o_eval -= config.pawn_on_last_rank_bonus / 2,
            2 => o_eval -= config.pawn_on_before_last_rank_bonus / 2,
            3 => o_eval -= config.pawn_on_before_before_last_rank_bonus / 2,
            _ => ()
        }
        match moves_until_promote {
            1 => tr!(PawnOnLastRankBonus, -0.5, -1),
            2 => tr!(PawnOnBeforeLastRankBonus, -0.5, -1),
            3 => tr!(PawnOnBeforeBeforeLastRankBonus, -0.5, -1),
            _ => ()
        }

        let black_pawns = board.bitboards[crate::model::BLACK_PAWN];
        if (file > 0 && sq >= 9 && ((1u64 << (sq - 9)) & black_pawns) != 0) ||
           (file < 7 && sq >= 7 && ((1u64 << (sq - 7)) & black_pawns) != 0) {
            tr!(PawnStructure, -1, 0);
            o_eval -= config.pawn_structure;
        }



        if moves_until_promote >= 5 && (file == 3 || file == 4) {
            tr!(PawnUndevelopedMalus, 1, 0);
            o_eval += config.pawn_undeveloped_malus;
        }



        let back_mask = (0x0101010101010101u64 << file) & ((1u64 << sq) - 1);
        let has_doubled_pawn = (black_pawns & back_mask) != 0;
        if has_doubled_pawn {
            tr!(PawnDoubleMalus, 1, 1);
            o_eval += config.pawn_double_malus;
            e_eval += config.pawn_double_malus;
        }

        let is_passed = (1u64 << sq) & precalculated_passed_pawns != 0;
        if is_passed {
            let relative_rank = (7 - rank) as usize;
            let (bonus_mg, bonus_eg) = (config.passed_pawn_bonus_mg[relative_rank], config.passed_pawn_bonus_eg[relative_rank]);
            let is_protected = (file > 0 && sq + 7 < 64 && ((1u64 << (sq + 7)) & black_pawns) != 0) ||
                               (file < 7 && sq + 9 < 64 && ((1u64 << (sq + 9)) & black_pawns) != 0);
            if is_protected {
                tr!(ProtectedPassedPawnMiddlegame, -1, 0);
                tr!(ProtectedPassedPawnEndgame, 0, -1);
                o_eval -= config.protected_passed_pawn_middlegame;
                e_eval -= config.protected_passed_pawn_endgame;
            }

            // Passed pawn blockade: enemy piece on square directly in front
            let front_sq = sq - 8;
            if front_sq >= 0 && ((1u64 << front_sq) & board.white_pieces) != 0 {
                tr!(PassedPawnBlockadedMalus, 0.5, 1);
                o_eval += config.passed_pawn_blockaded_malus / 2;
                e_eval += config.passed_pawn_blockaded_malus;
            }

            tr_param!(crate::eval_trace::passer_mg(relative_rank), -1, 0);
            tr_param!(crate::eval_trace::passer_eg(relative_rank), 0, -1);
            e_eval -= bonus_eg;
            o_eval -= bonus_mg;
        } else {
            // Candidate passed pawn: no opposing pawn on this file in front, and supported by friendly adjacent pawns
            let front_file_mask = (0x0101010101010101u64 << file) & ((1u64 << sq) - 1);
            let white_pawns = board.bitboards[crate::model::WHITE_PAWN];
            if (white_pawns & front_file_mask) == 0 && (2..=4).contains(&rank) {
                let adjacent_files = ADJACENT_FILES_MASK[file as usize];
                let friendly_adj_pawns = (black_pawns & adjacent_files).count_ones();
                // Ranks strictly ahead of this pawn; see the White counterpart for why this
                // is a rank predicate and not an index one.
                let ahead_of_rank = (1u64 << (rank * 8)) - 1;
                let enemy_adj_pawns = (white_pawns & adjacent_files & ahead_of_rank).count_ones();
                if friendly_adj_pawns >= enemy_adj_pawns {
                    let advancement = (5 - rank) as i16;
                    tr!(CandidatePassedPawnBonus, -(advancement as f64) / 2.0, -advancement);
                    o_eval -= (config.candidate_passed_pawn_bonus * advancement) / 2;
                    e_eval -= config.candidate_passed_pawn_bonus * advancement;
                }
            }
        }

        let adjacent_files = ADJACENT_FILES_MASK[file as usize];
        let is_isolated = (black_pawns & adjacent_files) == 0;
        if is_isolated {
            tr!(PawnIsolatedMalus, 1, 1);
            tr_const!(0, 15);
            o_eval += config.pawn_isolated_malus;
            e_eval += config.pawn_isolated_malus + 15;
        }

        let behind_and_same_rank_mask = !((1u64 << (rank * 8)) - 1);
        let is_backward = !is_isolated && (black_pawns & adjacent_files & behind_and_same_rank_mask) == 0;
        if is_backward {
            tr!(PawnBackwardMalus, 1, 1);
            tr_const!(0, 10);
            o_eval += config.pawn_backward_malus;
            e_eval += config.pawn_backward_malus + 10;
        }

        let has_phalanx = (file > 0 && ((1u64 << (sq - 1)) & black_pawns) != 0) ||
                          (file < 7 && ((1u64 << (sq + 1)) & black_pawns) != 0);
        if has_phalanx && (2..=4).contains(&rank) {
            let advancement = (5 - rank) as i16;
            tr!(PawnPhalanxMg, -advancement, 0);
            tr!(PawnPhalanxEg, 0, -advancement);
            o_eval -= config.pawn_phalanx_mg * advancement;
            e_eval -= config.pawn_phalanx_eg * advancement;
        }

        (o_eval, e_eval)
    }

    fn white_rook(&self, sq: u8, board: &Board, config: &Config, game_phase: i16, movegen: &MoveGenService, opp_king_ring: u64, opp_pawn_attacks: u64, friendly_pieces: u64) -> (i16, u8, i16, u64) {
        let mut o_eval = 0;
        let mut e_eval = 0;
        let rank = sq / 8;
        let file = sq % 8;
        let file_mask = 0x0101010101010101u64 << file;
        

        let has_white_pawns = (board.bitboards[crate::model::WHITE_PAWN] & file_mask) != 0;
        let has_black_pawns = (board.bitboards[crate::model::BLACK_PAWN] & file_mask) != 0;

        if !has_white_pawns {
            if !has_black_pawns {
                // Fully open file
                tr!(RookOpenFile, 1, 1);
                tr_const!(0, 10);
                o_eval += config.rook_open_file;
                e_eval += config.rook_open_file + 10;
            } else {
                // Half-open file
                tr!(RookHalfOpenFile, 1, 1);
                tr_const!(0, 5);
                o_eval += config.rook_half_open_file;
                e_eval += config.rook_half_open_file + 5;
            }

            let black_king = board.bitboards[crate::model::BLACK_KING];
            let black_queen = board.bitboards[crate::model::BLACK_QUEEN];
            if (black_king & file_mask) != 0 {
                tr!(RookOpenFileAttacksKing, 1, 0);
                o_eval += config.rook_open_file_attacks_king;
            }
            if (black_queen & file_mask) != 0 {
                tr!(RookOpenFileAttacksQueen, 1, 0);
                o_eval += config.rook_open_file_attacks_queen;
            }
        }

        let white_rooks = board.bitboards[crate::model::WHITE_ROOK];
        if (white_rooks & file_mask).count_ones() >= 2 {
            tr!(RookDoubledBonus, 1, 1);
            tr_const!(0, 10);
            o_eval += config.rook_doubled_bonus;
            e_eval += config.rook_doubled_bonus + 10;
        }

        // Rook behind passed pawn bonus
        let white_pawns = board.bitboards[crate::model::WHITE_PAWN];
        let mut file_pawns = white_pawns & file_mask;
        while file_pawns != 0 {
            let pawn_sq = file_pawns.trailing_zeros() as u8;
            if self.is_white_passed_pawn(pawn_sq, board) {
                let pawn_rank = pawn_sq / 8;
                if rank < pawn_rank {
                    tr!(RookBehindPassedPawnMiddlegame, 1, 0);
                    tr!(RookBehindPassedPawnEndgame, 0, 1);
                    o_eval += config.rook_behind_passed_pawn_middlegame;
                    e_eval += config.rook_behind_passed_pawn_endgame;
                }
            }
            file_pawns &= file_pawns - 1;
        }

        // Rook behind enemy passed pawn bonus
        let black_pawns = board.bitboards[crate::model::BLACK_PAWN];
        let mut file_black_pawns = black_pawns & file_mask;
        while file_black_pawns != 0 {
            let pawn_sq = file_black_pawns.trailing_zeros() as u8;
            if self.is_black_passed_pawn(pawn_sq, board) {
                let pawn_rank = pawn_sq / 8;
                if rank > pawn_rank {
                    tr!(RookBehindEnemyPassedPawnMg, 1, 0);
                    tr!(RookBehindEnemyPassedPawnEg, 0, 1);
                    o_eval += config.rook_behind_enemy_passed_pawn_mg;
                    e_eval += config.rook_behind_enemy_passed_pawn_eg;
                }
            }
            file_black_pawns &= file_black_pawns - 1;
        }

        // Rook on 7th Rank
        if rank == 6 {
            tr!(RookOnSeventh, 1, 1);
            tr_const!(0, 15);
            o_eval += config.rook_on_seventh;
            e_eval += config.rook_on_seventh + 15;

            // Enemy King cut off on 8th rank
            let black_king = board.bitboards[crate::model::BLACK_KING];
            if (black_king & 0xFF00000000000000u64) != 0 {
                tr!(RookOnSeventhKingCutoff, 0.5, 1);
                o_eval += config.rook_on_seventh_king_cutoff / 2;
                e_eval += config.rook_on_seventh_king_cutoff;
            }

            // Doubled rooks on the 7th rank (Pigs on the 7th)
            if (white_rooks & 0x00FF000000000000u64).count_ones() >= 2 {
                tr!(RooksDoubledOnSeventh, 1, 1);
                o_eval += config.rooks_doubled_on_seventh;
                e_eval += config.rooks_doubled_on_seventh;
            }
        }

        // Safe Rook mobility (excluding friendly blockers and squares attacked by enemy pawns)
        let attacks = movegen.get_rook_attacks(sq as usize, board.occupied);
        
        // Threat Matrix: Rook attacks Queen
        let attacked_black_queens = (attacks & board.bitboards[crate::model::BLACK_QUEEN]).count_ones() as i16;
        tr!(ThreatRookAttacksQueen, attacked_black_queens, 0);
        o_eval += attacked_black_queens * config.threat_rook_attacks_queen;
        let safe_mask = !(opp_pawn_attacks | friendly_pieces);
        let mobility = (attacks & safe_mask).count_ones() as i16;
        tr!(RookMobilityFactor, mobility, mobility);
        tr_const!(0, 3 * mobility);
        o_eval += mobility * config.rook_mobility_factor;
        e_eval += mobility * (config.rook_mobility_factor + 3);

        // Rook attacking/defending pawns in endgame
        let attacked_pawns = (attacks & board.bitboards[crate::model::BLACK_PAWN]).count_ones() as i16;
        let defended_pawns = (attacks & board.bitboards[crate::model::WHITE_PAWN]).count_ones() as i16;
        tr_const!(0, attacked_pawns * 15 + defended_pawns * 8);
        e_eval += attacked_pawns * 15;
        e_eval += defended_pawns * 8;

        // King ring attacks
        let attacks_on_ring = (attacks & opp_king_ring).count_ones() as i16;
        let attackers = if attacks_on_ring > 0 { 1 } else { 0 };
        let danger = attacks_on_ring * config.king_ring_attack_rook;

        let eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        (eval, attackers, danger, attacks)
    }

    fn black_rook(&self, sq: u8, board: &Board, config: &Config, game_phase: i16, movegen: &MoveGenService, opp_king_ring: u64, opp_pawn_attacks: u64, friendly_pieces: u64) -> (i16, u8, i16, u64) {
        let mut o_eval = 0;
        let mut e_eval = 0;
        let rank = sq / 8;
        let file = sq % 8;
        let file_mask = 0x0101010101010101u64 << file;
        

        let has_white_pawns = (board.bitboards[crate::model::WHITE_PAWN] & file_mask) != 0;
        let has_black_pawns = (board.bitboards[crate::model::BLACK_PAWN] & file_mask) != 0;

        if !has_black_pawns {
            if !has_white_pawns {
                // Fully open file
                tr!(RookOpenFile, -1, -1);
                tr_const!(0, -10);
                o_eval -= config.rook_open_file;
                e_eval -= config.rook_open_file + 10;
            } else {
                // Half-open file
                tr!(RookHalfOpenFile, -1, -1);
                tr_const!(0, -5);
                o_eval -= config.rook_half_open_file;
                e_eval -= config.rook_half_open_file + 5;
            }

            let white_king = board.bitboards[crate::model::WHITE_KING];
            let white_queen = board.bitboards[crate::model::WHITE_QUEEN];
            if (white_king & file_mask) != 0 {
                tr!(RookOpenFileAttacksKing, -1, 0);
                o_eval -= config.rook_open_file_attacks_king;
            }
            if (white_queen & file_mask) != 0 {
                tr!(RookOpenFileAttacksQueen, -1, 0);
                o_eval -= config.rook_open_file_attacks_queen;
            }
        }

        let black_rooks = board.bitboards[crate::model::BLACK_ROOK];
        if (black_rooks & file_mask).count_ones() >= 2 {
            tr!(RookDoubledBonus, -1, -1);
            tr_const!(0, -10);
            o_eval -= config.rook_doubled_bonus;
            e_eval -= config.rook_doubled_bonus + 10;
        }

        // Rook behind passed pawn bonus
        let black_pawns = board.bitboards[crate::model::BLACK_PAWN];
        let mut file_pawns = black_pawns & file_mask;
        while file_pawns != 0 {
            let pawn_sq = file_pawns.trailing_zeros() as u8;
            if self.is_black_passed_pawn(pawn_sq, board) {
                let pawn_rank = pawn_sq / 8;
                if rank > pawn_rank {
                    tr!(RookBehindPassedPawnMiddlegame, -1, 0);
                    tr!(RookBehindPassedPawnEndgame, 0, -1);
                    o_eval -= config.rook_behind_passed_pawn_middlegame;
                    e_eval -= config.rook_behind_passed_pawn_endgame;
                }
            }
            file_pawns &= file_pawns - 1;
        }

        // Rook behind enemy passed pawn bonus
        let white_pawns = board.bitboards[crate::model::WHITE_PAWN];
        let mut file_white_pawns = white_pawns & file_mask;
        while file_white_pawns != 0 {
            let pawn_sq = file_white_pawns.trailing_zeros() as u8;
            if self.is_white_passed_pawn(pawn_sq, board) {
                let pawn_rank = pawn_sq / 8;
                if rank < pawn_rank {
                    tr!(RookBehindEnemyPassedPawnMg, -1, 0);
                    tr!(RookBehindEnemyPassedPawnEg, 0, -1);
                    o_eval -= config.rook_behind_enemy_passed_pawn_mg;
                    e_eval -= config.rook_behind_enemy_passed_pawn_eg;
                }
            }
            file_white_pawns &= file_white_pawns - 1;
        }

        // Rook on 7th Rank (rank 1 for Black)
        if rank == 1 {
            tr!(RookOnSeventh, -1, -1);
            tr_const!(0, -15);
            o_eval -= config.rook_on_seventh;
            e_eval -= config.rook_on_seventh + 15;

            // Enemy King cut off on 1st rank
            let white_king = board.bitboards[crate::model::WHITE_KING];
            if (white_king & 0x00000000000000FFu64) != 0 {
                tr!(RookOnSeventhKingCutoff, -0.5, -1);
                o_eval -= config.rook_on_seventh_king_cutoff / 2;
                e_eval -= config.rook_on_seventh_king_cutoff;
            }

            // Doubled rooks on the 2nd rank
            if (black_rooks & 0x000000000000FF00u64).count_ones() >= 2 {
                tr!(RooksDoubledOnSeventh, -1, -1);
                o_eval -= config.rooks_doubled_on_seventh;
                e_eval -= config.rooks_doubled_on_seventh;
            }
        }

        // Safe Rook mobility (excluding friendly blockers and squares attacked by enemy pawns)
        let attacks = movegen.get_rook_attacks(sq as usize, board.occupied);
        
        // Threat Matrix: Rook attacks Queen
        let attacked_white_queens = (attacks & board.bitboards[crate::model::WHITE_QUEEN]).count_ones() as i16;
        tr!(ThreatRookAttacksQueen, -attacked_white_queens, 0);
        o_eval -= attacked_white_queens * config.threat_rook_attacks_queen;
        let safe_mask = !(opp_pawn_attacks | friendly_pieces);
        let mobility = (attacks & safe_mask).count_ones() as i16;
        tr!(RookMobilityFactor, -mobility, -mobility);
        tr_const!(0, -3 * mobility);
        o_eval -= mobility * config.rook_mobility_factor;
        e_eval -= mobility * (config.rook_mobility_factor + 3);

        // Rook attacking/defending pawns in endgame
        let attacked_pawns = (attacks & board.bitboards[crate::model::WHITE_PAWN]).count_ones() as i16;
        let defended_pawns = (attacks & board.bitboards[crate::model::BLACK_PAWN]).count_ones() as i16;
        tr_const!(0, -(attacked_pawns * 15 + defended_pawns * 8));
        e_eval -= attacked_pawns * 15;
        e_eval -= defended_pawns * 8;

        // King ring attacks
        let attacks_on_ring = (attacks & opp_king_ring).count_ones() as i16;
        let attackers = if attacks_on_ring > 0 { 1 } else { 0 };
        let danger = attacks_on_ring * config.king_ring_attack_rook;

        let eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        (eval, attackers, danger, attacks)
    }

    fn white_knight(&self, sq: u8, board: &Board, config: &Config, game_phase: i16, movegen: &MoveGenService, opp_king_ring: u64, friendly_true_outposts: u64, opp_pawn_attacks: u64, friendly_pieces: u64) -> (i16, u8, i16, u64) {
        let mut o_eval = 0;
        let mut e_eval = 0;
        let sq = sq as i32;
        let rank = sq / 8;
        let file = sq % 8;
        
    
        if rank == 0 || rank == 7 || file == 0 || file == 7 {
            tr!(KnightOnRimMalus, -1, 0);
            o_eval -= config.knight_on_rim_malus;
        }

        // Evaluate knight attacks on other pieces
        let attacks = movegen.get_knight_attacks(sq as usize);
        
        // Threat Matrix: Knight attacks Queen
        let attacked_black_queens = (attacks & board.bitboards[crate::model::BLACK_QUEEN]).count_ones() as i16;
        tr!(ThreatMinorAttacksQueen, attacked_black_queens, 0);
        o_eval += attacked_black_queens * config.threat_minor_attacks_queen;
        tr!(KnightAttacksRook, (attacks & board.bitboards[crate::model::BLACK_ROOK]).count_ones(), 0);
        tr!(KnightAttacksBishop, (attacks & board.bitboards[crate::model::BLACK_BISHOP]).count_ones(), 0);
        if board.white_to_move {
            tr!(KnightAttacksRookTempo, (attacks & board.bitboards[crate::model::BLACK_ROOK]).count_ones(), 0);
            tr!(KnightAttacksBishopTempo, (attacks & board.bitboards[crate::model::BLACK_BISHOP]).count_ones(), 0);
        }
        // Read from `config` at evaluation time. They were copied out of the defaults when the
        // service was built, so `setoption` and the tuner could not move them.
        for (target_bb_idx, bonus_simple, bonus_tempo) in [
            (crate::model::BLACK_ROOK, config.knight_attacks_rook, config.knight_attacks_rook_tempo),
            (crate::model::BLACK_BISHOP, config.knight_attacks_bishop, config.knight_attacks_bishop_tempo),
        ] {
            let count = (attacks & board.bitboards[target_bb_idx]).count_ones() as i16;
            o_eval += count * bonus_simple;
            if board.white_to_move {
                o_eval += count * bonus_tempo;
            }
        }

        // Safe Knight mobility (excluding friendly blockers and squares attacked by enemy pawns)
        let safe_mask = !(opp_pawn_attacks | friendly_pieces);
        let mobility = (attacks & safe_mask).count_ones() as i16;
        tr!(KnightMobilityFactor, mobility, mobility);
        o_eval += mobility * config.knight_mobility_factor;
        e_eval += mobility * config.knight_mobility_factor;

        let stands_on_outpost = (1u64 << sq) & friendly_true_outposts != 0;
        let attacks_outpost = (attacks & friendly_true_outposts) != 0;
        if stands_on_outpost || attacks_outpost {
            tr!(KnightOutpostTrueMg, 1, 0);
            tr!(KnightOutpostTrueEg, 0, 1);
            o_eval += config.knight_outpost_true_mg;
            e_eval += config.knight_outpost_true_eg;
        }

        let is_centered = (3..=5).contains(&rank) && (2..=5).contains(&file);
        if is_centered {
            tr!(KnightCentered, 0.5, 1);
            e_eval += config.knight_centered;
            o_eval += config.knight_centered / 2;
        }
    
        if sq == 1 || sq == 6 {
            tr!(UndevelopedKnightMalus, -1, 0);
            o_eval -= config.undeveloped_knight_malus;
        }

        if sq + 8 < 64 && ((1u64 << (sq + 8)) & board.bitboards[crate::model::BLACK_PAWN]) != 0 {
            tr!(KnightBlockesPawn, 0.5, 1);
            e_eval += config.knight_blockes_pawn;
            o_eval += config.knight_blockes_pawn / 2;
        }
            // King ring attacks
        let attacks_on_ring = (attacks & opp_king_ring).count_ones() as i16;
        let attackers = if attacks_on_ring > 0 { 1 } else { 0 };
        let danger = attacks_on_ring * config.king_ring_attack_knight;

        let eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        (eval, attackers, danger, attacks)
    }

    fn black_knight(&self, sq: u8, board: &Board, config: &Config, game_phase: i16, movegen: &MoveGenService, opp_king_ring: u64, friendly_true_outposts: u64, opp_pawn_attacks: u64, friendly_pieces: u64) -> (i16, u8, i16, u64) {
        let mut o_eval = 0;
        let mut e_eval = 0;
        let sq = sq as i32;
        let rank = sq / 8;
        let file = sq % 8;
        
    
        if rank == 0 || rank == 7 || file == 0 || file == 7 {
            tr!(KnightOnRimMalus, 1, 0);
            o_eval += config.knight_on_rim_malus;
        }
    
        // Evaluate knight attacks on other pieces
        let attacks = movegen.get_knight_attacks(sq as usize);
        
        // Threat Matrix: Knight attacks Queen
        let attacked_white_queens = (attacks & board.bitboards[crate::model::WHITE_QUEEN]).count_ones() as i16;
        tr!(ThreatMinorAttacksQueen, -attacked_white_queens, 0);
        o_eval -= attacked_white_queens * config.threat_minor_attacks_queen;
        tr!(KnightAttacksRook, -((attacks & board.bitboards[crate::model::WHITE_ROOK]).count_ones() as i32), 0);
        tr!(KnightAttacksBishop, -((attacks & board.bitboards[crate::model::WHITE_BISHOP]).count_ones() as i32), 0);
        if !board.white_to_move {
            tr!(KnightAttacksRookTempo, -((attacks & board.bitboards[crate::model::WHITE_ROOK]).count_ones() as i32), 0);
            tr!(KnightAttacksBishopTempo, -((attacks & board.bitboards[crate::model::WHITE_BISHOP]).count_ones() as i32), 0);
        }
        for (target_bb_idx, bonus_simple, bonus_tempo) in [
            (crate::model::WHITE_ROOK, config.knight_attacks_rook, config.knight_attacks_rook_tempo),
            (crate::model::WHITE_BISHOP, config.knight_attacks_bishop, config.knight_attacks_bishop_tempo),
        ] {
            let count = (attacks & board.bitboards[target_bb_idx]).count_ones() as i16;
            o_eval -= count * bonus_simple;
            if !board.white_to_move {
                o_eval -= count * bonus_tempo;
            }
        }

        // Safe Knight mobility (excluding friendly blockers and squares attacked by enemy pawns)
        let safe_mask = !(opp_pawn_attacks | friendly_pieces);
        let mobility = (attacks & safe_mask).count_ones() as i16;
        tr!(KnightMobilityFactor, -mobility, -mobility);
        o_eval -= mobility * config.knight_mobility_factor;
        e_eval -= mobility * config.knight_mobility_factor;

        let stands_on_outpost = (1u64 << sq) & friendly_true_outposts != 0;
        let attacks_outpost = (attacks & friendly_true_outposts) != 0;
        if stands_on_outpost || attacks_outpost {
            tr!(KnightOutpostTrueMg, -1, 0);
            tr!(KnightOutpostTrueEg, 0, -1);
            o_eval -= config.knight_outpost_true_mg;
            e_eval -= config.knight_outpost_true_eg;
        }

        let is_centered = (2..=4).contains(&rank) && (2..=5).contains(&file);
        if is_centered {
            tr!(KnightCentered, -0.5, -1);
            e_eval -= config.knight_centered;
            o_eval -= config.knight_centered / 2;
        }
    
        if sq == 57 || sq == 62 {
            tr!(UndevelopedKnightMalus, 1, 0);
            o_eval += config.undeveloped_knight_malus;
        }

        if sq >= 8 && ((1u64 << (sq - 8)) & board.bitboards[crate::model::WHITE_PAWN]) != 0 {
            tr!(KnightBlockesPawn, -0.5, -1);
            e_eval -= config.knight_blockes_pawn;
            o_eval -= config.knight_blockes_pawn / 2;
        }
            // King ring attacks
        let attacks_on_ring = (attacks & opp_king_ring).count_ones() as i16;
        let attackers = if attacks_on_ring > 0 { 1 } else { 0 };
        let danger = attacks_on_ring * config.king_ring_attack_knight;

        let eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        (eval, attackers, danger, attacks)
    }

    fn white_bishop(&self, sq: u8, board: &Board, config: &Config, game_phase: i16, movegen: &MoveGenService, opp_king_ring: u64, friendly_true_outposts: u64, opp_pawn_attacks: u64, friendly_pieces: u64) -> (i16, u8, i16, u64) {
        let mut o_eval = 0;
        let mut e_eval = 0;
        let sq = sq as i32;
        let file = sq % 8;
        let rank = sq / 8;
        

        if sq == 2 || sq == 5 {
            tr!(UndevelopedBishopMalus, -1, 0);
            o_eval -= config.undeveloped_bishop_malus;
        }

        if rank == 6 && file == 7 && ((1u64 << (sq - 9)) & board.occupied) != 0 {
            tr!(BishopTrappedAtRimMalus, -1, 0);
            o_eval -= config.bishop_trapped_at_rim_malus;
        }
        if rank == 6 && file == 0 && ((1u64 << (sq - 7)) & board.occupied) != 0 {
            tr!(BishopTrappedAtRimMalus, -1, 0);
            o_eval -= config.bishop_trapped_at_rim_malus;
        }

        // Safe Bishop mobility (excluding friendly blockers and squares attacked by enemy pawns)
        let attacks = movegen.get_bishop_attacks(sq as usize, board.occupied);
        
        // Threat Matrix: Bishop attacks Rook/Queen
        let attacked_black_rooks = (attacks & board.bitboards[crate::model::BLACK_ROOK]).count_ones() as i16;
        let attacked_black_queens = (attacks & board.bitboards[crate::model::BLACK_QUEEN]).count_ones() as i16;
        tr!(ThreatMinorAttacksRook, attacked_black_rooks, 0);
        tr!(ThreatMinorAttacksQueen, attacked_black_queens, 0);
        o_eval += attacked_black_rooks * config.threat_minor_attacks_rook;
        o_eval += attacked_black_queens * config.threat_minor_attacks_queen;
        let safe_mask = !(opp_pawn_attacks | friendly_pieces);
        let mobility = (attacks & safe_mask).count_ones() as i16;
        tr!(BishopMobilityFactor, mobility, mobility);
        o_eval += mobility * config.bishop_mobility_factor;
        e_eval += mobility * config.bishop_mobility_factor;

        let stands_on_outpost = (1u64 << sq) & friendly_true_outposts != 0;
        let attacks_outpost = (attacks & friendly_true_outposts) != 0;
        if stands_on_outpost || attacks_outpost {
            tr!(BishopOutpostTrueMg, 1, 0);
            tr!(BishopOutpostTrueEg, 0, 1);
            o_eval += config.bishop_outpost_true_mg;
            e_eval += config.bishop_outpost_true_eg;
        }

        // Bishop diagonal X-Ray alignment with enemy King or Queen on empty board ray
        let diag_ray = movegen.get_bishop_attacks(sq as usize, 0);
        let black_king = board.bitboards[crate::model::BLACK_KING];
        let black_queen = board.bitboards[crate::model::BLACK_QUEEN];
        if (diag_ray & black_king) != 0 {
            tr!(BishopDiagonalAttacksKing, 1, 0);
            o_eval += config.bishop_diagonal_attacks_king;
        }
        if (diag_ray & black_queen) != 0 {
            tr!(BishopDiagonalAttacksQueen, 1, 0);
            o_eval += config.bishop_diagonal_attacks_queen;
        }

        // King ring attacks
        let attacks_on_ring = (attacks & opp_king_ring).count_ones() as i16;
        let attackers = if attacks_on_ring > 0 { 1 } else { 0 };
        let danger = attacks_on_ring * config.king_ring_attack_bishop;

        let eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        (eval, attackers, danger, attacks)
    }

    fn black_bishop(&self, sq: u8, board: &Board, config: &Config, game_phase: i16, movegen: &MoveGenService, opp_king_ring: u64, friendly_true_outposts: u64, opp_pawn_attacks: u64, friendly_pieces: u64) -> (i16, u8, i16, u64) {
        let mut o_eval = 0;
        let mut e_eval = 0;
        let sq = sq as i32;
        let file = sq % 8;
        let rank = sq / 8;
        

        if sq == 58 || sq == 61 {
            tr!(UndevelopedBishopMalus, 1, 0);
            o_eval += config.undeveloped_bishop_malus;
        }

        if rank == 1 && file == 7 && ((1u64 << (sq + 7)) & board.occupied) != 0 {
            tr!(BishopTrappedAtRimMalus, 1, 0);
            o_eval += config.bishop_trapped_at_rim_malus;
        }
        if rank == 1 && file == 0 && ((1u64 << (sq + 9)) & board.occupied) != 0 {
            tr!(BishopTrappedAtRimMalus, 1, 0);
            o_eval += config.bishop_trapped_at_rim_malus;
        }

        // Safe Bishop mobility (excluding friendly blockers and squares attacked by enemy pawns)
        let attacks = movegen.get_bishop_attacks(sq as usize, board.occupied);
        
        // Threat Matrix: Bishop attacks Rook/Queen
        let attacked_white_rooks = (attacks & board.bitboards[crate::model::WHITE_ROOK]).count_ones() as i16;
        let attacked_white_queens = (attacks & board.bitboards[crate::model::WHITE_QUEEN]).count_ones() as i16;
        tr!(ThreatMinorAttacksRook, -attacked_white_rooks, 0);
        tr!(ThreatMinorAttacksQueen, -attacked_white_queens, 0);
        o_eval -= attacked_white_rooks * config.threat_minor_attacks_rook;
        o_eval -= attacked_white_queens * config.threat_minor_attacks_queen;
        let safe_mask = !(opp_pawn_attacks | friendly_pieces);
        let mobility = (attacks & safe_mask).count_ones() as i16;
        tr!(BishopMobilityFactor, -mobility, -mobility);
        o_eval -= mobility * config.bishop_mobility_factor;
        e_eval -= mobility * config.bishop_mobility_factor;

        let stands_on_outpost = (1u64 << sq) & friendly_true_outposts != 0;
        let attacks_outpost = (attacks & friendly_true_outposts) != 0;
        if stands_on_outpost || attacks_outpost {
            tr!(BishopOutpostTrueMg, -1, 0);
            tr!(BishopOutpostTrueEg, 0, -1);
            o_eval -= config.bishop_outpost_true_mg;
            e_eval -= config.bishop_outpost_true_eg;
        }

        // Bishop diagonal X-Ray alignment with enemy King or Queen on empty board ray
        let diag_ray = movegen.get_bishop_attacks(sq as usize, 0);
        let white_king = board.bitboards[crate::model::WHITE_KING];
        let white_queen = board.bitboards[crate::model::WHITE_QUEEN];
        if (diag_ray & white_king) != 0 {
            tr!(BishopDiagonalAttacksKing, -1, 0);
            o_eval -= config.bishop_diagonal_attacks_king;
        }
        if (diag_ray & white_queen) != 0 {
            tr!(BishopDiagonalAttacksQueen, -1, 0);
            o_eval -= config.bishop_diagonal_attacks_queen;
        }

        // King ring attacks
        let attacks_on_ring = (attacks & opp_king_ring).count_ones() as i16;
        let attackers = if attacks_on_ring > 0 { 1 } else { 0 };
        let danger = attacks_on_ring * config.king_ring_attack_bishop;

        let eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        (eval, attackers, danger, attacks)
    }

    fn white_queen(&self, sq: u8, board: &Board, config: &Config, game_phase: i16, movegen: &MoveGenService, opp_king_ring: u64, opp_pawn_attacks: u64, friendly_pieces: u64) -> (i16, u8, i16, u64) {
        let mut o_eval = 0;
        let mut e_eval = 0;

        let attackers_mask = movegen.get_attackers_mask_for_see(board, true, sq, board.occupied);
        let num_attackers = attackers_mask.count_ones() as i16;
        if num_attackers > 0 {
            tr!(QueenInAttack, -num_attackers, 0);
            tr!(QueenInAttackWithTempo, if !board.white_to_move { -1 } else { 0 }, 0);
            o_eval -= (config.queen_in_attack * num_attackers) + if !board.white_to_move { config.queen_in_attack_with_tempo } else { 0 };
        }

        // King ring attacks for queen
        let attacks = movegen.get_rook_attacks(sq as usize, board.occupied) | movegen.get_bishop_attacks(sq as usize, board.occupied);
        let attacks_on_ring = (attacks & opp_king_ring).count_ones() as i16;
        let attackers = if attacks_on_ring > 0 { 1 } else { 0 };
        let danger = attacks_on_ring * config.king_ring_attack_queen;

        // Safe Queen mobility (excluding friendly blockers and squares attacked by enemy pawns)
        let safe_mask = !(opp_pawn_attacks | friendly_pieces);
        let mobility = (attacks & safe_mask).count_ones() as i16;
        tr!(QueenMobilityFactor, mobility, mobility);
        o_eval += mobility * config.queen_mobility_factor;
        e_eval += mobility * config.queen_mobility_factor;

        let eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        (eval, attackers, danger, attacks)
    }

    fn black_queen(&self, sq: u8, board: &Board, config: &Config, game_phase: i16, movegen: &MoveGenService, opp_king_ring: u64, opp_pawn_attacks: u64, friendly_pieces: u64) -> (i16, u8, i16, u64) {
        let mut o_eval = 0;
        let mut e_eval = 0;

        let attackers_mask = movegen.get_attackers_mask_for_see(board, false, sq, board.occupied);
        let num_attackers = attackers_mask.count_ones() as i16;
        if num_attackers > 0 {
            tr!(QueenInAttack, num_attackers, 0);
            tr!(QueenInAttackWithTempo, if board.white_to_move { 1 } else { 0 }, 0);
            o_eval += (config.queen_in_attack * num_attackers) + if board.white_to_move { config.queen_in_attack_with_tempo } else { 0 };
        }

        // King ring attacks for queen
        let attacks = movegen.get_rook_attacks(sq as usize, board.occupied) | movegen.get_bishop_attacks(sq as usize, board.occupied);
        let attacks_on_ring = (attacks & opp_king_ring).count_ones() as i16;
        let attackers = if attacks_on_ring > 0 { 1 } else { 0 };
        let danger = attacks_on_ring * config.king_ring_attack_queen;

        // Safe Queen mobility (excluding friendly blockers and squares attacked by enemy pawns)
        let safe_mask = !(opp_pawn_attacks | friendly_pieces);
        let mobility = (attacks & safe_mask).count_ones() as i16;
        tr!(QueenMobilityFactor, -mobility, -mobility);
        o_eval -= mobility * config.queen_mobility_factor;
        e_eval -= mobility * config.queen_mobility_factor;

        let eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        (eval, attackers, danger, attacks)
    }
 
    /// `may_be_attacked` false is a promise that the White king is not in check, which spares
    /// the attacker scan; see [`Self::calc_eval_known_check`].
    fn white_king(&self, sq: u8, board: &Board, config: &Config, game_phase: i16, movegen: &MoveGenService, may_be_attacked: bool) -> (i16, u8, i16) {
        let mut o_eval = 0;
        let mut e_eval = 0;
        let sq = sq as i32;
        let rank = sq / 8;
        let file = sq % 8;
        
        let (pawn_shield_val, piece_shield_val) = if file <= 2 {
            (config.king_pawn_shield_queenside, config.king_piece_shield_queenside)
        } else if file >= 5 {
            (config.king_pawn_shield_kingside, config.king_piece_shield_kingside)
        } else {
            (config.king_pawn_shield, config.king_piece_shield)
        };

        if sq == 3 || sq == 4 || sq == 5 || sq == 11 || sq == 12 || sq == 13 {
            tr!(UndevelopedKingMalus, -1, 0);
            o_eval -= config.undeveloped_king_malus
        }

        let in_check = if may_be_attacked {
            movegen.get_attackers_mask(board, true, sq as u8, board.occupied).count_ones() as usize
        } else {
            0
        };
        if in_check == 1 {
            o_eval -= config.king_in_check_malus;
            e_eval -= config.king_in_check_malus;
        } else if in_check > 1 {
            o_eval -= config.king_in_double_check_malus;
            e_eval -= config.king_in_double_check_malus;
        }

        let white_pawns = board.bitboards[crate::model::WHITE_PAWN];
        let black_pawns = board.bitboards[crate::model::BLACK_PAWN];
        let file_mask = 0x0101010101010101u64 << file;
        if (file_mask & white_pawns) == 0 {
            let opp_heavy_on_file = (board.bitboards[crate::model::BLACK_ROOK] | board.bitboards[crate::model::BLACK_QUEEN]) & file_mask != 0;
            if (file_mask & black_pawns) == 0 {
                tr!(KingOpenFileMalus, -1, 0);
                o_eval -= config.king_open_file_malus;
                if opp_heavy_on_file {
                    tr!(KingOpenFileHeavyThreatMalus, -1, 0);
                    o_eval -= config.king_open_file_heavy_threat_malus;
                }
            } else {
                tr!(KingHalfOpenFileMalus, -1, 0);
                o_eval -= config.king_half_open_file_malus;
                if opp_heavy_on_file {
                    tr!(KingOpenFileHeavyThreatMalus, -0.5, 0);
                    o_eval -= config.king_open_file_heavy_threat_malus / 2;
                }
            }
        }
        if file > 0 && sq + 7 < 64 {
            let bit = 1u64 << (sq + 7);
            if (bit & white_pawns) != 0 { tr_param!(crate::eval_trace::king_shield(file).0, 1, 0); o_eval += pawn_shield_val; }
            else if sq + 15 < 64 && ((1u64 << (sq + 15)) & white_pawns) != 0 { tr_param!(crate::eval_trace::king_shield(file).0, 0.5, 0); o_eval += pawn_shield_val / 2; }
            else if (bit & board.white_pieces) != 0 { tr_param!(crate::eval_trace::king_shield(file).1, 1, 0); o_eval += piece_shield_val; }
        }
        if sq + 8 < 64 {
            let bit = 1u64 << (sq + 8);
            if (bit & white_pawns) != 0 { tr_param!(crate::eval_trace::king_shield(file).0, 1, 0); o_eval += pawn_shield_val; }
            else if sq + 16 < 64 && ((1u64 << (sq + 16)) & white_pawns) != 0 { tr_param!(crate::eval_trace::king_shield(file).0, 0.5, 0); o_eval += pawn_shield_val / 2; }
            else if (bit & board.white_pieces) != 0 { tr_param!(crate::eval_trace::king_shield(file).1, 1, 0); o_eval += piece_shield_val; }
        }
        if file < 7 && sq + 9 < 64 {
            let bit = 1u64 << (sq + 9);
            if (bit & white_pawns) != 0 { tr_param!(crate::eval_trace::king_shield(file).0, 1, 0); o_eval += pawn_shield_val; }
            else if sq + 17 < 64 && ((1u64 << (sq + 17)) & white_pawns) != 0 { tr_param!(crate::eval_trace::king_shield(file).0, 0.5, 0); o_eval += pawn_shield_val / 2; }
            else if (bit & board.white_pieces) != 0 { tr_param!(crate::eval_trace::king_shield(file).1, 1, 0); o_eval += piece_shield_val; }
        }

        if rank == 0 {
            tr!(KingTrappAtBaselineMalus, 0, -1);
            e_eval -= config.king_trapp_at_baseline_malus;
        }

        let eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        (eval, 0, 0)
    }

    /// See [`Self::white_king`] for `may_be_attacked`.
    fn black_king(&self, sq: u8, board: &Board, config: &Config, game_phase: i16, movegen: &MoveGenService, may_be_attacked: bool) -> (i16, u8, i16) {
        let mut o_eval = 0;
        let mut e_eval = 0;
        let sq = sq as i32;
        let rank = sq / 8;
        let file = sq % 8;
        
        let (pawn_shield_val, piece_shield_val) = if file <= 2 {
            (config.king_pawn_shield_queenside, config.king_piece_shield_queenside)
        } else if file >= 5 {
            (config.king_pawn_shield_kingside, config.king_piece_shield_kingside)
        } else {
            (config.king_pawn_shield, config.king_piece_shield)
        };

        if sq == 59 || sq == 60 || sq == 61 || sq == 51 || sq == 52 || sq == 53 {
            tr!(UndevelopedKingMalus, 1, 0);
            o_eval += config.undeveloped_king_malus
        }

        let in_check = if may_be_attacked {
            movegen.get_attackers_mask(board, false, sq as u8, board.occupied).count_ones() as usize
        } else {
            0
        };
        if in_check == 1 {
            o_eval += config.king_in_check_malus;
            e_eval += config.king_in_check_malus;
        } else if in_check > 1 {
            o_eval += config.king_in_double_check_malus;
            e_eval += config.king_in_double_check_malus;
        }

        let black_pawns = board.bitboards[crate::model::BLACK_PAWN];
        let white_pawns = board.bitboards[crate::model::WHITE_PAWN];
        let file_mask = 0x0101010101010101u64 << file;
        if (file_mask & black_pawns) == 0 {
            let opp_heavy_on_file = (board.bitboards[crate::model::WHITE_ROOK] | board.bitboards[crate::model::WHITE_QUEEN]) & file_mask != 0;
            if (file_mask & white_pawns) == 0 {
                tr!(KingOpenFileMalus, 1, 0);
                o_eval += config.king_open_file_malus;
                if opp_heavy_on_file {
                    tr!(KingOpenFileHeavyThreatMalus, 1, 0);
                    o_eval += config.king_open_file_heavy_threat_malus;
                }
            } else {
                tr!(KingHalfOpenFileMalus, 1, 0);
                o_eval += config.king_half_open_file_malus;
                if opp_heavy_on_file {
                    tr!(KingOpenFileHeavyThreatMalus, 0.5, 0);
                    o_eval += config.king_open_file_heavy_threat_malus / 2;
                }
            }
        }
        if file > 0 && sq >= 9 {
            let bit = 1u64 << (sq - 9);
            if (bit & black_pawns) != 0 { tr_param!(crate::eval_trace::king_shield(file).0, -1, 0); o_eval -= pawn_shield_val; }
            else if sq >= 17 && ((1u64 << (sq - 17)) & black_pawns) != 0 { tr_param!(crate::eval_trace::king_shield(file).0, -0.5, 0); o_eval -= pawn_shield_val / 2; }
            else if (bit & board.black_pieces) != 0 { tr_param!(crate::eval_trace::king_shield(file).1, -1, 0); o_eval -= piece_shield_val; }
        }
        if sq >= 8 {
            let bit = 1u64 << (sq - 8);
            if (bit & black_pawns) != 0 { tr_param!(crate::eval_trace::king_shield(file).0, -1, 0); o_eval -= pawn_shield_val; }
            else if sq >= 16 && ((1u64 << (sq - 16)) & black_pawns) != 0 { tr_param!(crate::eval_trace::king_shield(file).0, -0.5, 0); o_eval -= pawn_shield_val / 2; }
            else if (bit & board.black_pieces) != 0 { tr_param!(crate::eval_trace::king_shield(file).1, -1, 0); o_eval -= piece_shield_val; }
        }
        if file < 7 && sq >= 7 {
            let bit = 1u64 << (sq - 7);
            if (bit & black_pawns) != 0 { tr_param!(crate::eval_trace::king_shield(file).0, -1, 0); o_eval -= pawn_shield_val; }
            else if sq >= 15 && ((1u64 << (sq - 15)) & black_pawns) != 0 { tr_param!(crate::eval_trace::king_shield(file).0, -0.5, 0); o_eval -= pawn_shield_val / 2; }
            else if (bit & board.black_pieces) != 0 { tr_param!(crate::eval_trace::king_shield(file).1, -1, 0); o_eval -= piece_shield_val; }
        }

        if rank == 7 {
            tr!(KingTrappAtBaselineMalus, 0, 1);
            e_eval += config.king_trapp_at_baseline_malus;
        }

        let eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        (eval, 0, 0)
    }

    /// The king-safety penalty `(mg, eg)` of the king of `us` (0 White, 1 Black), positive is bad
    /// for `us`. Ethereal's form: a weighted sum `u` of the attackers by type, the ring squares
    /// they hit (scaled to a ring of nine), the weak ring squares, the safe checks by type, a
    /// discount without the enemy queen and a constant; `u^2 / 720` in the middlegame and `u / 20`
    /// in the endgame, each only where positive. It reads only when the enemy has more than one
    /// piece on the ring, or one beside its queen.
    #[allow(clippy::too_many_arguments)]
    fn king_safety(&self, board: &Board, config: &Config, movegen: &MoveGenService, maps: &AttackMaps, us: usize,
        king_sq: u8, ring: u64, attack: &RingAttack) -> (i16, i16) {
        use crate::model::{BLACK_QUEEN, WHITE_BISHOP, WHITE_KING, WHITE_KNIGHT, WHITE_QUEEN, WHITE_ROOK};
        let them = 1 - us;
        let has_queen = board.bitboards[if them == 0 { WHITE_QUEEN } else { BLACK_QUEEN }] != 0;
        if attack.count <= 1 - has_queen as i32 {
            return (0, 0);
        }
        let their_pieces = if them == 0 { board.white_pieces } else { board.black_pieces };
        // Attacked by them, defended at most once by us and then only by the king or the queen.
        let weak = maps.all[them] & !maps.two[us] & (!maps.all[us] | maps.by[us][WHITE_QUEEN] | maps.by[us][WHITE_KING]);
        let safe = !their_pieces & (!maps.all[us] | (weak & maps.two[them]));
        let knight_checks = movegen.get_knight_attacks(king_sq as usize);
        let bishop_checks = movegen.get_bishop_attacks(king_sq as usize, board.occupied);
        let rook_checks = movegen.get_rook_attacks(king_sq as usize, board.occupied);
        let features: [i32; 12] = [
            attack.by_type[0],
            attack.by_type[1],
            attack.by_type[2],
            attack.by_type[3],
            0, // the ring hits, scaled below
            (weak & ring).count_ones() as i32,
            !has_queen as i32,
            (knight_checks & safe & maps.by[them][WHITE_KNIGHT]).count_ones() as i32,
            (bishop_checks & safe & maps.by[them][WHITE_BISHOP]).count_ones() as i32,
            (rook_checks & safe & maps.by[them][WHITE_ROOK]).count_ones() as i32,
            ((bishop_checks | rook_checks) & safe & maps.by[them][WHITE_QUEEN]).count_ones() as i32,
            1,
        ];
        let weights = [
            (config.king_safety_knight_attacker_mg, config.king_safety_knight_attacker_eg),
            (config.king_safety_bishop_attacker_mg, config.king_safety_bishop_attacker_eg),
            (config.king_safety_rook_attacker_mg, config.king_safety_rook_attacker_eg),
            (config.king_safety_queen_attacker_mg, config.king_safety_queen_attacker_eg),
            (config.king_safety_ring_attacks_mg, config.king_safety_ring_attacks_eg),
            (config.king_safety_weak_squares_mg, config.king_safety_weak_squares_eg),
            (config.king_safety_no_queen_mg, config.king_safety_no_queen_eg),
            (config.king_safety_safe_knight_check_mg, config.king_safety_safe_knight_check_eg),
            (config.king_safety_safe_bishop_check_mg, config.king_safety_safe_bishop_check_eg),
            (config.king_safety_safe_rook_check_mg, config.king_safety_safe_rook_check_eg),
            (config.king_safety_safe_queen_check_mg, config.king_safety_safe_queen_check_eg),
            (config.king_safety_adjustment_mg, config.king_safety_adjustment_eg),
        ];
        let area = ring.count_ones() as i32;
        let (mut u_mg, mut u_eg) = (0i32, 0i32);
        for (i, (&f, &(w_mg, w_eg))) in features.iter().zip(weights.iter()).enumerate() {
            if i == 4 {
                u_mg += w_mg as i32 * 9 * attack.hits / area;
                u_eg += w_eg as i32 * 9 * attack.hits / area;
                tr_safety!(us, crate::eval_trace::KING_SAFETY[4].0 as usize, crate::eval_trace::KING_SAFETY[4].1 as usize,
                    9.0 * attack.hits as f64 / area as f64);
            } else if f != 0 {
                u_mg += w_mg as i32 * f;
                u_eg += w_eg as i32 * f;
                tr_safety!(us, crate::eval_trace::KING_SAFETY[i].0 as usize, crate::eval_trace::KING_SAFETY[i].1 as usize, f);
            }
        }
        let (m, e) = (u_mg.max(0), u_eg.max(0));
        let mg = (m * m / crate::eval_trace::SAFETY_MG_DIVISOR).min(i16::MAX as i32 / 4);
        let eg = (e / crate::eval_trace::SAFETY_EG_DIVISOR).min(i16::MAX as i32 / 4);
        (mg as i16, eg as i16)
    }

    /// Threats on the attack maps, White-positive and tapered: enemy pieces we attack and they
    /// do not defend (hanging), minors attacked by our pawns, rooks by our pawns or minors, the
    /// queen by anything lesser, and the pieces our pawns would attack after a safe push. Kings
    /// and pawns are never victims. Counted per victim, which the older per-attacker pairs are not.
    fn threats(&self, board: &Board, config: &Config, maps: &AttackMaps, game_phase: i16) -> i16 {
        let w = Self::threat_features(board, maps, 0);
        let b = Self::threat_features(board, maps, 1);
        let weights = [
            (config.threat_hanging_mg, config.threat_hanging_eg),
            (config.threat_lesser_on_minor_mg, config.threat_lesser_on_minor_eg),
            (config.threat_lesser_on_rook_mg, config.threat_lesser_on_rook_eg),
            (config.threat_lesser_on_queen_mg, config.threat_lesser_on_queen_eg),
            (config.threat_safe_pawn_push_mg, config.threat_safe_pawn_push_eg),
        ];
        let (mut mg, mut eg) = (0i16, 0i16);
        for (i, &(w_mg, w_eg)) in weights.iter().enumerate() {
            let diff = w[i] - b[i];
            if diff != 0 {
                tr_param!(crate::eval_trace::THREATS[i].0 as usize, diff, 0);
                tr_param!(crate::eval_trace::THREATS[i].1 as usize, 0, diff);
                mg += diff * w_mg;
                eg += diff * w_eg;
            }
        }
        self.calculate_weighted_eval(mg, eg, game_phase)
    }

    /// The threat counts of `us` (0 White, 1 Black) in the order of [`Self::threats`].
    #[inline(always)]
    pub(crate) fn threat_features(board: &Board, maps: &AttackMaps, us: usize) -> [i16; 5] {
        use crate::model::{WHITE_BISHOP, WHITE_KNIGHT, WHITE_PAWN, WHITE_QUEEN, WHITE_ROOK};
        let them = 1 - us;
        let off = if them == 0 { 0 } else { 6 };
        let bb = &board.bitboards;
        let (minors, rooks, queens) = (bb[off + WHITE_KNIGHT] | bb[off + WHITE_BISHOP], bb[off + WHITE_ROOK], bb[off + WHITE_QUEEN]);
        let pieces = minors | rooks | queens;
        let by = &maps.by[us];
        let hanging = pieces & maps.all[us] & !maps.all[them];
        let on_minor = minors & by[WHITE_PAWN];
        let on_rook = rooks & (by[WHITE_PAWN] | by[WHITE_KNIGHT] | by[WHITE_BISHOP]);
        let on_queen = queens & (by[WHITE_PAWN] | by[WHITE_KNIGHT] | by[WHITE_BISHOP] | by[WHITE_ROOK]);
        let empty = !board.occupied;
        let our_pawns = bb[if us == 0 { WHITE_PAWN } else { crate::model::BLACK_PAWN }];
        let pushes = if us == 0 {
            let one = (our_pawns << 8) & empty;
            one | (((one & 0x0000_0000_00FF_0000) << 8) & empty)
        } else {
            let one = (our_pawns >> 8) & empty;
            one | (((one & 0x0000_FF00_0000_0000) >> 8) & empty)
        };
        let safe_pushes = pushes & !maps.by[them][WHITE_PAWN] & (maps.all[us] | !maps.all[them]);
        let push_attacks = if us == 0 { Self::get_white_pawn_attacks(safe_pushes) } else { Self::get_black_pawn_attacks(safe_pushes) };
        [
            hanging.count_ones() as i16,
            on_minor.count_ones() as i16,
            on_rook.count_ones() as i16,
            on_queen.count_ones() as i16,
            (push_attacks & pieces).count_ones() as i16,
        ]
    }

    fn calculate_weighted_eval(&self, o_eval: i16, e_eval: i16, game_phase: i16) -> i16 {
        let o_eval = o_eval as i32;
        let e_eval = e_eval as i32;
        let game_phase = game_phase as i32;
        let res = ((o_eval * game_phase) + (e_eval * (256 - game_phase))) / 256;
        debug_assert!(res < 32_767 && res > -32_767);
        tr_unit!(o_eval, e_eval, res);
        res as i16
    }

    /// The material part of the evaluation, White-positive, tapered like every other term: the
    /// `MATERIAL_MG` / `MATERIAL_EG` values of `pst.rs` that are also summed into the piece-square
    /// totals. The positional cap measures the rest of the evaluation against it. The kings cancel.
    fn tapered_material(board: &Board, game_phase: i16) -> i16 {
        let (mut mg, mut eg) = (0i32, 0i32);
        for piece in crate::model::WHITE_PAWN..crate::model::WHITE_KING {
            let diff = board.bitboards[piece].count_ones() as i32
                - board.bitboards[crate::model::BLACK_PAWN + piece].count_ones() as i32;
            mg += diff * crate::pst::MATERIAL_MG[piece] as i32;
            eg += diff * crate::pst::MATERIAL_EG[piece] as i32;
        }
        ((mg * game_phase as i32 + eg * (256 - game_phase as i32)) / 256) as i16
    }

    /// return Value of 255 means early game and values towards 0 means endgamephase
    /// a middle value like 128 respects early and late game evaluation in the same weight
    /// All with 6 or less pieces is considered pure endgame
    /// The game phase 0..=255 the evaluation tapers with (`texel-lazy` reads it for the lazy test).
    #[cfg(feature = "texel")]
    pub fn game_phase(&self, board: &Board) -> u32 {
        self.get_game_phase(board)
    }

    fn get_game_phase(&self, board: &Board) -> u32 {
        let knights = (board.bitboards[crate::model::WHITE_KNIGHT] | board.bitboards[crate::model::BLACK_KNIGHT]).count_ones();
        let bishops = (board.bitboards[crate::model::WHITE_BISHOP] | board.bitboards[crate::model::BLACK_BISHOP]).count_ones();
        let rooks = (board.bitboards[crate::model::WHITE_ROOK] | board.bitboards[crate::model::BLACK_ROOK]).count_ones();
        let queens = (board.bitboards[crate::model::WHITE_QUEEN] | board.bitboards[crate::model::BLACK_QUEEN]).count_ones();
        
        let total_phase = knights + bishops + rooks * 2 + queens * 4;
        // Max phase is usually 24 (4 knights/bishops + 4 rooks + 2 queens = 4 + 8 + 8 = 20... wait, 4*1 + 4*1 + 4*2 + 2*4 = 4+4+8+8 = 24).
        // If promotions occurred, it could be higher, we cap at 24 for the calculation.
        let phase = std::cmp::min(total_phase, 24);
        (phase * 255) / 24
    }

    /// adjust eval when exchange pieces with advantage
    fn adjust_eval(&self, eval: i16, game_phase: i16, config: &Config) -> i16 {
        if game_phase + 100 < 255 && (eval <= -200 || eval >= 200) {
            let mut mult: f32 = 255_f32 / (game_phase + 100) as f32;
            assert!(mult >= 1.0);
            if mult > config.max_eval_mult {
                mult = config.max_eval_mult;
            }
            let eval_f32 = eval as f32 * mult;
            if eval_f32 > i16::MAX.into() {
                return i16::MAX;
            }
            else if -eval_f32 > i16::MAX.into() {
                return -i16::MAX;
            } else {
                return eval_f32 as i16;
            }
        }
        eval        
    }

    /// Squares on ranks 4 to 6 (White) or 3 to 5 (Black) that a friendly pawn defends and that no
    /// enemy pawn can ever attack: no enemy pawn stands on an adjacent file in front of them. The
    /// set an enemy pawn can ever attack is its attack span, its current attacks filled towards
    /// its own back rank, so that one fill replaces the per-square scan over adjacent files.
    #[inline(always)]
    fn true_outposts(white_pawns: u64, black_pawns: u64, white: bool) -> u64 {
        const FILE_A: u64 = 0x0101010101010101;
        const FILE_H: u64 = 0x8080808080808080;
        if white {
            let supported = ((white_pawns << 9) & !FILE_A) | ((white_pawns << 7) & !FILE_H);
            let mut span = Self::get_black_pawn_attacks(black_pawns);
            span |= span >> 8;
            span |= span >> 16;
            span |= span >> 32;
            supported & 0x0000FFFFFF000000u64 & !span
        } else {
            let supported = ((black_pawns >> 9) & !FILE_H) | ((black_pawns >> 7) & !FILE_A);
            let mut span = Self::get_white_pawn_attacks(white_pawns);
            span |= span << 8;
            span |= span << 16;
            span |= span << 32;
            supported & 0x000000FFFFFF0000u64 & !span
        }
    }

    /// The dynamic pawn terms of both sides as `(mg, eg)`: pawns defending a knight outpost or a
    /// bishop, pawns attacking a piece, the king distance of passed pawns and the pawn storm.
    ///
    /// Every term but the king distance is the same constant for each pawn that qualifies, so it
    /// is a population count over a set instead of a call per pawn; `i16` addition wraps the same
    /// way in either order, which keeps the sums identical to the per-pawn form.
    /// How many enemy pieces other than pawns, the king included, a pawn of each side attacks.
    /// Set-wise, so `cheap_eval` can afford it too.
    #[inline(always)]
    fn pawn_threat_counts(board: &Board) -> (i16, i16) {
        use crate::model::*;
        const FILE_A: u64 = 0x0101010101010101;
        const FILE_H: u64 = 0x8080808080808080;
        let bb = &board.bitboards;
        let black_pieces = bb[BLACK_ROOK] | bb[BLACK_KNIGHT] | bb[BLACK_BISHOP] | bb[BLACK_QUEEN] | bb[BLACK_KING];
        let white_pieces = bb[WHITE_ROOK] | bb[WHITE_KNIGHT] | bb[WHITE_BISHOP] | bb[WHITE_QUEEN] | bb[WHITE_KING];
        let w_attacks = ((((black_pieces >> 9) & !FILE_H) | ((black_pieces >> 7) & !FILE_A)) & bb[WHITE_PAWN]).count_ones() as i16;
        let b_attacks = ((((white_pieces << 9) & !FILE_A) | ((white_pieces << 7) & !FILE_H)) & bb[BLACK_PAWN]).count_ones() as i16;
        (w_attacks, b_attacks)
    }

    /// The pawn-threat term as `(mg, eg)`: the middlegame weight plus the tempo for the side to
    /// move, half the weight in the endgame. One formula for the full evaluation and `cheap_eval`.
    #[inline(always)]
    fn pawn_threat_scores(board: &Board, config: &Config, w_attacks: i16, b_attacks: i16) -> (i16, i16) {
        let tempo = config.pawn_attacks_opponent_fig_with_tempo;
        let mg = w_attacks * (config.pawn_attacks_opponent_fig + if board.white_to_move { tempo } else { 0 })
            - b_attacks * (config.pawn_attacks_opponent_fig + if !board.white_to_move { tempo } else { 0 });
        let eg = (w_attacks - b_attacks) * (config.pawn_attacks_opponent_fig / 2);
        (mg, eg)
    }

    fn pawn_dynamic_scores(board: &Board, config: &Config, white_passed: u64, black_passed: u64) -> (i16, i16) {
        use crate::model::{WHITE_PAWN, WHITE_KNIGHT, WHITE_BISHOP, WHITE_KING,
            BLACK_PAWN, BLACK_KNIGHT, BLACK_BISHOP, BLACK_KING};
        const FILE_A: u64 = 0x0101010101010101;
        const FILE_H: u64 = 0x8080808080808080;
        let bb = &board.bitboards;
        let wp = bb[WHITE_PAWN];
        let bp = bb[BLACK_PAWN];
        let mut mg: i16 = 0;
        let mut eg: i16 = 0;

        // A pawn on rank 3 to 5 with a friendly knight diagonally in front of it.
        let w_knights = (((bb[WHITE_KNIGHT] >> 7) & !FILE_A) | ((bb[WHITE_KNIGHT] >> 9) & !FILE_H))
            & wp & 0x000000FFFFFF0000u64;
        let b_knights = (((bb[BLACK_KNIGHT] << 9) & !FILE_A) | ((bb[BLACK_KNIGHT] << 7) & !FILE_H))
            & bp & 0x0000FFFFFF000000u64;
        tr!(PawnSupportsKnightOutpost, w_knights.count_ones() as i32 - b_knights.count_ones() as i32, 0);
        mg += (w_knights.count_ones() as i16 - b_knights.count_ones() as i16) * config.pawn_supports_knight_outpost;

        // A pawn with a friendly bishop diagonally in front of it.
        let w_bishops = (((bb[WHITE_BISHOP] >> 9) & !FILE_H) | ((bb[WHITE_BISHOP] >> 7) & !FILE_A)) & wp;
        let b_bishops = (((bb[BLACK_BISHOP] << 9) & !FILE_A) | ((bb[BLACK_BISHOP] << 7) & !FILE_H)) & bp;
        tr!(PawnDefendsBishop, 0, w_bishops.count_ones() as i32 - b_bishops.count_ones() as i32);
        eg += (w_bishops.count_ones() as i16 - b_bishops.count_ones() as i16) * config.pawn_defends_bishop;

        // A pawn attacking an enemy piece other than a pawn, the king included.
        let (w_attacks, b_attacks) = Self::pawn_threat_counts(board);
        tr!(PawnAttacksOpponentFig, w_attacks - b_attacks, (w_attacks - b_attacks) as f64 / 2.0);
        tr!(PawnAttacksOpponentFigWithTempo, if board.white_to_move { w_attacks } else { -b_attacks }, 0);
        let (threat_mg, threat_eg) = Self::pawn_threat_scores(board, config, w_attacks, b_attacks);
        mg += threat_mg;
        eg += threat_eg;

        // The king distance of each passed pawn.
        let white_king_sq = bb[WHITE_KING].trailing_zeros() as u8;
        let black_king_sq = bb[BLACK_KING].trailing_zeros() as u8;
        let mut passers = white_passed & wp;
        while passers != 0 {
            let sq = passers.trailing_zeros() as u8;
            tr!(KingPasserDistWeight, 0, crate::eval_trace::king_passer_coefficient(sq, true, white_king_sq, black_king_sq));
            eg += Self::king_passer_proximity_score(sq, true, white_king_sq, black_king_sq, config);
            passers &= passers - 1;
        }
        let mut passers = black_passed & bp;
        while passers != 0 {
            let sq = passers.trailing_zeros() as u8;
            tr!(KingPasserDistWeight, 0, -crate::eval_trace::king_passer_coefficient(sq, false, white_king_sq, black_king_sq));
            eg -= Self::king_passer_proximity_score(sq, false, white_king_sq, black_king_sq, config);
            passers &= passers - 1;
        }

        // The pawn storm on the wing of an enemy king that stands on its own back two ranks: a
        // pawn on that wing (files a to c, or f to h) gains with every rank it has advanced.
        const WING_QUEEN: u64 = 0x0707070707070707;
        const WING_KING: u64 = 0xE0E0E0E0E0E0E0E0;
        let (bk_file, bk_rank) = ((black_king_sq % 8) as i32, (black_king_sq / 8) as i32);
        if bk_rank >= 5 {
            let wing = if bk_file >= 5 { WING_KING } else if bk_file <= 2 { WING_QUEEN } else { 0 };
            let mut storm = wp & wing & 0xFFFFFFFFFF000000u64;
            while storm != 0 {
                let rank = (storm.trailing_zeros() / 8) as i32;
                tr!(PawnStormBonus, rank - 2, 0);
                mg += config.pawn_storm_bonus * (rank - 2) as i16;
                storm &= storm - 1;
            }
        }
        let (wk_file, wk_rank) = ((white_king_sq % 8) as i32, (white_king_sq / 8) as i32);
        if wk_rank <= 2 {
            let wing = if wk_file >= 5 { WING_KING } else if wk_file <= 2 { WING_QUEEN } else { 0 };
            let mut storm = bp & wing & 0x000000FFFFFFFFFFu64;
            while storm != 0 {
                let rank = (storm.trailing_zeros() / 8) as i32;
                tr!(PawnStormBonus, -(5 - rank), 0);
                mg -= config.pawn_storm_bonus * (5 - rank) as i16;
                storm &= storm - 1;
            }
        }

        (mg, eg)
    }

    #[inline(always)]
    pub fn get_white_pawn_attacks(white_pawns: u64) -> u64 {
        ((white_pawns & !0x0101010101010101u64) << 7) | ((white_pawns & !0x8080808080808080u64) << 9)
    }

    #[inline(always)]
    pub fn get_black_pawn_attacks(black_pawns: u64) -> u64 {
        ((black_pawns & !0x8080808080808080u64) >> 7) | ((black_pawns & !0x0101010101010101u64) >> 9)
    }

    #[inline(always)]
    pub fn king_passer_proximity_score(
        sq: u8,
        is_white: bool,
        wk_sq: u8,
        bk_sq: u8,
        config: &Config,
    ) -> i16 {
        if config.king_passer_dist_weight == 0 {
            return 0;
        }
        let p_rank = (sq / 8) as i32;
        let p_file = (sq % 8) as i32;
        let wk_rank = (wk_sq / 8) as i32;
        let wk_file = (wk_sq % 8) as i32;
        let bk_rank = (bk_sq / 8) as i32;
        let bk_file = (bk_sq % 8) as i32;

        let w_dist = (p_rank - wk_rank).abs().max((p_file - wk_file).abs());
        let b_dist = (p_rank - bk_rank).abs().max((p_file - bk_file).abs());

        let rank_factor = if is_white { p_rank + 1 } else { 8 - p_rank };
        let delta_dist = if is_white { b_dist - w_dist } else { w_dist - b_dist };
        ((delta_dist * (config.king_passer_dist_weight as i32) * rank_factor) / 8) as i16
    }

    #[inline(always)]
    fn is_white_passed_pawn(&self, sq: u8, board: &Board) -> bool {
        (board.bitboards[crate::model::BLACK_PAWN] & WHITE_PASSED_PAWN_MASKS[sq as usize]) == 0
    }

    #[inline(always)]
    fn is_black_passed_pawn(&self, sq: u8, board: &Board) -> bool {
        (board.bitboards[crate::model::WHITE_PAWN] & BLACK_PASSED_PAWN_MASKS[sq as usize]) == 0
    }

    #[inline(always)]
    fn get_king_ring(&self, king_sq: u8) -> u64 {
        KING_RING_MASKS[king_sq as usize]
    }

    /// The per-square form of [`Self::true_outposts`], kept as the reference its tests compare
    /// against.
    #[cfg(test)]
    fn is_true_outpost(&self, sq: u8, is_white: bool, board: &Board) -> bool {
        let file = (sq % 8) as i32;
        let rank = (sq / 8) as i32;
        
        if is_white {
            if !(3..=5).contains(&rank) {
                return false;
            }
            let white_pawns = board.bitboards[crate::model::WHITE_PAWN];
            let mut supported = false;
            if file > 0 {
                let support_sq = (rank - 1) * 8 + (file - 1);
                if (white_pawns & (1u64 << support_sq)) != 0 {
                    supported = true;
                }
            }
            if file < 7 {
                let support_sq = (rank - 1) * 8 + (file + 1);
                if (white_pawns & (1u64 << support_sq)) != 0 {
                    supported = true;
                }
            }
            if !supported {
                return false;
            }
            let black_pawns = board.bitboards[crate::model::BLACK_PAWN];
            if file > 0 {
                let adjacent_file_mask = 0x0101010101010101u64 << (file - 1);
                let threat_mask = adjacent_file_mask & !((1u64 << ((rank + 1) * 8)) - 1);
                if (black_pawns & threat_mask) != 0 {
                    return false;
                }
            }
            if file < 7 {
                let adjacent_file_mask = 0x0101010101010101u64 << (file + 1);
                let threat_mask = adjacent_file_mask & !((1u64 << ((rank + 1) * 8)) - 1);
                if (black_pawns & threat_mask) != 0 {
                    return false;
                }
            }
            true
        } else {
            if !(2..=4).contains(&rank) {
                return false;
            }
            let black_pawns = board.bitboards[crate::model::BLACK_PAWN];
            let mut supported = false;
            if file > 0 {
                let support_sq = (rank + 1) * 8 + (file - 1);
                if (black_pawns & (1u64 << support_sq)) != 0 {
                    supported = true;
                }
            }
            if file < 7 {
                let support_sq = (rank + 1) * 8 + (file + 1);
                if (black_pawns & (1u64 << support_sq)) != 0 {
                    supported = true;
                }
            }
            if !supported {
                return false;
            }
            let white_pawns = board.bitboards[crate::model::WHITE_PAWN];
            if file > 0 {
                let adjacent_file_mask = 0x0101010101010101u64 << (file - 1);
                let threat_mask = adjacent_file_mask & ((1u64 << (rank * 8)) - 1);
                if (white_pawns & threat_mask) != 0 {
                    return false;
                }
            }
            if file < 7 {
                let adjacent_file_mask = 0x0101010101010101u64 << (file + 1);
                let threat_mask = adjacent_file_mask & ((1u64 << (rank * 8)) - 1);
                if (white_pawns & threat_mask) != 0 {
                    return false;
                }
            }
            true
        }
    }

    fn is_opposite_colored_bishops_endgame(board: &Board) -> bool {
        let w_bishops = board.bitboards[crate::model::WHITE_BISHOP];
        let b_bishops = board.bitboards[crate::model::BLACK_BISHOP];
        
        if w_bishops.count_ones() == 1 && b_bishops.count_ones() == 1 {
            let w_others = board.bitboards[crate::model::WHITE_KNIGHT] |
                           board.bitboards[crate::model::WHITE_ROOK] |
                           board.bitboards[crate::model::WHITE_QUEEN];
            let b_others = board.bitboards[crate::model::BLACK_KNIGHT] |
                           board.bitboards[crate::model::BLACK_ROOK] |
                           board.bitboards[crate::model::BLACK_QUEEN];
            
            if w_others == 0 && b_others == 0 {
                let w_sq = w_bishops.trailing_zeros() as i32;
                let b_sq = b_bishops.trailing_zeros() as i32;
                
                let w_color = (w_sq % 8 + w_sq / 8) % 2;
                let b_color = (b_sq % 8 + b_sq / 8) % 2;
                
                return w_color != b_color;
            }
        }
        false
    }

    #[inline(always)]
    pub fn is_insufficient_material(board: &Board) -> bool {
        let white_pawns = board.bitboards[crate::model::WHITE_PAWN];
        let black_pawns = board.bitboards[crate::model::BLACK_PAWN];
        let w_pawn_count = white_pawns.count_ones();
        let b_pawn_count = black_pawns.count_ones();
        let total_pawns = w_pawn_count + b_pawn_count;

        // Fast path: multiple pawns on board -> not insufficient material
        if total_pawns > 1 {
            return false;
        }

        let w_rooks = board.bitboards[crate::model::WHITE_ROOK];
        let w_queens = board.bitboards[crate::model::WHITE_QUEEN];
        let b_rooks = board.bitboards[crate::model::BLACK_ROOK];
        let b_queens = board.bitboards[crate::model::BLACK_QUEEN];

        if (w_rooks | w_queens | b_rooks | b_queens) != 0 {
            return false;
        }

        let w_knights = board.bitboards[crate::model::WHITE_KNIGHT].count_ones();
        let w_bishops = board.bitboards[crate::model::WHITE_BISHOP].count_ones();
        let b_knights = board.bitboards[crate::model::BLACK_KNIGHT].count_ones();
        let b_bishops = board.bitboards[crate::model::BLACK_BISHOP].count_ones();

        let w_minors = w_knights + w_bishops;
        let b_minors = b_knights + b_bishops;

        // Single pawn endgame check: Wrong-colored bishop + rook pawn (KBP vs K dead draw)
        if total_pawns == 1 {
            // White KBP vs Black K
            if w_pawn_count == 1 && b_pawn_count == 0 && w_bishops == 1 && w_knights == 0 && b_minors == 0 {
                let p_sq = white_pawns.trailing_zeros() as i32;
                let p_file = p_sq % 8;
                if p_file == 0 || p_file == 7 {
                    let prom_sq = if p_file == 0 { 56 } else { 63 };
                    let prom_color = (prom_sq % 8 + prom_sq / 8) % 2;
                    let b_sq = board.bitboards[crate::model::WHITE_BISHOP].trailing_zeros() as i32;
                    let b_color = (b_sq % 8 + b_sq / 8) % 2;
                    if b_color != prom_color {
                        let bk_sq = board.bitboards[crate::model::BLACK_KING].trailing_zeros() as i32;
                        let bk_file = bk_sq % 8;
                        let bk_rank = bk_sq / 8;
                        let prom_file = prom_sq % 8;
                        let prom_rank = prom_sq / 8;
                        let dist_to_prom = (bk_file - prom_file).abs().max((bk_rank - prom_rank).abs());
                        if dist_to_prom <= 1 || (bk_file == p_file && bk_rank > p_sq / 8) {
                            return true;
                        }
                    }
                }
            }
            // Black KBP vs White K
            if b_pawn_count == 1 && w_pawn_count == 0 && b_bishops == 1 && b_knights == 0 && w_minors == 0 {
                let p_sq = black_pawns.trailing_zeros() as i32;
                let p_file = p_sq % 8;
                if p_file == 0 || p_file == 7 {
                    let prom_sq = if p_file == 0 { 0 } else { 7 };
                    let prom_color = (prom_sq % 8 + prom_sq / 8) % 2;
                    let b_sq = board.bitboards[crate::model::BLACK_BISHOP].trailing_zeros() as i32;
                    let b_color = (b_sq % 8 + b_sq / 8) % 2;
                    if b_color != prom_color {
                        let wk_sq = board.bitboards[crate::model::WHITE_KING].trailing_zeros() as i32;
                        let wk_file = wk_sq % 8;
                        let wk_rank = wk_sq / 8;
                        let prom_file = prom_sq % 8;
                        let prom_rank = prom_sq / 8;
                        let dist_to_prom = (wk_file - prom_file).abs().max((wk_rank - prom_rank).abs());
                        if dist_to_prom <= 1 || (wk_file == p_file && wk_rank < p_sq / 8) {
                            return true;
                        }
                    }
                }
            }
            return false;
        }

        // KvK
        if w_minors + b_minors == 0 {
            return true;
        }

        // KNvK or KBvK (one side has at most 1 minor piece, other side has 0)
        if (w_minors <= 1 && b_minors == 0) || (b_minors <= 1 && w_minors == 0) {
            return true;
        }

        // KNNvK (two knights vs bare king cannot force checkmate)
        if (w_knights == 2 && w_bishops == 0 && b_minors == 0)
            || (b_knights == 2 && b_bishops == 0 && w_minors == 0)
        {
            return true;
        }

        // KN vs KN (one knight each, no pawns or other pieces)
        if w_knights == 1 && w_bishops == 0 && b_knights == 1 && b_bishops == 0 {
            return true;
        }

        // KB vs KN and KN vs KB (one minor piece each, no pawns or other pieces)
        if (w_bishops == 1 && w_knights == 0 && b_knights == 1 && b_bishops == 0)
            || (w_knights == 1 && w_bishops == 0 && b_bishops == 1 && b_knights == 0)
        {
            return true;
        }

        // KB vs KB with same-color bishops (no pawns)
        if w_bishops == 1 && w_knights == 0 && b_bishops == 1 && b_knights == 0 {
            let w_sq = board.bitboards[crate::model::WHITE_BISHOP].trailing_zeros() as i32;
            let b_sq = board.bitboards[crate::model::BLACK_BISHOP].trailing_zeros() as i32;
            let w_color = (w_sq % 8 + w_sq / 8) % 2;
            let b_color = (b_sq % 8 + b_sq / 8) % 2;
            if w_color == b_color {
                return true;
            }
        }

        false
    }

    #[inline(always)]
    pub fn apply_endgame_mopup(&self, mut eval: i16, board: &Board, game_phase: i16, config: &Config) -> i16 {
        if !config.enable_endgame_mopup
            || game_phase > config.mopup_max_game_phase
            || eval.abs() < config.mopup_eval_threshold
        {
            return eval;
        }

        let white_winning = eval > 0;
        let (winning_non_pawns, losing_non_pawns, losing_pawns) = if white_winning {
            (
                board.bitboards[crate::model::WHITE_KNIGHT]
                    | board.bitboards[crate::model::WHITE_BISHOP]
                    | board.bitboards[crate::model::WHITE_ROOK]
                    | board.bitboards[crate::model::WHITE_QUEEN],
                board.bitboards[crate::model::BLACK_KNIGHT]
                    | board.bitboards[crate::model::BLACK_BISHOP]
                    | board.bitboards[crate::model::BLACK_ROOK]
                    | board.bitboards[crate::model::BLACK_QUEEN],
                board.bitboards[crate::model::BLACK_PAWN],
            )
        } else {
            (
                board.bitboards[crate::model::BLACK_KNIGHT]
                    | board.bitboards[crate::model::BLACK_BISHOP]
                    | board.bitboards[crate::model::BLACK_ROOK]
                    | board.bitboards[crate::model::BLACK_QUEEN],
                board.bitboards[crate::model::WHITE_KNIGHT]
                    | board.bitboards[crate::model::WHITE_BISHOP]
                    | board.bitboards[crate::model::WHITE_ROOK]
                    | board.bitboards[crate::model::WHITE_QUEEN],
                board.bitboards[crate::model::WHITE_PAWN],
            )
        };

        if winning_non_pawns != 0 && losing_non_pawns == 0 && losing_pawns == 0 {
            let winning_king_sq = if white_winning {
                board.bitboards[crate::model::WHITE_KING].trailing_zeros() as i32
            } else {
                board.bitboards[crate::model::BLACK_KING].trailing_zeros() as i32
            };
            let losing_king_sq = if white_winning {
                board.bitboards[crate::model::BLACK_KING].trailing_zeros() as i32
            } else {
                board.bitboards[crate::model::WHITE_KING].trailing_zeros() as i32
            };

            let losing_rank = losing_king_sq / 8;
            let losing_file = losing_king_sq % 8;

            // 1. Center distance of losing king (pushing to edge/corners)
            let center_dist_rank = (losing_rank - 3).abs().max((losing_rank - 4).abs());
            let center_dist_file = (losing_file - 3).abs().max((losing_file - 4).abs());
            let push_to_edge_bonus =
                (center_dist_rank + center_dist_file) * config.mopup_center_weight as i32;

            // 2. Proximity between kings (Chebyshev distance)
            let winning_rank = winning_king_sq / 8;
            let winning_file = winning_king_sq % 8;
            let king_dist = (winning_rank - losing_rank).abs().max((winning_file - losing_file).abs());
            let close_kings_bonus = (7 - king_dist) * config.mopup_proximity_weight as i32;

            let mopup_total = (push_to_edge_bonus + close_kings_bonus) as i16;
            if white_winning {
                eval += mopup_total;
            } else {
                eval -= mopup_total;
            }
        }
        eval
    }


    /// The per-pawn form of [`Self::pawn_dynamic_scores`], kept as the reference its tests
    /// compare against.
    #[cfg(test)]
    fn white_pawn_dynamic_score(&self, sq: u8, board: &Board, config: &Config, precalculated_passed_pawns: u64) -> (i16, i16) {
        let mut o_eval = 0;
        let mut e_eval = 0;
        let sq = sq as i32;
        let rank = sq / 8;
        let file = sq % 8;
        let on_rank = rank + 1;

        if (3..=5).contains(&on_rank) {
            let white_knights = board.bitboards[crate::model::WHITE_KNIGHT];
            let has_white_knight_support =
                (file > 0 && ((1u64 << (sq + 7)) & white_knights) != 0) ||
                (file < 7 && ((1u64 << (sq + 9)) & white_knights) != 0);
            if has_white_knight_support {
                o_eval += config.pawn_supports_knight_outpost;
            }
        }

        let white_bishops = board.bitboards[crate::model::WHITE_BISHOP];
        if (file < 7 && ((1u64 << (sq + 9)) & white_bishops) != 0) ||
           (file > 0 && ((1u64 << (sq + 7)) & white_bishops) != 0) {
            e_eval += config.pawn_defends_bishop;
        }

        let black_non_pawns = board.bitboards[crate::model::BLACK_ROOK] |
                              board.bitboards[crate::model::BLACK_KNIGHT] |
                              board.bitboards[crate::model::BLACK_BISHOP] |
                              board.bitboards[crate::model::BLACK_QUEEN] |
                              board.bitboards[crate::model::BLACK_KING];
        let attacked_by_pawn =
            (file < 7 && ((1u64 << (sq + 9)) & black_non_pawns) != 0) ||
            (file > 0 && ((1u64 << (sq + 7)) & black_non_pawns) != 0);
        if attacked_by_pawn {
            o_eval += config.pawn_attacks_opponent_fig + if board.white_to_move {
                config.pawn_attacks_opponent_fig_with_tempo
            } else {
                0 
            };
            e_eval += config.pawn_attacks_opponent_fig / 2;
        }

        let is_passed = (1u64 << sq) & precalculated_passed_pawns != 0;
        if is_passed {
            let white_king_sq = board.bitboards[crate::model::WHITE_KING].trailing_zeros() as u8;
            let black_king_sq = board.bitboards[crate::model::BLACK_KING].trailing_zeros() as u8;
            e_eval += Self::king_passer_proximity_score(sq as u8, true, white_king_sq, black_king_sq, config);
        }

        // Pawn Storm against enemy castled king
        let black_king_sq = board.bitboards[crate::model::BLACK_KING].trailing_zeros() as i32;
        let bk_file = black_king_sq % 8;
        let bk_rank = black_king_sq / 8;
        if bk_rank >= 5 {
            if bk_file >= 5 && file >= 5 && rank >= 3 {
                o_eval += config.pawn_storm_bonus * (rank - 2) as i16;
            } else if bk_file <= 2 && file <= 2 && rank >= 3 {
                o_eval += config.pawn_storm_bonus * (rank - 2) as i16;
            }
        }

        (o_eval, e_eval)
    }

    /// See [`Self::white_pawn_dynamic_score`].
    #[cfg(test)]
    fn black_pawn_dynamic_score(&self, sq: u8, board: &Board, config: &Config, precalculated_passed_pawns: u64) -> (i16, i16) {
        let mut o_eval = 0;
        let mut e_eval = 0;
        let sq = sq as i32;
        let rank = sq / 8;
        let file = sq % 8;

        if (3..=5).contains(&rank) {
            let black_knights = board.bitboards[crate::model::BLACK_KNIGHT];
            let has_black_knight_support =
                (file > 0 && sq >= 9 && ((1u64 << (sq - 9)) & black_knights) != 0) ||
                (file < 7 && sq >= 7 && ((1u64 << (sq - 7)) & black_knights) != 0);
            if has_black_knight_support {
                o_eval -= config.pawn_supports_knight_outpost;
            }
        }

        let black_bishops = board.bitboards[crate::model::BLACK_BISHOP];
        if (file > 0 && sq >= 9 && ((1u64 << (sq - 9)) & black_bishops) != 0) ||
           (file < 7 && sq >= 7 && ((1u64 << (sq - 7)) & black_bishops) != 0) {
            e_eval -= config.pawn_defends_bishop;
        }

        let white_non_pawns = board.bitboards[crate::model::WHITE_ROOK] |
                              board.bitboards[crate::model::WHITE_KNIGHT] |
                              board.bitboards[crate::model::WHITE_BISHOP] |
                              board.bitboards[crate::model::WHITE_QUEEN] |
                              board.bitboards[crate::model::WHITE_KING];
        let attacked_by_pawn =
            (file > 0 && sq >= 9 && ((1u64 << (sq - 9)) & white_non_pawns) != 0) ||
            (file < 7 && sq >= 7 && ((1u64 << (sq - 7)) & white_non_pawns) != 0);
        if attacked_by_pawn {
            o_eval -= config.pawn_attacks_opponent_fig + if !board.white_to_move {
                config.pawn_attacks_opponent_fig_with_tempo
            } else {
                0
            };
            e_eval -= config.pawn_attacks_opponent_fig / 2;
        }

        let is_passed = (1u64 << sq) & precalculated_passed_pawns != 0;
        if is_passed {
            let white_king_sq = board.bitboards[crate::model::WHITE_KING].trailing_zeros() as u8;
            let black_king_sq = board.bitboards[crate::model::BLACK_KING].trailing_zeros() as u8;
            e_eval -= Self::king_passer_proximity_score(sq as u8, false, white_king_sq, black_king_sq, config);
        }

        // Pawn Storm against enemy castled king
        let white_king_sq = board.bitboards[crate::model::WHITE_KING].trailing_zeros() as i32;
        let wk_file = white_king_sq % 8;
        let wk_rank = white_king_sq / 8;
        if wk_rank <= 2 {
            if wk_file >= 5 && file >= 5 && rank <= 4 {
                o_eval -= config.pawn_storm_bonus * (5 - rank) as i16;
            } else if wk_file <= 2 && file <= 2 && rank <= 4 {
                o_eval -= config.pawn_storm_bonus * (5 - rank) as i16;
            }
        }

        (o_eval, e_eval)
    }
}

#[cfg(test)]
mod tests {
    use crate::service::Service;
    use crate::config::Config;



    #[test]
    fn get_eval_even_test() {
        equal_eval("rnbqk1nr/2p2pp1/1p2p3/8/8/1P2P3/2P2PP1/RNBQK1NR w KQkq - 0 1");
        equal_eval("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        equal_eval("rnbqkbnr/1ppp1pp1/8/8/8/8/1PPP1PP1/RNBQKBNR w KQkq - 0 1");
        equal_eval("rnbqk1n1/pppp1ppp/4p3/8/8/4P3/PPPP1PPP/RNBQK1N1 w HQhq - 0 1");
        equal_eval("rnk2bnr/pppppppp/8/8/8/8/PPPPPPPP/RNK2BNR w KQkq - 0 1");
        // Until v0.47.0 this entry had Black's rook on a4 against White's on a3, which is not a
        // mirror image; the hand-set tables happened to score it within 10. Its mirror image.
        equal_eval("3qk1r1/ppppp1pp/r2bbp1n/8/8/R2BBP1N/PPPPP1PP/3QK1R1 w Kk - 0 1");
        equal_eval("r1b1k2r/ppp1p1p1/5P1p/2npN1B1/2NPn1b1/5p1P/PPP1P1P1/R1B1K2R w Qq - 0 1");
        equal_eval("8/8/8/8/2k5/4K3/8/8 w - - 0 1");
        equal_eval("rn2k2r/p2ppppp/4b3/8/8/4B3/P2PPPPP/RN2K2R w KQkq - 0 1");
        equal_eval("rnbqkb1r/pppppppp/8/5n2/5N2/8/PPPPPPPP/RNBQKB1R w KQkq - 0 1");
        equal_eval("rnbqkb1r/pppppppp/5n2/8/8/5N2/PPPPPPPP/RNBQKB1R w KQkq - 0 1");
        equal_eval("rnbqkbnr/p6p/1p4p1/2pPPp2/2PppP2/1P4P1/P6P/RNBQKBNR w KQkq - 0 1");
        equal_eval("1k6/3p4/4P3/8/8/4p3/3P4/1K6 w - - 0 1");
        equal_eval("rnbqkb1r/ppppp1pp/6n1/6P1/6p1/6N1/PPPPP1PP/RNBQKB1R w KQkq - 0 1");
        equal_eval("3k4/8/p1p5/1p2ppp1/1P2PPP1/P1P5/8/3K4 w - - 0 1");
        equal_eval("6k1/5ppp/8/8/8/8/5PPP/6K1 w - - 0 1");
        equal_eval("7k/5ppp/8/8/8/8/5PPP/7K w - - 0 1");
    }

    #[test]
    fn eval_fig_value_test() {
        // The ranges follow the material of the Texel re-fits (`task.md` 29): pawn 78, knight 408,
        // bishop 433, rook 562, queen 1390 in the middlegame since the re-fit without
        // `adjust_eval` (29.19), 116, 543, 580, 782 and 1883 from v0.47.1, against 100, 300, 300,
        // 500 and 950 until v0.46.4. Each range keeps its place around the evaluation it reads; the
        // pawn range moved again with the mobility of `task.md` 29.24 (66 -> 39: a missing e-pawn
        // opens Black's bishop and queen), and again with the king safety of step 49 (39 -> 7: the
        // central pawn shield, worth White's e2 against Black's empty e7, fell from 37 to 5 when the
        // new king safety took over its part).
        // Figure values test for white
        eval_between("rnbqkbnr/pppp1ppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 1, 120);
        eval_between("rnbqkb1r/pppp1ppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 360, 600);
        eval_between("rn1qkb1r/pppp1ppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 790, 1140);
        eval_between("r2qkb1r/pppp1ppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 1110, 1510);
        eval_between("3qkb2/pppp1ppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQ - 0 1", 2220, 2820);
        eval_between("4k3/pppp1ppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQha - 0 1", 4010, 4910);

        // Figure values test for black
        eval_between("rnbqkbnr/pppppppp/8/8/8/8/PPPPP1PP/RNBQKBNR b KQkq - 0 1", -210, 90);
        eval_between("rnbqkbnr/pppppppp/8/8/8/8/PPPPP1PP/RNBQKB1R b KQkq - 0 1", -640, -420);
        eval_between("rnbqkbnr/pppppppp/8/8/8/8/PPPP1PPP/RN1QKB1R b KQkq - 0 1", -1140, -790);
        eval_between("rnbqkbnr/pppppppp/8/8/8/8/PPPP1PPP/R2QKB1R b KQkq - 0 1", -1510, -1110);
        eval_between("rnbqkbnr/pppppppp/8/8/8/8/PPPP1PPP/3QKB2 b - - 0 1", -2820, -2220);
        eval_between("rnbqkbnr/pppppppp/8/8/8/8/PPPP1PPP/4K3 b kq - 0 1", -4910, -4010);
    }

    #[test]
    fn position_unequel_test() {
        // position unequel: a knight on its fourth rank against one on its third, all else
        // symmetric. Since the safe pawn push of step 49 B2 a knight on f5 in the opening is kicked by
        // e4 or g4 with tempo (-23 -> +9), so the pawns on d5 / h5 against d4 / h4 make those pushes
        // unsafe for both sides; the advanced knight reads -9 / +9 against 0 for the symmetric pair.
        eval_between("rnbqkb1r/ppp1ppp1/8/3p1n1p/3P3P/5N2/PPP1PPP1/RNBQKB1R w KQkq - 0 1", -50, -1);
        eval_between("rnbqkb1r/ppp1ppp1/5n2/3p3p/3P1N1P/8/PPP1PPP1/RNBQKB1R w KQkq - 0 1", 1, 50);
        eval_between("1k6/8/8/4P3/8/4p3/8/1K6 w - - 0 1", -700, -100);
        eval_between("1k6/3p4/8/4P3/8/4p3/3P4/1K6 w - - 0 1", -200, -10);
        eval_between("1k6/3p4/8/4P3/8/4p3/3P4/1K6 w - - 0 1", -200, -10);
    }

    #[test]
    fn compare_eval_test() {
        let fen_service = Service::new().fen;
        let eval_service = Service::new().eval;
        let movegen = &Service::new().move_gen;
        let config = &Config::new();

        let board = fen_service.set_fen("rnb1k1n1/pp4p1/2p3Nr/3p3p/q7/1RP3P1/3NPPBP/3QK2R w Kq - 3 19");
        let eval1 = eval_service.calc_eval(&board, config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX, 180);

        let board = fen_service.set_fen("rnb1k1n1/pp4p1/2p3Nr/3B3p/q7/1RP3P1/3NPP1P/3QK2R b Kq - 0 19");
        let eval2 = eval_service.calc_eval(&board, config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX, 180);

        let board = fen_service.set_fen("rnb1k1n1/pp4p1/6Nr/3p3p/q7/1RP3P1/3NPPBP/3QK2R w Kq - 3 19");
        let eval3 = eval_service.calc_eval(&board, config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX, 180);

        let board = fen_service.set_fen("rnb1k3/pp2n1p1/7r/3p3p/q4N2/1RP3P1/3NPP1P/3QK2R w Kq - 2 21");
        let eval4 = eval_service.calc_eval(&board, config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX, 180);

        println!("{}", eval1);
        println!("{}", eval2);
        println!("{}", eval3);
        println!("{}", eval4);
    }

    #[test]
    fn unequal_position_test() {
        // Asymmetric King position with pawns on board (Black king centralized on c4 vs White on g3).
        // -112 until the king-centre step went (task.md 29.10), which counted the centre a second
        // time on top of the king's endgame table; -54 with the re-fitted table alone.
        eval_between("8/p7/8/8/2k5/6K1/P7/8 w - - 0 1", -90, -30);
    }

    #[test]
    fn knight_position_test() {
        fib("rnbqkbnr/pppppppp/8/8/8/5N2/PPPPPPPP/RNBQKB1R b KQkq - 1 1", "rnbqkbnr/pppppppp/8/8/8/7N/PPPPPPPP/RNBQKB1R b KQkq - 1 1");
        // Developing a knight is not worse for Black than leaving it at home. Until v0.47.0 the
        // pair developed it to the rim, a6; the re-fitted knight table of v0.47.1 scores a6 12 cp
        // below b8 (the rim against the development), so the pair develops it to c6.
        fib("rnbqkbnr/pppppppp/8/8/8/5N2/PPPPPPPP/RNBQKB1R w KQkq - 0 1", "r1bqkbnr/pppppppp/2n5/8/8/5N2/PPPPPPPP/RNBQKB1R w KQkq - 0 1");
        fib("rnbqkbnr/pppppppp/8/8/5N2/8/PPPPPPPP/RNBQKB1R w KQkq - 0 1", "rnbqkbnr/pppppp1p/8/6p1/5NP1/8/PPPPPP1P/RNBQKB1R w KQkq - 0 1");
        fib("rnbqkbnr/pppp1ppp/4p3/8/5N2/4P3/PPPP1PPP/RNBQKB1R w KQkq - 0 1", "rnbqkbnr/pppp1ppp/4p3/8/4N3/4P3/PPPP1PPP/RNBQKB1R w KQkq - 0 1");
        fib("rnbqkbnr/ppppp1pp/8/5p2/5N2/8/PPPPPPPP/RNBQKB1R w KQkq - 0 1", "rnbqkbnr/ppppp1pp/8/5p2/8/4N3/PPPPPPPP/RNBQKB1R w KQkq - 0 1");

        // special position to find bug
        fib("rnbqkbnr/1p3ppp/p7/2p5/1P1p4/N4N2/P2PPPPP/R1BQKB1R b KQkq - 1 7", "rnbqkbnr/1p3ppp/8/1pp5/PP1p4/5N2/3PPPPP/R1BQKB1R w KQkq - 0 8");
    }

    #[test]
    fn advance_pawn_eval_test() {
        fib("8/1k6/8/4P3/8/8/1K6/8 w - - 0 1", "8/1k6/8/8/4P3/8/1K6/8 w - - 0 1");
        fib("8/1k6/4P3/8/8/8/1K6/8 w - - 0 1", "8/1k6/8/4P3/8/8/1K6/8 w - - 0 1");
        fib("8/1k2P3/8/8/8/8/1K6/8 w - - 0 1", "8/1k6/4P3/8/8/8/1K6/8 w - - 0 1");

        fib("8/1k6/8/4p3/8/8/1K6/8 b - - 0 1", "8/1k6/8/8/4p3/8/1K6/8 b - - 0 1");
        fib("8/1k6/8/8/4p3/8/1K6/8 b - - 0 1", "8/1k6/8/8/8/4p3/1K6/8 b - - 0 1");
        fib("8/1k6/8/8/8/4p3/1K6/8 b - - 0 1", "8/1k6/8/8/8/8/1K2p3/8 b - - 0 1");
    }

    #[test]
    fn knight_attack_test() {
        // white knight attacks
        fib("8/1k6/4b3/8/5N2/1K6/8/8 w - - 0 1", "8/1k6/4b3/8/5N2/1K6/8/8 b - - 0 1");
        fib("8/1k6/4r3/8/5N2/1K6/8/8 w - - 0 1", "8/1k6/4r3/8/5N2/1K6/8/8 b - - 0 1");
        fib("8/1k6/4q3/8/5N2/1K6/8/8 w - - 0 1", "8/1k6/4q3/8/5N2/1K6/8/8 b - - 0 1");

        // black knight attacks
        fib("8/1k6/5n2/8/4B3/1K6/8/8 w - - 0 1", "8/1k6/5n2/8/4B3/1K6/8/8 b - - 0 1");
        fib("8/1k6/5n2/8/4R3/1K6/8/8 w - - 0 1", "8/1k6/5n2/8/4R3/1K6/8/8 b - - 0 1");
        fib("8/1k6/5n2/8/4Q3/1K6/8/8 w - - 0 1", "8/1k6/5n2/8/4Q3/1K6/8/8 b - - 0 1");
    }

    #[test]
    fn bishop_position_test() {
        // white bishop trapped at rim
        fib("r3k2r/pp1n2p1/2p3p1/5p2/3PnB2/2P3P1/PP2B1PP/R4RK1 b kq - 4 18", "r3k2r/pp1n2p1/2p3p1/5p2/3Pn2B/2P3P1/PP2B1PP/R4RK1 b kq - 4 18");
    }

    #[test]
    fn game_phase_test() {
        let fen = Service::new().fen;
        let eval = Service::new().eval;

        // init board
        let board = fen.set_init_board();
        assert!(eval.get_game_phase(&board) > 254);
        assert!(eval.get_game_phase(&board) < 256);

        // 7 pieces board (1 knight, 1 queen = phase 5/24 = 53)
        let board = fen.set_fen("8/8/2kq4/3ppp2/8/8/5N2/4K3 w - - 0 1");
        assert_eq!(53, eval.get_game_phase(&board));

        // 6 pieces board (1 knight, 1 queen = phase 5/24 = 53)
        let board = fen.set_fen("8/8/2kq4/4pp2/8/8/5N2/4K3 w - - 0 1");
        assert_eq!(53, eval.get_game_phase(&board));

        // 3 pieces board (1 knight = phase 1/24 = 10)
        let board = fen.set_fen("8/8/2k5/8/8/8/5N2/4K3 w - - 0 1");
        assert_eq!(10, eval.get_game_phase(&board));
    }

    #[test]
    pub fn double_pawn_test() {
        equal_eval("2k5/3p1p2/3p4/5p2/5P2/3P4/3P1P2/2K5 w - - 0 1");
        eval_between("2k5/5p2/5p2/8/8/8/4PP2/2K5 w - - 0 1", 0, 60);
        eval_between("2k5/4pp2/8/8/8/5P2/5P2/2K5 w - - 0 1", -60, 0);
        fib("2k5/4pp2/8/8/8/4P3/5P2/2K5 w - - 0 1", "2k5/4pp2/8/8/8/5P2/5P2/2K5 w - - 0 1");
        fib("2k5/5p2/4p3/8/8/8/4PP2/2K5 w - - 0 1", "2k5/5p2/4p3/8/5P2/8/5P2/2K5 w - - 0 1");
    }

    #[test]
    pub fn print_eval_for_fig_test() {
        _print_eval_for_fig("8/5Bp1/4P3/6p1/1b1k1P2/5K2/8/8 w - - 0 1");
    }

    #[test]
    pub fn adjust_eval_test() {
        let eval_service = Service::new().eval;
        let config = &Config::new();

        println!("{}", eval_service.adjust_eval(0, 255, config));
        println!("{}", eval_service.adjust_eval(0, 100, config));
        println!("{}", eval_service.adjust_eval(200, 160, config));
        println!("{}", eval_service.adjust_eval(200, 100, config));
        println!("{}", eval_service.adjust_eval(200, 90, config));
        println!("{}", eval_service.adjust_eval(200, 50, config));
    }

    fn fib(fen1: &str, fen2: &str) {
        let fen = Service::new().fen;
        let eval = Service::new().eval;
        let movegen = Service::new().move_gen;
        let config = Config::for_tests();

        let board1 = fen.set_fen(fen1);
        let board2 = fen.set_fen(fen2);
        let eval1 = eval.calc_eval(&board1, &config, &movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX, 180);
        let eval2 = eval.calc_eval(&board2, &config, &movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX, 180);

        println!("FIB: eval1={} eval2={} diff={} | fen1='{}' fen2='{}'", eval1, eval2, eval1 - eval2, fen1, fen2);

        if eval1 < eval2 - 10 {
            println!("-->> eval1 is unexpectedly much less than eval2: {}", eval1 - eval2);
            assert!(false);
        }
    }

    /// The attack maps `evaluate` builds against a per-square reference: for every square, the
    /// pieces of each side that attack it by `get_attackers_mask`. Positions with pawn double
    /// attacks, batteries, a queen, both kings in contact and pieces on the rim.
    #[test]
    fn test_attack_maps_match_the_attackers_of_every_square() {
        let service = Service::new();
        let config = Config::new();
        for fen in [
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "r1bq1rk1/pp2bppp/2n1pn2/2pp4/2PP4/2N1PN2/PP2BPPP/R1BQ1RK1 w - - 0 8",
            "2r3k1/1q3ppp/p2p4/1p1Pp3/1P2P1Q1/P2B4/5PPP/2R3K1 b - - 0 25",
            "8/8/3k4/3P4/3K4/8/8/8 w - - 0 1",
            "N6r/1p1k1ppp/2np4/b1p1p3/4P3/2P2N2/PP1P1PPP/R1B1KB1R b KQ - 0 12",
            "rnb1k1n1/pp4p1/2p3Nr/3p3p/q7/1RP3P1/3NPPBP/3QK2R w Kq - 3 19",
        ] {
            let board = service.fen.set_fen(fen);
            service.eval.calc_eval(&board, &config, &service.move_gen, &crate::pawn_hash::PawnHashTable::new(16),
                i16::MIN + 1, i16::MAX - 1, 0);
            let maps = super::LAST_MAPS.with(|m| m.get());
            for side in 0..2 {
                let offset = if side == 0 { 0 } else { 6 };
                for sq in 0..64u8 {
                    // `get_attackers_mask(board, white, ..)` returns the attackers of the side
                    // that is not `white`.
                    let attackers = service.move_gen.get_attackers_mask(&board, side == 1, sq, board.occupied);
                    let bit = 1u64 << sq;
                    assert_eq!(maps.all[side] & bit != 0, attackers != 0, "{fen}: side {side} attacks square {sq}");
                    assert_eq!(maps.two[side] & bit != 0, attackers.count_ones() >= 2, "{fen}: side {side} attacks square {sq} twice");
                    for piece in 0..6 {
                        assert_eq!(maps.by[side][piece] & bit != 0, attackers & board.bitboards[offset + piece] != 0,
                            "{fen}: side {side} piece {piece} attacks square {sq}");
                    }
                }
            }
        }
    }

    /// The king-safety feature `index` of the king of `us` in `fen`, read through the endgame
    /// penalty: with only that feature's endgame weight at 20, `u_eg / 20` is the feature itself
    /// (the ring hits scaled to nine squares, rounded down). `None` when the gate does not open.
    fn king_safety_feature(fen: &str, us: usize, index: usize) -> Option<i16> {
        let service = Service::new();
        let board = service.fen.set_fen(fen);
        let set = |c: &mut Config, i: usize, v: i16| match i {
            0 => c.king_safety_knight_attacker_eg = v,
            1 => c.king_safety_bishop_attacker_eg = v,
            2 => c.king_safety_rook_attacker_eg = v,
            3 => c.king_safety_queen_attacker_eg = v,
            4 => c.king_safety_ring_attacks_eg = v,
            5 => c.king_safety_weak_squares_eg = v,
            6 => c.king_safety_no_queen_eg = v,
            7 => c.king_safety_safe_knight_check_eg = v,
            8 => c.king_safety_safe_bishop_check_eg = v,
            9 => c.king_safety_safe_rook_check_eg = v,
            10 => c.king_safety_safe_queen_check_eg = v,
            _ => c.king_safety_adjustment_eg = v,
        };
        // Every king-safety weight at 0 but the one asked for.
        let mut zero = Config::new();
        for i in 0..12 {
            set(&mut zero, i, 0);
        }
        zero.king_safety_knight_attacker_mg = 0;
        zero.king_safety_bishop_attacker_mg = 0;
        zero.king_safety_rook_attacker_mg = 0;
        zero.king_safety_queen_attacker_mg = 0;
        zero.king_safety_ring_attacks_mg = 0;
        zero.king_safety_weak_squares_mg = 0;
        zero.king_safety_no_queen_mg = 0;
        zero.king_safety_safe_knight_check_mg = 0;
        zero.king_safety_safe_bishop_check_mg = 0;
        zero.king_safety_safe_rook_check_mg = 0;
        zero.king_safety_safe_queen_check_mg = 0;
        zero.king_safety_adjustment_mg = 0;
        let mut config = zero.clone();
        set(&mut config, index, 20);
        service.eval.calc_eval(&board, &config, &service.move_gen, &crate::pawn_hash::PawnHashTable::new(16),
            i16::MIN + 1, i16::MAX - 1, 0);
        let maps = super::LAST_MAPS.with(|m| m.get());
        let king_sq = board.bitboards[if us == 0 { crate::model::WHITE_KING } else { crate::model::BLACK_KING }].trailing_zeros() as u8;
        let ring = service.eval.get_king_ring(king_sq);
        let mut attack = super::RingAttack::default();
        let off = if us == 0 { 6 } else { 0 };
        for (slot, piece) in [crate::model::WHITE_KNIGHT, crate::model::WHITE_BISHOP, crate::model::WHITE_ROOK, crate::model::WHITE_QUEEN].into_iter().enumerate() {
            let mut b = board.bitboards[off + piece];
            while b != 0 {
                let sq = b.trailing_zeros() as usize;
                let attacks = match piece {
                    crate::model::WHITE_KNIGHT => service.move_gen.get_knight_attacks(sq),
                    crate::model::WHITE_BISHOP => service.move_gen.get_bishop_attacks(sq, board.occupied),
                    crate::model::WHITE_ROOK => service.move_gen.get_rook_attacks(sq, board.occupied),
                    _ => service.move_gen.get_bishop_attacks(sq, board.occupied) | service.move_gen.get_rook_attacks(sq, board.occupied),
                };
                attack.add(slot, attacks & ring);
                b &= b - 1;
            }
        }
        // The adjustment alone says whether the gate opened: it is 1 whenever it does.
        let mut gate = zero;
        gate.king_safety_adjustment_eg = 20;
        if service.eval.king_safety(&board, &gate, &service.move_gen, &maps, us, king_sq, ring, &attack).1 == 0 {
            return None;
        }
        let (_, eg) = service.eval.king_safety(&board, &config, &service.move_gen, &maps, us, king_sq, ring, &attack);
        Some(eg)
    }

    /// A rook check from d1 is safe until a White rook covers the first rank; the queen on h4
    /// opens the gate on its own and makes f2 and h2 weak (defended by the king alone).
    #[test]
    fn test_king_safety_features_count_what_they_name() {
        let attacked = "3r2k1/5ppp/8/8/7q/8/5PPP/6K1 w - - 0 1";
        let covered = "3r2k1/5ppp/8/8/7q/8/5PPP/R5K1 w - - 0 1";
        assert_eq!(king_safety_feature(attacked, 0, 9), Some(1), "safe rook check from d1");
        assert_eq!(king_safety_feature(covered, 0, 9), Some(0), "d1 covered by the rook on a1");
        assert_eq!(king_safety_feature(attacked, 0, 3), Some(1), "one queen on the ring");
        assert_eq!(king_safety_feature(attacked, 0, 2), Some(0), "the rook does not reach the ring");
        assert_eq!(king_safety_feature(attacked, 0, 5), Some(2), "f2 and h2 weak");
        assert_eq!(king_safety_feature(attacked, 0, 4), Some(3), "two ring squares of five: 9 * 2 / 5");
        assert_eq!(king_safety_feature(attacked, 0, 6), Some(0), "the enemy has a queen");
        assert_eq!(king_safety_feature(attacked, 0, 11), Some(1), "the adjustment is one");
        // Without the queen one attacker does not open the gate, two do.
        assert_eq!(king_safety_feature("3r2k1/5ppp/8/8/8/8/5PPP/6K1 w - - 0 1", 0, 11), None, "no attacker");
        assert_eq!(king_safety_feature("6k1/5ppp/8/8/8/5n2/5PPP/6K1 w - - 0 1", 0, 11), None, "one knight alone");
        let two = "6k1/5ppp/8/8/8/5n2/5PPP/4r1K1 w - - 0 1";
        assert_eq!(king_safety_feature(two, 0, 6), Some(1), "no enemy queen");
        assert_eq!(king_safety_feature(two, 0, 0), Some(1), "one knight on the ring");
        assert_eq!(king_safety_feature(two, 0, 2), Some(1), "one rook on the ring");
        // A knight on e5 reaches f3, one of the knight-check squares of g1 (e2, f3, h3); the pawn
        // on g2 covers it.
        let knight = "6k1/5ppp/8/4n3/7q/8/5P1P/6K1 w - - 0 1";
        assert_eq!(king_safety_feature(knight, 0, 7), Some(1), "safe knight check from f3");
        let knight_covered = "6k1/5ppp/8/4n3/7q/8/5PPP/6K1 w - - 0 1";
        assert_eq!(king_safety_feature(knight_covered, 0, 7), Some(0), "f3 covered by the pawn on g2");
        // The same from Black's side: the colour mirror of the first position.
        assert_eq!(king_safety_feature("6k1/5ppp/8/7Q/8/8/5PPP/3R2K1 b - - 0 1", 1, 9), Some(1), "safe rook check from d8");
        assert_eq!(king_safety_feature("6k1/5ppp/8/7Q/8/8/5PPP/3R2K1 b - - 0 1", 1, 5), Some(2), "f7 and h7 weak");
    }

    /// The threat counts of each side on constructed positions.
    #[test]
    fn test_threat_features_count_what_they_name() {
        let features = |fen: &str, us: usize| -> [i16; 5] {
            let service = Service::new();
            let board = service.fen.set_fen(fen);
            service.eval.calc_eval(&board, &Config::new(), &service.move_gen, &crate::pawn_hash::PawnHashTable::new(16),
                i16::MIN + 1, i16::MAX - 1, 0);
            super::EvalService::threat_features(&board, &super::LAST_MAPS.with(|m| m.get()), us)
        };
        // [hanging, lesser on minor, lesser on rook, lesser on queen, safe pawn push]
        assert_eq!(features("4k3/8/3p4/4N3/8/8/8/4K3 w - - 0 1", 1), [1, 1, 0, 0, 0], "a knight on e5 hit by a pawn, undefended");
        assert_eq!(features("4k3/8/3p4/4N3/3P4/8/8/4K3 w - - 0 1", 1), [0, 1, 0, 0, 0], "defended by d4: not hanging");
        assert_eq!(features("4k3/8/8/1b6/8/3R4/8/4K3 w - - 0 1", 1), [1, 0, 1, 0, 0], "a bishop on a rook");
        assert_eq!(features("4k3/8/8/8/2n5/8/3Q4/4K3 w - - 0 1", 1), [0, 0, 0, 1, 0], "a knight on the queen, the king defends d2");
        assert_eq!(features("4k3/8/5n2/8/4P3/8/8/4K3 w - - 0 1", 0), [0, 0, 0, 0, 1], "e4-e5 hits the knight on f6");
        assert_eq!(features("4k3/8/3p1n2/8/4P3/8/8/4K3 w - - 0 1", 0), [0, 0, 0, 0, 0], "e5 attacked by the pawn on d6: not safe");
        assert_eq!(features("4k3/8/5n2/8/8/8/4P3/4K3 w - - 0 1", 0), [0, 0, 0, 0, 0], "e2-e4 hits nothing; a double push is not a threat on f5");
        assert_eq!(features("4k3/8/8/5n2/8/8/4P3/4K3 w - - 0 1", 0), [0, 0, 0, 0, 1], "e2-e4 hits the knight on f5");
    }

    fn equal_eval(fen: &str) {
        let fen_service = Service::new().fen;
        let eval_service = Service::new().eval;
        let movegen = Service::new().move_gen;

        let config = &Config::_for_evel_equal_tests();
        let board = &fen_service.set_fen(fen);
        let eval = eval_service.calc_eval(board, config, &movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX, 180);
        assert!(eval.abs() <= 10, "FEN: {} | Eval {} is not close to 0", fen, eval);
    }

    fn eval_between(fen: &str, lower: i16, higher: i16) {
        print!("Test: {} | ", fen);
        let fen_service = Service::new().fen;
        let eval_service = Service::new().eval;
        let movegen = Service::new().move_gen;

        let config = &Config::_for_evel_equal_tests();
        let board = &fen_service.set_fen(fen);
        let eval = eval_service.calc_eval(board, config, &movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX, 180);
        println!("Eval: {}", eval);
        assert!(eval >= lower);
        assert!(eval <= higher);
    }

    fn _print_eval_for_fig(fen: &str) {
        let fen_service = Service::new().fen;
        let eval_service = Service::new().eval;
        let movegen = Service::new().move_gen;

        let board = &fen_service.set_fen(fen);
        let mut config = Config::new();
        config.print_eval_per_figure = true;
        eval_service.calc_eval(board, &config, &movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX, 180);
        println!("------------");
    }

    #[test]
    fn test_positional_evaluation_capping() {
        let fen_service = Service::new().fen;
        let eval_service = Service::new().eval;
        let movegen = &Service::new().move_gen;
        
        let board = fen_service.set_init_board();
        
        // 1. Normal Aggressiveness Test (Cap = 150)
        let mut config_normal = Config::new();
        config_normal.use_nnue = false;
        config_normal.enable_positional_cap = true;
        config_normal.set_aggressiveness(crate::config::Aggressiveness::Normal);
        config_normal.your_turn_bonus = 1000; // Enormous positional bonus to force capping
        let eval_normal = eval_service.calc_eval(&board, &config_normal, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX, 180);
        // Soft cap calculation: 150 + (1000 - 150) / 5 = 150 + 170 = 320
        assert_eq!(eval_normal, 320, "Normal aggressiveness eval should be soft capped at 320");

        // 2. Aggressive Test (Cap = 250)
        let mut config_aggressive = Config::new();
        config_aggressive.use_nnue = false;
        config_aggressive.enable_positional_cap = true;
        config_aggressive.set_aggressiveness(crate::config::Aggressiveness::Aggressive);
        config_aggressive.your_turn_bonus = 1000;
        let eval_aggressive = eval_service.calc_eval(&board, &config_aggressive, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX, 180);
        // Soft cap calculation: 250 + (1000 - 250) / 5 = 250 + 150 = 400
        assert_eq!(eval_aggressive, 400, "Aggressive eval should be soft capped at 400");

        // 3. HighAggressive Test (Cap = 400)
        let mut config_high = Config::new();
        config_high.use_nnue = false;
        config_high.enable_positional_cap = true;
        config_high.set_aggressiveness(crate::config::Aggressiveness::HighAggressive);
        config_high.your_turn_bonus = 1000;
        let eval_high = eval_service.calc_eval(&board, &config_high, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX, 180);
        // Soft cap calculation: 400 + (1000 - 400) / 5 = 400 + 120 = 520
        assert_eq!(eval_high, 520, "High aggressive eval should be soft capped at 520");
    }

    #[test]
    fn test_new_positional_features() {
        let fen_service = Service::new().fen;
        let eval_service = Service::new().eval;
        let movegen = &Service::new().move_gen;

        // 1. Connected Passed Pawns Test
        {
            let board = fen_service.set_fen("8/8/8/8/4PP2/k7/8/K7 w - - 0 1");
            let mut config = Config::for_tests();
            config.max_eval_mult = 1.0;
            config.connected_passed_pawn_mg = 50;
            config.connected_passed_pawn_eg = 100;
            let eval_with = eval_service.calc_eval(&board, &config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX, 180);

            config.connected_passed_pawn_mg = 0;
            config.connected_passed_pawn_eg = 0;
            let eval_without = eval_service.calc_eval(&board, &config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX, 180);
            
            // Expected bonus: 2 connected pawns, each gets EG bonus (100) = 200 total
            let diff = eval_with - eval_without;
            assert_eq!(diff, 200, "Connected passed pawns bonus not applied correctly");
        }

        // 2. True Outposts Test
        {
            // Outpost square at e5, supported by White pawn on d4
            let board = fen_service.set_fen("8/8/8/4N3/3P4/k7/8/K7 w - - 0 1");
            let mut config = Config::for_tests();
            config.max_eval_mult = 1.0;
            config.knight_outpost_true_mg = 60;
            config.knight_outpost_true_eg = 30;
            let eval_with = eval_service.calc_eval(&board, &config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX, 180);

            config.knight_outpost_true_mg = 0;
            config.knight_outpost_true_eg = 0;
            let eval_without = eval_service.calc_eval(&board, &config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX, 180);

            // Phase = 1 (1 Knight = 1/24 * 255 = 10) -> mostly endgame (eg weight is 246/256)
            // Expected bonus: weighted outpost bonus ~ 30
            let diff = eval_with - eval_without;
            assert!(diff >= 25 && diff <= 35, "True outpost occupancy bonus not applied correctly, diff={}", diff);

            // Knight attacking true outpost (Knight on c4, outpost at e5)
            let board_att = fen_service.set_fen("8/8/8/8/2NP4/k7/8/K7 w - - 0 1");
            config.knight_outpost_true_mg = 60;
            config.knight_outpost_true_eg = 30;
            let eval_with_att = eval_service.calc_eval(&board_att, &config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX, 180);

            config.knight_outpost_true_mg = 0;
            config.knight_outpost_true_eg = 0;
            let eval_without_att = eval_service.calc_eval(&board_att, &config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX, 180);

            let diff_att = eval_with_att - eval_without_att;
            assert!(diff_att >= 25 && diff_att <= 35, "True outpost control bonus not applied correctly, diff={}", diff_att);
        }

        // 3. Asymmetric Castling / King Safety Test
        {
            let mut config = Config::for_tests();
            config.max_eval_mult = 1.0;
            config.king_pawn_shield_kingside = 100;
            config.king_pawn_shield_queenside = 20;
            config.king_pawn_shield = 0; // standard shield set to 0

            // The test is in middlegame/endgame interpolation.
            // Let's set game phase higher by adding queens to keep it in middlegame
            let board_ks_mg = fen_service.set_fen("q7/8/8/8/8/k7/5PPP/6QK w - - 0 1");
            let board_qs_mg = fen_service.set_fen("q7/8/8/8/8/k7/PPP5/2K3Q1 w - - 0 1");

            // Evaluate with shields active
            let eval_ks = eval_service.calc_eval(&board_ks_mg, &config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX, 180);
            let eval_qs = eval_service.calc_eval(&board_qs_mg, &config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX, 180);

            config.king_pawn_shield_kingside = 0;
            config.king_pawn_shield_queenside = 0;

            // Evaluate without shields
            let eval_ks_no = eval_service.calc_eval(&board_ks_mg, &config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX, 180);
            let eval_qs_no = eval_service.calc_eval(&board_qs_mg, &config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX, 180);

            let ks_diff = eval_ks - eval_ks_no;
            let qs_diff = eval_qs - eval_qs_no;

            assert!(ks_diff > qs_diff, "Kingside shield bonus {} should be larger than Queenside shield bonus {}, ks_diff={}, qs_diff={}", 100, 20, ks_diff, qs_diff);
        }

        // 4. Opposite-Colored Bishops Endgame Scaling Test
        {
            let board = fen_service.set_fen("8/8/8/4b3/8/k2B4/8/K7 w - - 0 1");
            let mut config = Config::for_tests();
            config.max_eval_mult = 1.0;
            
            config.opposite_bishops_draw_scale = 100;
            let eval_unscaled = eval_service.calc_eval(&board, &config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX, 180);

            config.opposite_bishops_draw_scale = 50;
            let eval_scaled = eval_service.calc_eval(&board, &config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX, 180);

            assert_eq!(eval_scaled, eval_unscaled / 2, "Opposite-colored bishops endgame evaluation not scaled correctly");
        }

        // 5. Tarrasch Rule (Rook Behind Enemy Passed Pawn) Test
        {
            // Black passed pawn on d4, White Rook behind it on d8 (rank 7 > rank 3)
            let board = fen_service.set_fen("3R4/8/8/8/3p4/k7/8/K7 w - - 0 1");
            let mut config = Config::for_tests();
            config.max_eval_mult = 1.0;
            config.rook_behind_enemy_passed_pawn_mg = 50;
            config.rook_behind_enemy_passed_pawn_eg = 100;
            let eval_with = eval_service.calc_eval(&board, &config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX, 180);

            config.rook_behind_enemy_passed_pawn_mg = 0;
            config.rook_behind_enemy_passed_pawn_eg = 0;
            let eval_without = eval_service.calc_eval(&board, &config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX, 180);

            // phase is 2 rooks = 4/24 * 255 = 42 -> mostly endgame
            let diff = eval_with - eval_without;
            assert!(diff >= 75 && diff <= 100, "Rook behind enemy passed pawn bonus not applied correctly, diff={}", diff);
        }

        // 6. Connected Passed Pawns Bitwise Accuracy Test for White and Black
        {
            let fen_service = Service::new().fen;
            let eval_service = Service::new().eval;
            let movegen = &Service::new().move_gen;
            let config = Config::for_tests();

            // White connected passed pawns on e5, f5 vs isolated passed pawns on a5, e5
            let board_connected = fen_service.set_fen("8/8/8/4PP2/8/k7/8/K7 w - - 0 1");
            let board_isolated = fen_service.set_fen("8/8/8/P3P3/8/k7/8/K7 w - - 0 1");

            let eval_connected = eval_service.calc_eval(&board_connected, &config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX, 180);
            let eval_isolated = eval_service.calc_eval(&board_isolated, &config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX, 180);

            assert!(eval_connected > eval_isolated, "Connected passed pawns should evaluate higher than isolated passed pawns");

            // Black connected passed pawns on e4, f4 vs isolated passed pawns on a4, e4
            let board_b_connected = fen_service.set_fen("8/8/8/K7/4pp2/8/8/k7 b - - 0 1");
            let board_b_isolated = fen_service.set_fen("8/8/8/K7/p3p3/8/8/k7 b - - 0 1");

            let eval_b_connected = eval_service.calc_eval(&board_b_connected, &config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX, 180);
            let eval_b_isolated = eval_service.calc_eval(&board_b_isolated, &config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX, 180);

            assert!(eval_b_connected < eval_b_isolated, "Black connected passed pawns should give a lower (more negative for White) eval than isolated");
        }

        // 7. Knight King Ring Attack Symmetry Test
        {
            let fen_service = Service::new().fen;
            let eval_service = Service::new().eval;
            let movegen = &Service::new().move_gen;
            let config = Config::for_tests();

            // Symmetrical position with White Knight attacking Black King ring vs Black Knight attacking White King ring
            let board_white_att = fen_service.set_fen("4k3/8/8/5N2/8/8/8/4K3 w - - 0 1");
            let board_black_att = fen_service.set_fen("4k3/8/8/8/5n2/8/8/4K3 b - - 0 1");

            let eval_w = eval_service.calc_eval(&board_white_att, &config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX, 180);
            let eval_b = eval_service.calc_eval(&board_black_att, &config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX, 180);

            assert_eq!(eval_w, -eval_b, "Evaluation should be perfectly symmetric for White and Black knight attacks on king ring");
        }
    }

    #[test]
    fn test_eval_service_nnue_missing_file_fail_fast() {
        if std::env::var("TEST_NNUE_FAIL_FAST").is_ok() {
            let mut config = Config::new();
            config.use_nnue = true;
            config.nnue_model_path = "eval_models/non_existent_model_file.bin".to_string();
            let _ = super::EvalService::new(&config);
            return;
        }

        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .arg("eval_service::tests::test_eval_service_nnue_missing_file_fail_fast")
            .arg("--exact")
            .arg("--nocapture")
            .env("TEST_NNUE_FAIL_FAST", "1")
            .output()
            .expect("Failed to execute subprocess test");

        assert!(!output.status.success(), "Subprocess should fail fast with exit code 1");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("RIP Critical Error: Failed to load NNUE network file 'eval_models/non_existent_model_file.bin'"),
            "Stderr should contain critical error log, got: {}",
            stderr
        );
    }

    #[test]
    fn test_insufficient_material_detection() {
        let fen_service = Service::new().fen;
        let eval_service = Service::new().eval;
        let movegen = &Service::new().move_gen;
        let mut config = Config::new();
        config.use_nnue = false;
        let pawn_table = crate::pawn_hash::PawnHashTable::new(16);

        // 1. KvK (Draw)
        let board_kvk = fen_service.set_fen("8/8/8/4k3/8/8/4K3/8 w - - 0 1");
        assert!(super::EvalService::is_insufficient_material(&board_kvk));
        assert_eq!(eval_service.calc_eval(&board_kvk, &config, movegen, &pawn_table, i16::MIN, i16::MAX, 180), 0);
        assert_eq!(eval_service.cheap_eval(&board_kvk, &config, &pawn_table), 0);

        // 2. KNvK (Draw)
        let board_knvk = fen_service.set_fen("8/8/8/4k3/8/5N2/4K3/8 w - - 0 1");
        assert!(super::EvalService::is_insufficient_material(&board_knvk));
        assert_eq!(eval_service.calc_eval(&board_knvk, &config, movegen, &pawn_table, i16::MIN, i16::MAX, 180), 0);
        assert_eq!(eval_service.cheap_eval(&board_knvk, &config, &pawn_table), 0);

        // 3. KBvK (Draw)
        let board_kbvk = fen_service.set_fen("8/8/8/4k3/8/5B2/4K3/8 w - - 0 1");
        assert!(super::EvalService::is_insufficient_material(&board_kbvk));
        assert_eq!(eval_service.calc_eval(&board_kbvk, &config, movegen, &pawn_table, i16::MIN, i16::MAX, 180), 0);
        assert_eq!(eval_service.cheap_eval(&board_kbvk, &config, &pawn_table), 0);

        // 4. KNNvK (Draw)
        let board_knnvk = fen_service.set_fen("8/8/8/4k3/8/5NN1/4K3/8 w - - 0 1");
        assert!(super::EvalService::is_insufficient_material(&board_knnvk));
        assert_eq!(eval_service.calc_eval(&board_knnvk, &config, movegen, &pawn_table, i16::MIN, i16::MAX, 180), 0);
        assert_eq!(eval_service.cheap_eval(&board_knnvk, &config, &pawn_table), 0);

        // 5. KBvKB same-color bishops (Draw)
        // c1 is dark (2+0=2%2=0), f4 is dark (5+3=8%2=0) -> same color
        let board_kbvkb_same = fen_service.set_fen("8/8/8/8/5b2/8/4K3/2B1k3 w - - 0 1");
        assert!(super::EvalService::is_insufficient_material(&board_kbvkb_same));
        assert_eq!(eval_service.calc_eval(&board_kbvkb_same, &config, movegen, &pawn_table, i16::MIN, i16::MAX, 180), 0);

        // 6. KBvKB opposite-color bishops (NOT insufficient material directly, but opposite bishops draw scale applies)
        // c1 is dark (2+0=2%2=0), c4 is light (2+3=5%2=1) -> opposite color
        let board_kbvkb_opp = fen_service.set_fen("8/8/8/8/2b5/8/4K3/2B1k3 w - - 0 1");
        assert!(!super::EvalService::is_insufficient_material(&board_kbvkb_opp));

        // 7. KPvK (Pawns present -> NOT insufficient material)
        let board_kpvk = fen_service.set_fen("8/8/8/4k3/8/5P2/4K3/8 w - - 0 1");
        assert!(!super::EvalService::is_insufficient_material(&board_kpvk));
        assert_ne!(eval_service.calc_eval(&board_kpvk, &config, movegen, &pawn_table, i16::MIN, i16::MAX, 180), 0);

        // 8. KRvK (Rooks present -> NOT insufficient material)
        let board_krvk = fen_service.set_fen("8/8/8/4k3/8/5R2/4K3/8 w - - 0 1");
        assert!(!super::EvalService::is_insufficient_material(&board_krvk));
        assert_ne!(eval_service.calc_eval(&board_krvk, &config, movegen, &pawn_table, i16::MIN, i16::MAX, 180), 0);
    }

    #[test]
    fn test_endgame_mopup_heuristics() {
        let fen_service = Service::new().fen;
        let eval_service = Service::new().eval;
        let movegen = &Service::new().move_gen;
        let mut config = Config::new();
        config.use_nnue = false;
        config.enable_endgame_mopup = true;
        let pawn_table = crate::pawn_hash::PawnHashTable::new(16);

        // Position A: White K+R vs Black K. Black King is on a8 (corner, rank 7, file 0) and White King is on b6 (close, rank 5, file 1).
        let board_cornered = fen_service.set_fen("k7/8/1K6/8/8/8/1R6/8 w - - 0 1");
        let eval_cornered = eval_service.calc_eval(&board_cornered, &config, movegen, &pawn_table, i16::MIN, i16::MAX, 180);

        // Position B: White K+R vs Black K. Black King is on d4 (center, rank 3, file 3) and White King is on h1 (distant).
        let board_centered = fen_service.set_fen("8/8/8/8/3k4/8/1R6/7K w - - 0 1");
        let eval_centered = eval_service.calc_eval(&board_centered, &config, movegen, &pawn_table, i16::MIN, i16::MAX, 180);

        assert!(
            eval_cornered > eval_centered + 50,
            "Cornered and closely contained enemy king should have significantly higher score due to Mop-Up (got cornered={}, centered={})",
            eval_cornered,
            eval_centered
        );

        // Test Mop-Up disabled when losing side has pawns
        let board_with_black_pawn = fen_service.set_fen("k7/p7/1K6/8/8/8/1R6/8 w - - 0 1");
        let mut config_no_mop = config.clone();
        config_no_mop.enable_endgame_mopup = false;

        let eval_with_pawn = eval_service.calc_eval(&board_with_black_pawn, &config, movegen, &pawn_table, i16::MIN, i16::MAX, 180);
        let eval_with_pawn_no_mop = eval_service.calc_eval(&board_with_black_pawn, &config_no_mop, movegen, &pawn_table, i16::MIN, i16::MAX, 180);
        assert_eq!(
            eval_with_pawn, eval_with_pawn_no_mop,
            "Mop-Up should not be applied if the losing side still has pawns"
        );

        // Test Black winning Mop-Up symmetry
        let board_black_winning = fen_service.set_fen("8/1r6/8/8/8/1k6/8/K7 b - - 0 1");
        let eval_black_winning = eval_service.calc_eval(&board_black_winning, &config, movegen, &pawn_table, i16::MIN, i16::MAX, 180);
        assert!(
            eval_black_winning < -500,
            "Black winning in pawnless endgame should have a strongly negative evaluation"
        );
    }

    #[test]
    fn test_endgame_mopup_edge_push_monotonic_gradient() {
        // The edge push alone: the White king stands two squares from the Black king in every
        // position and the rook stays on h1, so the proximity term is the same (5 x 15) and only
        // the distance of the Black king from the centre moves the bonus, 10 per step. Measured
        // as the mop-up bonus (with minus without), as the proximity test below does: on the
        // whole evaluation the king tables took part, and with the White king fixed on b1 the
        // proximity term cancelled the push - centre and semi-centre read 667 and 668 on v0.47.4
        // and tied after the material re-fit of `task.md` 29.19.
        let fen_service = Service::new().fen;
        let eval_service = Service::new().eval;
        let movegen = &Service::new().move_gen;
        let mut config = Config::new();
        config.use_nnue = false;
        config.enable_endgame_mopup = true;
        let pawn_table = crate::pawn_hash::PawnHashTable::new(16);

        let get_mopup_bonus = |fen: &str| -> i16 {
            let board = fen_service.set_fen(fen);
            let eval_with = eval_service.calc_eval(&board, &config, movegen, &pawn_table, i16::MIN, i16::MAX, 180);
            let mut config_no_mop = config.clone();
            config_no_mop.enable_endgame_mopup = false;
            let eval_without = eval_service.calc_eval(&board, &config_no_mop, movegen, &pawn_table, i16::MIN, i16::MAX, 180);
            eval_with - eval_without
        };

        // Centre distance as the rule counts it (the farther of the two central ranks and files):
        // d4 1 + 1 = 2, c5 1 + 2 = 3, c8 4 + 2 = 6, a8 4 + 4 = 8.
        let mop_center = get_mopup_bonus("8/8/8/8/3k4/8/3K4/7R w - - 0 1");
        let mop_semi = get_mopup_bonus("8/8/8/2k5/8/2K5/8/7R w - - 0 1");
        let mop_edge = get_mopup_bonus("2k5/8/2K5/8/8/8/8/7R w - - 0 1");
        let mop_corner = get_mopup_bonus("k7/8/K7/8/8/8/8/7R w - - 0 1");

        assert_eq!(mop_center, 2 * 10 + 5 * 15, "centre d4");
        assert_eq!(mop_semi, 3 * 10 + 5 * 15, "semi-centre c5");
        assert_eq!(mop_edge, 6 * 10 + 5 * 15, "edge c8");
        assert_eq!(mop_corner, 8 * 10 + 5 * 15, "corner a8");
        assert!(mop_center < mop_semi && mop_semi < mop_edge && mop_edge < mop_corner,
            "the bonus must grow towards the corner ({mop_center}, {mop_semi}, {mop_edge}, {mop_corner})");
    }

    #[test]
    fn test_endgame_mopup_king_proximity_monotonic_gradient() {
        let fen_service = Service::new().fen;
        let eval_service = Service::new().eval;
        let movegen = &Service::new().move_gen;
        let mut config = Config::new();
        config.use_nnue = false;
        config.enable_endgame_mopup = true;
        let pawn_table = crate::pawn_hash::PawnHashTable::new(16);

        let get_mopup_bonus = |fen: &str| -> i16 {
            let board = fen_service.set_fen(fen);
            let eval_with = eval_service.calc_eval(&board, &config, movegen, &pawn_table, i16::MIN, i16::MAX, 180);
            let mut config_no_mop = config.clone();
            config_no_mop.enable_endgame_mopup = false;
            let eval_without = eval_service.calc_eval(&board, &config_no_mop, movegen, &pawn_table, i16::MIN, i16::MAX, 180);
            eval_with - eval_without
        };

        // Enemy King fixed at a8, White Rook on h2.
        // White King moves progressively closer from distance 7 down to distance 1:
        // 1. Dist = 7 (White King at h1): proximity bonus = (7 - 7) * 15 = 0
        let mop_d7 = get_mopup_bonus("k7/8/8/8/8/8/7R/7K w - - 0 1");

        // 2. Dist = 5 (White King at f3): proximity bonus = (7 - 5) * 15 = 30
        let mop_d5 = get_mopup_bonus("k7/8/8/8/8/5K2/7R/8 w - - 0 1");

        // 3. Dist = 3 (White King at d5): proximity bonus = (7 - 3) * 15 = 60
        let mop_d3 = get_mopup_bonus("k7/8/8/3K4/8/8/7R/8 w - - 0 1");

        // 4. Dist = 2 (White King at c6): proximity bonus = (7 - 2) * 15 = 75
        let mop_d2 = get_mopup_bonus("k7/8/2K5/8/8/8/7R/8 w - - 0 1");

        // 5. Dist = 1 (White King at b7): proximity bonus = (7 - 1) * 15 = 90
        let mop_d1 = get_mopup_bonus("k7/1K6/8/8/8/8/7R/8 w - - 0 1");

        assert!(mop_d7 < mop_d5, "d5 bonus must exceed d7: {} vs {}", mop_d5, mop_d7);
        assert!(mop_d5 < mop_d3, "d3 bonus must exceed d5: {} vs {}", mop_d3, mop_d5);
        assert!(mop_d3 < mop_d2, "d2 bonus must exceed d3: {} vs {}", mop_d2, mop_d3);
        assert!(mop_d2 < mop_d1, "d1 bonus must exceed d2: {} vs {}", mop_d1, mop_d2);
    }

    #[test]
    fn test_endgame_mopup_disabled_when_losing_side_has_pieces() {
        let fen_service = Service::new().fen;
        let eval_service = Service::new().eval;
        let movegen = &Service::new().move_gen;
        let mut config = Config::new();
        config.use_nnue = false;
        config.enable_endgame_mopup = true;
        let pawn_table = crate::pawn_hash::PawnHashTable::new(16);

        let verify_mopup_inactive = |fen: &str, description: &str| {
            let board = fen_service.set_fen(fen);
            let eval_with = eval_service.calc_eval(&board, &config, movegen, &pawn_table, i16::MIN, i16::MAX, 180);
            let mut config_no_mop = config.clone();
            config_no_mop.enable_endgame_mopup = false;
            let eval_without = eval_service.calc_eval(&board, &config_no_mop, movegen, &pawn_table, i16::MIN, i16::MAX, 180);
            assert_eq!(
                eval_with, eval_without,
                "Mop-Up should be disabled when defending side has pieces: {}",
                description
            );
        };

        // 1. KQ vs KR (Black has Rook)
        verify_mopup_inactive("k7/8/8/8/8/8/1r6/1K1Q4 w - - 0 1", "KQ vs KR");

        // 2. KRB vs KR (Black has Rook)
        verify_mopup_inactive("k7/8/8/8/8/8/1r6/1KBR4 w - - 0 1", "KRB vs KR");

        // 3. KQ vs KQ (Black has Queen)
        verify_mopup_inactive("k7/8/8/8/8/8/1q6/1K1Q4 w - - 0 1", "KQ vs KQ");

        // 4. KR vs KN (Black has Knight)
        verify_mopup_inactive("k7/8/8/8/8/8/1n6/1K1R4 w - - 0 1", "KR vs KN");

        // 5. KR vs KB (Black has Bishop)
        verify_mopup_inactive("k7/8/8/8/8/8/1b6/1K1R4 w - - 0 1", "KR vs KB");
    }

    #[test]
    fn test_safe_mobility_excludes_enemy_pawn_attacks() {
        let fen_service = Service::new().fen;
        let eval_service = Service::new().eval;
        let movegen = &Service::new().move_gen;
        let mut config = Config::new();
        config.use_nnue = false;

        // Position A: White Knight on d4 with free squares (Black pawns on a7/h7 far away)
        let board_free_knight = fen_service.set_fen("7k/p6p/8/8/3N4/8/8/K7 w - - 0 1");
        let free_pawn_attacks = super::EvalService::get_black_pawn_attacks(board_free_knight.bitboards[crate::model::BLACK_PAWN]);
        let (free_knight_eval, _, _, _) = eval_service.white_knight(27, &board_free_knight, &config, 100, movegen, 0, 0, free_pawn_attacks, board_free_knight.white_pieces);

        // Position B: Black pawns on c6 and e6 control knight landing squares - restricting safe mobility
        let board_guarded = fen_service.set_fen("7k/8/2p1p3/8/3N4/8/8/K7 w - - 0 1");
        let guarded_pawn_attacks = super::EvalService::get_black_pawn_attacks(board_guarded.bitboards[crate::model::BLACK_PAWN]);
        let (guarded_knight_eval, _, _, _) = eval_service.white_knight(27, &board_guarded, &config, 100, movegen, 0, 0, guarded_pawn_attacks, board_guarded.white_pieces);

        assert!(
            free_knight_eval > guarded_knight_eval,
            "Knight with safe mobility should score higher than knight restricted by enemy pawns (free={}, guarded={})",
            free_knight_eval, guarded_knight_eval
        );
    }

    #[test]
    fn test_safe_mobility_excludes_friendly_blockers() {
        let fen_service = Service::new().fen;
        let eval_service = Service::new().eval;
        let movegen = &Service::new().move_gen;
        let mut config = Config::new();
        config.use_nnue = false;

        // Position A: White Rook on d4 on open board
        let board_open_rook = fen_service.set_fen("7k/8/8/8/3R4/8/8/K7 w - - 0 1");
        let (open_rook_eval, _, _, _) = eval_service.white_rook(27, &board_open_rook, &config, 100, movegen, 0, 0, board_open_rook.white_pieces);

        // Position B: White Rook on d4 hemmed in by friendly pawns on d3, d5, c4, e4
        let board_blocked_rook = fen_service.set_fen("7k/8/8/3P4/2PRP3/3P4/8/K7 w - - 0 1");
        let (blocked_rook_eval, _, _, _) = eval_service.white_rook(27, &board_blocked_rook, &config, 100, movegen, 0, 0, board_blocked_rook.white_pieces);

        assert!(
            open_rook_eval > blocked_rook_eval,
            "Open rook must have higher mobility evaluation than blocked rook (open={}, blocked={})",
            open_rook_eval, blocked_rook_eval
        );
    }

    #[test]
    fn test_queen_mobility_evaluation() {
        let fen_service = Service::new().fen;
        let eval_service = Service::new().eval;
        let movegen = &Service::new().move_gen;
        let mut config = Config::new();
        config.use_nnue = false;

        let board_open_queen = fen_service.set_fen("7k/8/8/8/3Q4/8/8/K7 w - - 0 1");
        let board_trapped_queen = fen_service.set_fen("7k/8/8/8/8/8/1PP5/KPQ5 w - - 0 1");

        let (open_q_eval, _, _, _) = eval_service.white_queen(27, &board_open_queen, &config, 128, movegen, 0, 0, board_open_queen.white_pieces);
        let (trapped_q_eval, _, _, _) = eval_service.white_queen(2, &board_trapped_queen, &config, 128, movegen, 0, 0, board_trapped_queen.white_pieces);

        assert!(
            open_q_eval > trapped_q_eval,
            "Central open Queen must evaluate with higher mobility than trapped corner Queen (open={}, trapped={})",
            open_q_eval, trapped_q_eval
        );
    }

    #[test]
    fn test_king_passer_proximity_monotonic_gradient() {
        let fen_service = Service::new().fen;
        let eval_service = Service::new().eval;
        let movegen = &Service::new().move_gen;
        let mut config = Config::new();
        config.use_nnue = false;
        config.king_passer_dist_weight = 12;
        let pawn_table = crate::pawn_hash::PawnHashTable::new(16);

        // White passed pawn on e5 (sq 36). Black king fixed far on h8.
        // Move White king progressively closer from a1 (dist 4) to d4 (dist 1):
        // 1. Dist = 4 (White king at a1, sq 0): w_dist = max(|4-0|, |4-0|) = 4
        let b1 = fen_service.set_fen("7k/8/8/4P3/8/8/8/K7 w - - 0 1");
        let e1 = eval_service.calc_eval(&b1, &config, movegen, &pawn_table, i16::MIN, i16::MAX, 180);

        // 2. Dist = 2 (White king at c3, sq 18): w_dist = max(|4-2|, |4-2|) = 2
        let b2 = fen_service.set_fen("7k/8/8/4P3/8/2K5/8/8 w - - 0 1");
        let e2 = eval_service.calc_eval(&b2, &config, movegen, &pawn_table, i16::MIN, i16::MAX, 180);

        // 3. Dist = 1 (White king at d4, sq 27): w_dist = max(|4-3|, |4-3|) = 1
        let b3 = fen_service.set_fen("7k/8/8/4P3/3K4/8/8/8 w - - 0 1");
        let e3 = eval_service.calc_eval(&b3, &config, movegen, &pawn_table, i16::MIN, i16::MAX, 180);

        assert!(e1 < e2, "Proximity distance 2 must exceed distance 4: e1={}, e2={}", e1, e2);
        assert!(e2 < e3, "Proximity distance 1 must exceed distance 2: e2={}, e3={}", e2, e3);
    }

    #[test]
    fn test_king_passer_proximity_black_white_symmetry() {
        let fen_service = Service::new().fen;
        let eval_service = Service::new().eval;
        let movegen = &Service::new().move_gen;
        let mut config = Config::new();
        config.use_nnue = false;
        config.king_passer_dist_weight = 12;
        let pawn_table = crate::pawn_hash::PawnHashTable::new(16);

        // White passed pawn on e5, White king on d4, Black king on h8
        let board_white = fen_service.set_fen("7k/8/8/4P3/3K4/8/8/8 w - - 0 1");
        let eval_w = eval_service.calc_eval(&board_white, &config, movegen, &pawn_table, i16::MIN, i16::MAX, 180);

        // Mirror: Black passed pawn on e4, Black king on d5, White king on h1
        let board_black = fen_service.set_fen("8/8/8/3k4/4p3/8/8/7K b - - 0 1");
        let eval_b = eval_service.calc_eval(&board_black, &config, movegen, &pawn_table, i16::MIN, i16::MAX, 180);

        assert_eq!(
            eval_w, -eval_b,
            "White and Black king passer proximity must be numerically symmetric: eval_w={}, eval_b={}",
            eval_w, eval_b
        );
    }

    #[test]
    fn test_extended_insufficient_material_detection() {
        let fen_service = Service::new().fen;

        // 1. KN vs KN
        let b_kn_kn = fen_service.set_fen("8/8/8/8/8/5n2/8/K1N4k w - - 0 1");
        assert!(super::EvalService::is_insufficient_material(&b_kn_kn), "KN vs KN must be insufficient material (draw)");

        // 2. KB vs KN
        let b_kb_kn = fen_service.set_fen("8/8/8/8/8/5n2/8/K1B4k w - - 0 1");
        assert!(super::EvalService::is_insufficient_material(&b_kb_kn), "KB vs KN must be insufficient material (draw)");

        // 3. KN vs KB
        let b_kn_kb = fen_service.set_fen("8/8/8/8/8/5b2/8/K1N4k w - - 0 1");
        assert!(super::EvalService::is_insufficient_material(&b_kn_kb), "KN vs KB must be insufficient material (draw)");

        // 4. KB vs KB (same-color bishops is draw; opposite-colored is scaled by opposite_bishops_draw_scale)
        let b_kb_kb_same = fen_service.set_fen("5b1k/8/8/8/8/8/8/2B4K w - - 0 1");
        assert!(super::EvalService::is_insufficient_material(&b_kb_kb_same), "Same-colored KB vs KB must be insufficient material (draw)");

        let b_kb_kb_opp = fen_service.set_fen("4b2k/8/8/8/8/8/8/2B4K w - - 0 1");
        assert!(!super::EvalService::is_insufficient_material(&b_kb_kb_opp), "Opposite-colored KB vs KB is not insufficient material directly");
    }

    #[test]
    fn test_pawn_phalanx_evaluation() {
        let fen_service = Service::new().fen;
        let eval_service = Service::new().eval;
        let movegen = &Service::new().move_gen;
        let mut config = Config::new();
        config.use_nnue = false;
        let pawn_table = crate::pawn_hash::PawnHashTable::new(16);

        // Position A: White pawn phalanx on d4 + e4 (rank 4)
        let board_phalanx = fen_service.set_fen("7k/8/8/8/3PP3/8/8/K7 w - - 0 1");
        let eval_phalanx = eval_service.calc_eval(&board_phalanx, &config, movegen, &pawn_table, i16::MIN, i16::MAX, 180);

        // Position B: Disjoint White pawns on d4 + a4 (no phalanx duo)
        let board_disjoint = fen_service.set_fen("7k/8/8/8/P2P4/8/8/K7 w - - 0 1");
        let eval_disjoint = eval_service.calc_eval(&board_disjoint, &config, movegen, &pawn_table, i16::MIN, i16::MAX, 180);

        assert!(
            eval_phalanx > eval_disjoint,
            "Pawn phalanx on d4+e4 must score higher than disjoint pawns on d4+a4 (phalanx={}, disjoint={})",
            eval_phalanx, eval_disjoint
        );
    }

    #[test]
    fn test_pawn_phalanx_black_white_symmetry() {
        let fen_service = Service::new().fen;
        let eval_service = Service::new().eval;
        let movegen = &Service::new().move_gen;
        let mut config = Config::new();
        config.use_nnue = false;
        let pawn_table = crate::pawn_hash::PawnHashTable::new(16);

        // White phalanx on d4 + e4 (rank 4), kings on h1/h8
        let board_w = fen_service.set_fen("7k/8/8/8/3PP3/8/8/7K w - - 0 1");
        let eval_w = eval_service.calc_eval(&board_w, &config, movegen, &pawn_table, i16::MIN, i16::MAX, 180);

        // Black phalanx on d5 + e5 (rank 5), kings on h8/h1
        let board_b = fen_service.set_fen("7k/8/8/3pp3/8/8/8/7K b - - 0 1");
        let eval_b = eval_service.calc_eval(&board_b, &config, movegen, &pawn_table, i16::MIN, i16::MAX, 180);

        assert_eq!(
            eval_w, -eval_b,
            "White and Black pawn phalanx must be numerically symmetric: eval_w={}, eval_b={}",
            eval_w, eval_b
        );
    }

    #[test]
    fn test_rook_open_file_attacks_king_and_queen() {
        let fen_service = Service::new().fen;
        let eval_service = Service::new().eval;
        let movegen = &Service::new().move_gen;
        let mut config = Config::new();
        config.use_nnue = false;

        // Position A: White Rook on e1 on open e-file facing Black Queen on e8 through a knight on
        // e5 (an x-ray: the bonus reads the file, not the attack). Knights on d5 and e5 in both
        // positions give both rooks the same mobility. Since the threats of step 49 B2 a direct
        // rook-on-queen attack is worth -67 in the rook's own term (the queen's attacked term and the
        // per-victim threats carry it), which hid the alignment bonus when the file was empty.
        let board_aligned_q = fen_service.set_fen("4q2k/8/8/3nn3/8/8/8/4R1K1 w - - 0 1");
        let (eval_aligned_q, _, _, _) = eval_service.white_rook(4, &board_aligned_q, &config, 150, movegen, 0, 0, board_aligned_q.white_pieces);

        // Position B: White Rook on d1 on open d-file with Black Queen on e8 (not aligned)
        let board_unaligned_q = fen_service.set_fen("4q2k/8/8/3nn3/8/8/8/3R2K1 w - - 0 1");
        let (eval_unaligned_q, _, _, _) = eval_service.white_rook(3, &board_unaligned_q, &config, 150, movegen, 0, 0, board_unaligned_q.white_pieces);

        assert!(
            eval_aligned_q > eval_unaligned_q,
            "Rook aligned with enemy Queen on open file must evaluate higher than unaligned rook (aligned={}, unaligned={})",
            eval_aligned_q, eval_unaligned_q
        );
    }

    #[test]
    fn test_king_open_file_heavy_threat_penalty() {
        let fen_service = Service::new().fen;
        let eval_service = Service::new().eval;
        let movegen = &Service::new().move_gen;
        let mut config = Config::new();
        config.use_nnue = false;

        // Position A: White King on e1 with open e-file facing Black Rook on e8 (heavy piece threat)
        let board_threat = fen_service.set_fen("4r2k/8/8/8/8/8/8/4K3 w - - 0 1");
        let (eval_threat, _, _) = eval_service.white_king(4, &board_threat, &config, 150, movegen, true);

        // Position B: White King on e1 with open e-file facing Black Knight on e8 (no heavy piece on file)
        let board_no_threat = fen_service.set_fen("4n2k/8/8/8/8/8/8/4K3 w - - 0 1");
        let (eval_no_threat, _, _) = eval_service.white_king(4, &board_no_threat, &config, 150, movegen, true);

        assert!(
            eval_threat < eval_no_threat,
            "King facing enemy Rook on open file must receive heavier malus than facing Knight (threat={}, no_threat={})",
            eval_threat, eval_no_threat
        );
    }

    #[test]
    fn test_bishop_diagonal_attacks_king_and_queen() {
        let fen_service = Service::new().fen;
        let eval_service = Service::new().eval;
        let movegen = &Service::new().move_gen;
        let mut config = Config::new();
        config.use_nnue = false;

        // Position A: White Bishop on b2 on open diagonal targeting Black King on h8 (aligned). The
        // king stands at the end of the diagonal, so the bishop's mobility is that of position B;
        // on g7 it took one square away, which cancelled the bonus once it was fitted to 13.
        let board_aligned_k = fen_service.set_fen("7k/8/8/8/8/8/1B6/7K w - - 0 1");
        let (eval_aligned_k, _, _, _) = eval_service.white_bishop(9, &board_aligned_k, &config, 150, movegen, 0, 0, 0, board_aligned_k.white_pieces);

        // Position B: White Bishop on b2 with Black King on h7 (not on b2 diagonal)
        let board_unaligned_k = fen_service.set_fen("8/7k/8/8/8/8/1B6/7K w - - 0 1");
        let (eval_unaligned_k, _, _, _) = eval_service.white_bishop(9, &board_unaligned_k, &config, 150, movegen, 0, 0, 0, board_unaligned_k.white_pieces);

        assert!(
            eval_aligned_k > eval_unaligned_k,
            "Bishop diagonally aligned with enemy King must evaluate higher than unaligned bishop (aligned={}, unaligned={})",
            eval_aligned_k, eval_unaligned_k
        );
    }

    #[test]
    fn test_rook_on_seventh_king_cutoff_and_doubled() {
        let fen_service = Service::new().fen;
        let eval_service = Service::new().eval;
        let movegen = &Service::new().move_gen;
        let mut config = Config::new();
        config.use_nnue = false;

        // Position A: White Rook on a7 with Black King cut off on e8 (8th rank)
        let board_cutoff = fen_service.set_fen("4k3/R7/8/8/8/8/8/4K3 w - - 0 1");
        let (eval_cutoff, _, _, _) = eval_service.white_rook(48, &board_cutoff, &config, 150, movegen, 0, 0, board_cutoff.white_pieces);

        // Position B: White Rook on a7 with Black King active on e6 (6th rank, not trapped on 8th)
        let board_nocutoff = fen_service.set_fen("8/R7/4k3/8/8/8/8/4K3 w - - 0 1");
        let (eval_nocutoff, _, _, _) = eval_service.white_rook(48, &board_nocutoff, &config, 150, movegen, 0, 0, board_nocutoff.white_pieces);

        assert!(
            eval_cutoff > eval_nocutoff,
            "Rook on 7th rank with enemy King cut off on 8th rank must score higher (cutoff={}, nocutoff={})",
            eval_cutoff, eval_nocutoff
        );
    }

    #[test]
    fn test_passed_pawn_blockade_penalty() {
        let fen_service = Service::new().fen;
        let eval_service = Service::new().eval;
        let movegen = &Service::new().move_gen;
        let mut config = Config::new();
        config.use_nnue = false;
        let pawn_table = crate::pawn_hash::PawnHashTable::new(16);

        // Position A: White passed pawn on e5 with Black Knight blockading on e6
        let board_blockaded = fen_service.set_fen("7k/8/4n3/4P3/8/8/8/7K w - - 0 1");
        let eval_blockaded = eval_service.calc_eval(&board_blockaded, &config, movegen, &pawn_table, i16::MIN, i16::MAX, 180);

        // Position B: White passed pawn on e5 with Black Knight unaligned on a6 (unblockaded)
        let board_free = fen_service.set_fen("7k/8/n7/4P3/8/8/8/7K w - - 0 1");
        let eval_free = eval_service.calc_eval(&board_free, &config, movegen, &pawn_table, i16::MIN, i16::MAX, 180);

        assert!(
            eval_free > eval_blockaded,
            "Free passed pawn must evaluate higher than blockaded passed pawn (free={}, blockaded={})",
            eval_free, eval_blockaded
        );
    }

    #[test]
    fn test_candidate_passed_pawn_evaluation() {
        let fen_service = Service::new().fen;
        let eval_service = Service::new().eval;
        let movegen = &Service::new().move_gen;
        let mut config = Config::new();
        config.use_nnue = false;
        let pawn_table = crate::pawn_hash::PawnHashTable::new(16);

        // Position A: White has candidate passed pawn on c4 supported by b3 and d3 against single Black pawn on b5 (2 vs 1 majority)
        let board_candidate = fen_service.set_fen("7k/8/8/1p6/2P5/1P1P4/8/7K w - - 0 1");
        let eval_candidate = eval_service.calc_eval(&board_candidate, &config, movegen, &pawn_table, i16::MIN, i16::MAX, 180);

        // Position B: Symmetrical pawns (no candidate passer)
        let board_equal = fen_service.set_fen("7k/8/8/1p1p4/2P5/1P1P4/8/7K w - - 0 1");
        let eval_equal = eval_service.calc_eval(&board_equal, &config, movegen, &pawn_table, i16::MIN, i16::MAX, 180);

        assert!(
            eval_candidate > eval_equal,
            "Candidate passed pawn with wing majority must score higher (candidate={}, equal={})",
            eval_candidate, eval_equal
        );
    }

    #[test]
    fn test_pawn_storm_evaluation() {
        let fen_service = Service::new().fen;
        let eval_service = Service::new().eval;
        let movegen = &Service::new().move_gen;
        let mut config = Config::new();
        config.use_nnue = false;
        let pawn_table = crate::pawn_hash::PawnHashTable::new(16);

        // Position A: White pawn storming on g4/h4 against Black King castled on g8
        let board_storm = fen_service.set_fen("6k1/8/8/8/6PP/8/8/7K w - - 0 1");
        let eval_storm = eval_service.calc_eval(&board_storm, &config, movegen, &pawn_table, i16::MIN, i16::MAX, 180);

        // Position B: White pawns on a4/b4 on opposite wing (no storm against g8 king)
        let board_nostorm = fen_service.set_fen("6k1/8/8/8/PP6/8/8/7K w - - 0 1");
        let eval_nostorm = eval_service.calc_eval(&board_nostorm, &config, movegen, &pawn_table, i16::MIN, i16::MAX, 180);

        assert!(
            eval_storm > eval_nostorm,
            "Flank pawn storm against enemy castled king must evaluate higher (storm={}, nostorm={})",
            eval_storm, eval_nostorm
        );
    }

    #[test]
    fn test_wrong_colored_bishop_rook_pawn_dead_draw() {
        let fen_service = Service::new().fen;

        // Position 1: White a-pawn (a5) + light-squared Bishop (c4, color 1) vs Black King on a8 (color 1).
        // C4 is light (sq 26: 2+3=5, color 1), A8 is light (sq 56: 0+7=7, color 1). Correct bishop controlling promotion square.
        let b_correct_bishop = fen_service.set_fen("k7/8/8/P7/2B5/8/8/7K w - - 0 1");
        assert!(!super::EvalService::is_insufficient_material(&b_correct_bishop), "Correct bishop controlling promotion square must not be insufficient material");

        // Position 2: White a-pawn (a5) + dark-squared Bishop (c3, color 0) vs Black King on a8 (color 1).
        // A8 is light (color 1), c3 is dark (sq 18: 2+2=4, color 0). Wrong-colored bishop + Black king in corner a8!
        let b_wrong_bishop = fen_service.set_fen("k7/8/8/P7/8/2B5/8/7K w - - 0 1");
        assert!(super::EvalService::is_insufficient_material(&b_wrong_bishop), "Wrong-colored bishop + rook pawn with enemy king in corner must be recognized as draw");

        // Position 3: Black h-pawn (h4) + dark-squared Bishop (c3, color 0) vs White King on h1 (color 1).
        // H1 is light (sq 7: 7+0=7, color 1), c3 is dark (sq 18: 2+2=4, color 0). Wrong-colored bishop + White king on h1!
        let b_wrong_black = fen_service.set_fen("7k/8/8/8/7p/2b5/8/7K w - - 0 1");
        assert!(super::EvalService::is_insufficient_material(&b_wrong_black), "Black wrong-colored bishop + rook pawn with White king on h1 must be recognized as draw");
    }

    #[test]
    fn test_all_new_eval_features_black_white_symmetry() {
        let fen_service = Service::new().fen;
        let eval_service = Service::new().eval;
        let movegen = &Service::new().move_gen;
        let mut config = Config::new();
        config.use_nnue = false;
        let pawn_table = crate::pawn_hash::PawnHashTable::new(16);

        // White pawn storm g4/h4 against g8 king
        let board_w = fen_service.set_fen("6k1/8/8/8/6PP/8/8/7K w - - 0 1");
        let eval_w = eval_service.calc_eval(&board_w, &config, movegen, &pawn_table, i16::MIN, i16::MAX, 180);

        // Black pawn storm g5/h5 against g1 king
        let board_b = fen_service.set_fen("7k/8/8/6pp/8/8/8/6K1 b - - 0 1");
        let eval_b = eval_service.calc_eval(&board_b, &config, movegen, &pawn_table, i16::MIN, i16::MAX, 180);

        assert_eq!(
            eval_w, -eval_b,
            "White and Black evaluation must be numerically symmetric: eval_w={}, eval_b={}",
            eval_w, eval_b
        );
    }

    /// The same position with the colours exchanged and the board flipped top to bottom, which is
    /// strategically identical for the other side. Mirrors `swap_colours` in
    /// `scripts/verify_negamax_identity.py`.
    fn mirror_fen(fen: &str) -> String {
        let parts: Vec<&str> = fen.split_whitespace().collect();
        let board: String = parts[0]
            .split('/')
            .rev()
            .map(|rank| {
                rank.chars()
                    .map(|c| {
                        if c.is_ascii_uppercase() {
                            c.to_ascii_lowercase()
                        } else if c.is_ascii_lowercase() {
                            c.to_ascii_uppercase()
                        } else {
                            c
                        }
                    })
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("/");
        let side = if parts[1] == "w" { "b" } else { "w" };
        let castling = if parts[2] == "-" {
            "-".to_string()
        } else {
            let mut rights: Vec<char> = parts[2]
                .chars()
                .map(|c| if c.is_ascii_uppercase() { c.to_ascii_lowercase() } else { c.to_ascii_uppercase() })
                .collect();
            rights.sort_unstable();
            rights.into_iter().collect()
        };
        let en_passant = if parts[3] == "-" {
            "-".to_string()
        } else {
            let rank: u32 = parts[3][1..2].parse().expect("en passant rank");
            format!("{}{}", &parts[3][0..1], 9 - rank)
        };
        format!("{} {} {} {} {} {}", board, side, castling, en_passant, parts[4], parts[5])
    }

    fn eval_of(fen: &str, config: &Config, alpha: i16, beta: i16) -> i16 {
        let service = Service::new();
        let board = service.fen.set_fen(fen);
        let pawn_table = crate::pawn_hash::PawnHashTable::new(16);
        service.eval.calc_eval(&board, config, &service.move_gen, &pawn_table, alpha, beta,
                               config.lazy_eval_margin_search)
    }

    /// A position and its colour mirror must evaluate to exact negatives of each other.
    ///
    /// The candidate-passed-pawn rule used to break this: it counted enemy pawns on the adjacent
    /// files with an *index* threshold, which on an adjacent-files mask also admits the pawn's
    /// own rank - `sq + 1` for White and `sq - 1` for Black. A colour mirror flips the rank and
    /// leaves the file alone, so the two masks were not mirror images and the same structure
    /// scored 8 cp (middlegame) / 16 cp (endgame) differently depending on which colour held it.
    /// The four positions below are drawn from the book sweep that found it; each one moved
    /// before the mask became rank-based. See `task.md` 7.2.
    #[test]
    fn test_candidate_passed_pawn_mask_is_colour_symmetric() {
        let mut config = Config::new();
        config.use_nnue = false;

        // The infinite window plus windows that put Lazy Evaluation in play, because the search
        // never calls `calc_eval` with an infinite window.
        let windows: [(i16, i16); 4] = [(i16::MIN, i16::MAX), (-50, 50), (0, 1), (-1000, 1000)];

        let positions = [
            "r1bqkb1r/1ppp1ppp/p1n2n2/1B6/3pP3/5N2/PPP2PPP/RNBQ1RK1 w kq - 0 6",
            "r1bqkb1r/pp1npppp/3p1n2/1B6/3pP3/2N2N2/PPP2PPP/R1BQK2R w KQkq - 0 6",
            "r1bqkb1r/pppp1ppp/2n2n2/1B6/3pP3/5N2/PPP2PPP/RNBQ1RK1 b kq - 0 5",
            "r1bqkbnr/pp1p1ppp/2n5/2p1p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 0 4",
        ];

        for fen in positions.iter() {
            let mirrored = mirror_fen(fen);
            for (alpha, beta) in windows.iter() {
                let direct = eval_of(fen, &config, *alpha, *beta);
                // A Black node hands `calc_eval` the window restated on the absolute scale, which
                // swaps the bounds and changes their sign.
                let mirror = eval_of(&mirrored, &config, beta.saturating_neg(), alpha.saturating_neg());
                assert_eq!(
                    direct, -mirror,
                    "mirrored positions must evaluate to exact negatives at window ({}, {}):\n  \
                     {} -> {}\n  {} -> {}",
                    alpha, beta, fen, direct, mirrored, mirror);
            }
        }
    }

    /// The mask itself, on a position built to isolate it.
    ///
    /// White has a candidate pawn on d5 supported on the e-file, with black pawns on c7 (which
    /// denies it passed status) and on e5 - the neighbour on the candidate's *own* rank. e5 is
    /// beside the candidate, not in front of it, and must not count as an enemy in its path. The
    /// index threshold used to count it for White and not for the mirrored Black pawn, which is
    /// exactly the leak this test pins.
    #[test]
    fn test_candidate_passed_pawn_ignores_same_rank_enemy_pawns() {
        let fen = "4k3/2p5/8/3Pp3/4P3/8/8/4K3 w - - 0 1";
        let mirrored = mirror_fen(fen);
        assert_eq!(mirrored, "4k3/8/8/4p3/3pP3/8/2P5/4K3 b - - 0 1");

        let mut config = Config::new();
        config.use_nnue = false;

        // The rule must actually be engaged on this position, or the symmetry assertion below
        // would hold trivially on a position the candidate branch never reaches.
        let mut without = config.clone();
        without.candidate_passed_pawn_bonus = 0;
        assert_ne!(
            eval_of(fen, &config, i16::MIN, i16::MAX),
            eval_of(fen, &without, i16::MIN, i16::MAX),
            "the candidate passed pawn rule must fire on this position");

        assert_eq!(
            eval_of(fen, &config, i16::MIN, i16::MAX),
            -eval_of(&mirrored, &config, i16::MIN, i16::MAX),
            "an enemy pawn on the candidate's own rank must not count as being in front of it");
    }

    /// Every position of `games` random legal games of up to `plies` plies, from a fixed seed so
    /// that a failure reproduces.
    fn random_positions(games: usize, plies: usize, seed: u64) -> Vec<crate::model::Board> {
        let service = Service::new();
        let config = Config::for_tests();
        let zobrist_table = crate::zobrist::ZobristTable::with_capacity(1);
        let stop_flag = std::sync::atomic::AtomicBool::new(false);
        let pv_nodes = std::sync::Mutex::new(std::collections::HashMap::new());
        let history_table = [[[0i32; 64]; 64]; 2];
        let context = crate::model::SearchContext {
            zobrist_table: &zobrist_table,
            stop_flag: &stop_flag,
            pv_nodes: &pv_nodes,
            killer_moves: [None; 2],
            history_table: &history_table,
            counter_move: None,
            start_time: std::time::Instant::now(),
            target_time: None,
            root_moves_total: 0,
            root_moves_searched: 0,
            root_depth: 0,
        };
        let mut state = seed;
        let mut positions = Vec::new();
        for _ in 0..games {
            let mut board = service.fen.set_init_board();
            for _ in 0..plies {
                let mut moves = crate::model::MoveList::new();
                service.move_gen.generate_valid_moves_list(
                    &mut board, &mut crate::model::Stats::new(), &config, &context, false, &mut moves);
                if moves.is_empty() {
                    break;
                }
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                let mv = moves.moves[(state % moves.len as u64) as usize];
                board.do_move(&mv);
                positions.push(board.clone());
            }
        }
        positions
    }

    #[test]
    fn test_the_set_wise_pawn_terms_equal_the_per_pawn_form() {
        // `pawn_dynamic_scores` and `true_outposts` replace a call per pawn and a scan per square.
        // They are only an optimisation if they agree with that form everywhere, which is checked
        // here against the per-pawn and per-square originals over random games.
        use crate::model::{WHITE_PAWN, BLACK_PAWN};
        let service = Service::new();
        let eval = &service.eval;
        let config = Config::new();
        let positions = random_positions(60, 160, 0x9E37_79B9_7F4A_7C15);
        assert!(positions.len() > 5000, "only {} positions", positions.len());
        for board in &positions {
            let white_pawns = board.bitboards[WHITE_PAWN];
            let black_pawns = board.bitboards[BLACK_PAWN];
            let mut white_passed = 0u64;
            let mut black_passed = 0u64;
            let (mut mg, mut eg) = (0i16, 0i16);
            for sq in 0..64u8 {
                if white_pawns & (1u64 << sq) != 0 && eval.is_white_passed_pawn(sq, board) {
                    white_passed |= 1u64 << sq;
                }
                if black_pawns & (1u64 << sq) != 0 && eval.is_black_passed_pawn(sq, board) {
                    black_passed |= 1u64 << sq;
                }
            }
            for sq in 0..64u8 {
                if white_pawns & (1u64 << sq) != 0 {
                    let (m, e) = eval.white_pawn_dynamic_score(sq, board, &config, white_passed);
                    mg += m;
                    eg += e;
                }
                if black_pawns & (1u64 << sq) != 0 {
                    let (m, e) = eval.black_pawn_dynamic_score(sq, board, &config, black_passed);
                    mg += m;
                    eg += e;
                }
            }
            let fen = service.fen.get_fen(board);
            assert_eq!(super::EvalService::pawn_dynamic_scores(board, &config, white_passed, black_passed),
                (mg, eg), "dynamic pawn terms differ in {fen}");

            for white in [true, false] {
                let mut reference = 0u64;
                for sq in 0..64u8 {
                    if eval.is_true_outpost(sq, white, board) {
                        reference |= 1u64 << sq;
                    }
                }
                assert_eq!(super::EvalService::true_outposts(white_pawns, black_pawns, white), reference,
                    "outposts for {} differ in {fen}", if white { "White" } else { "Black" });
            }
        }
    }

    #[test]
    fn test_the_known_check_evaluation_equals_the_reference() {
        // The search tells the evaluation whether the side to move is in check instead of having it
        // test, and skips `cheap_eval` where no lazy return is possible. Over random positions,
        // in check and not, and over open and narrow windows, the value must be the reference's.
        // Each call gets an empty pawn table, so neither reads what the other stored.
        let service = Service::new();
        let config = Config::new();
        let positions = random_positions(30, 160, 0xD1B5_4A32_D192_ED03);
        let windows = [(i16::MIN, i16::MAX), (-40, 40), (150, 151), (-400, -399), (900, 901)];
        let mut in_check_seen = 0;
        for board in &positions {
            let in_check = service.move_gen.is_in_check(board);
            in_check_seen += in_check as usize;
            for (alpha, beta) in windows {
                for margin in [120, 180] {
                    let reference = service.eval.calc_eval(board, &config, &service.move_gen,
                        &crate::pawn_hash::PawnHashTable::new(16), alpha, beta, margin);
                    let known = service.eval.calc_eval_known_check(board, &config, &service.move_gen,
                        &crate::pawn_hash::PawnHashTable::new(16), alpha, beta, margin, in_check);
                    assert_eq!(known, reference, "{} at ({alpha}, {beta}) margin {margin}",
                        service.fen.get_fen(board));
                }
            }
        }
        assert!(in_check_seen > 50, "the corpus must contain positions in check, saw {in_check_seen}");
    }

    #[test]
    fn test_opposite_bishops_scaling_keeps_the_sign_of_a_winning_evaluation() {
        // Four connected passed pawns against a bishop of the other colour. The draw scale was
        // multiplied in `i16`, which wraps above 642 centipawns at the default of 51 and turned
        // this evaluation negative for White.
        let service = Service::new();
        let config = Config::new();
        let board = service.fen.set_fen("4k3/8/8/PPPP4/8/8/3B4/4K2b w - - 0 1");
        let eval = service.eval.calc_eval(&board, &config, &service.move_gen,
            &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX, 180);
        assert!(eval > 300, "White is winning, got {eval}");
    }

    #[test]
    fn test_the_knight_attack_bonuses_follow_the_configuration() {
        // The four knight-attack parameters were copied out of the defaults when the evaluation was
        // built, so `setoption` and the tuner changed a `Config` field that nothing read.
        let service = Service::new();
        let board = service.fen.set_fen("8/1k6/4r3/8/5N2/1K6/8/8 w - - 0 1");
        let eval_with = |shape: &dyn Fn(&mut Config)| {
            let mut config = Config::new();
            shape(&mut config);
            service.eval.calc_eval(&board, &config, &service.move_gen,
                &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX, 180)
        };
        let default = eval_with(&|_| {});
        let bigger = eval_with(&|c| c.knight_attacks_rook += 200);
        assert!(bigger > default, "KnightAttacksRook must reach the evaluation: {default} -> {bigger}");
    }
}
