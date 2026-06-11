use crate::config::Config;
use crate::model::Board;
use crate::move_gen_service::MoveGenService;


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

pub struct EvalService {
    _knight_moves: [i16; 8],
    attack_bonus_white: [(i16, i16, i16); 2],
    attack_bonus_black: [(i16, i16, i16); 2],
}

impl EvalService {

    pub fn new(config: &Config) -> Self {
        Self {
            _knight_moves: [-21, -19, -12, -8, 21, 19, 12, 8],
            attack_bonus_white: [
                (21, config.knight_attacks_rook, config.knight_attacks_rook_tempo),
                (23, config.knight_attacks_bishop, config.knight_attacks_bishop_tempo),
            ],
            attack_bonus_black: [
                (11, config.knight_attacks_rook, config.knight_attacks_rook_tempo),
                (13, config.knight_attacks_bishop, config.knight_attacks_bishop_tempo),
            ],
        }
    }

    pub fn _set_custom_config(&mut self, config: &Config) {
        self.attack_bonus_white = [
            (21, config.knight_attacks_rook, config.knight_attacks_rook_tempo),
            (23, config.knight_attacks_bishop, config.knight_attacks_bishop_tempo),
        ];
        self.attack_bonus_black = [
            (11, config.knight_attacks_rook, config.knight_attacks_rook_tempo),
            (13, config.knight_attacks_bishop, config.knight_attacks_bishop_tempo),
        ];
    }

    pub fn calc_eval(&self, board: &Board, config: &Config, movegen: &MoveGenService, pawn_table: &crate::pawn_hash::PawnHashTable, alpha: i16, beta: i16) -> i16 {
        let mut scaled_config;
        let config = if config.aggressiveness == crate::config::Aggressiveness::Normal {
            config
        } else {
            scaled_config = config.clone();
            match config.aggressiveness {
                crate::config::Aggressiveness::Normal => {}
                crate::config::Aggressiveness::Aggressive => {
                    scaled_config.king_ring_attack_knight = (config.king_ring_attack_knight * 15) / 10;
                    scaled_config.king_ring_attack_bishop = (config.king_ring_attack_bishop * 15) / 10;
                    scaled_config.king_ring_attack_rook = (config.king_ring_attack_rook * 15) / 10;
                    scaled_config.king_ring_attack_queen = (config.king_ring_attack_queen * 15) / 10;
                    scaled_config.queen_in_attack = (config.queen_in_attack * 13) / 10;
                    scaled_config.queen_in_attack_with_tempo = (config.queen_in_attack_with_tempo * 13) / 10;
                    scaled_config.knight_mobility_factor = (config.knight_mobility_factor * 12) / 10;
                    scaled_config.bishop_mobility_factor = (config.bishop_mobility_factor * 12) / 10;
                    scaled_config.rook_mobility_factor = (config.rook_mobility_factor * 12) / 10;
                }
                crate::config::Aggressiveness::HighAggressive => {
                    scaled_config.king_ring_attack_knight = config.king_ring_attack_knight * 2;
                    scaled_config.king_ring_attack_bishop = config.king_ring_attack_bishop * 2;
                    scaled_config.king_ring_attack_rook = config.king_ring_attack_rook * 2;
                    scaled_config.king_ring_attack_queen = config.king_ring_attack_queen * 2;
                    scaled_config.queen_in_attack = (config.queen_in_attack * 16) / 10;
                    scaled_config.queen_in_attack_with_tempo = (config.queen_in_attack_with_tempo * 16) / 10;
                    scaled_config.knight_mobility_factor = (config.knight_mobility_factor * 14) / 10;
                    scaled_config.bishop_mobility_factor = (config.bishop_mobility_factor * 14) / 10;
                    scaled_config.rook_mobility_factor = (config.rook_mobility_factor * 14) / 10;
                }
            }
            &scaled_config
        };
        let game_phase = self.get_game_phase(board) as i16;
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

        if let Some((cached_mg, cached_eg)) = pawn_table.get(board.pawn_key) {
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
        let mut dyn_mg = 0;
        let mut dyn_eg = 0;
        let mut temp_w = board.bitboards[crate::model::WHITE_PAWN];
        while temp_w != 0 {
            let sq = temp_w.trailing_zeros() as u8;
            let (mg, eg) = self.white_pawn_dynamic_score(sq, board, config, white_passed_pawns);
            dyn_mg += mg;
            dyn_eg += eg;
            temp_w &= temp_w - 1;
        }
        let mut temp_b = board.bitboards[crate::model::BLACK_PAWN];
        while temp_b != 0 {
            let sq = temp_b.trailing_zeros() as u8;
            let (mg, eg) = self.black_pawn_dynamic_score(sq, board, config, black_passed_pawns);
            dyn_mg += mg;
            dyn_eg += eg;
            temp_b &= temp_b - 1;
        }

        eval += self.calculate_weighted_eval(struct_mg + dyn_mg, struct_eg + dyn_eg, game_phase);

        // Lazy Evaluation Pruning
        if config.enable_lazy_eval {
            let margin = config.lazy_eval_margin;
            if eval + margin <= alpha {
                return alpha;
            }
            if eval - margin >= beta {
                return beta;
            }
        }

        // Precalculate true outpost squares
        let mut white_true_outposts = 0u64;
        let mut black_true_outposts = 0u64;
        for r in 3..=5 {
            for f in 0..8 {
                let sq = r * 8 + f;
                if self.is_true_outpost(sq as u8, true, board) {
                    white_true_outposts |= 1u64 << sq;
                }
            }
        }
        for r in 2..=4 {
            for f in 0..8 {
                let sq = r * 8 + f;
                if self.is_true_outpost(sq as u8, false, board) {
                    black_true_outposts |= 1u64 << sq;
                }
            }
        }

        let white_king_sq = board.bitboards[crate::model::WHITE_KING].trailing_zeros() as u8;
        let black_king_sq = board.bitboards[crate::model::BLACK_KING].trailing_zeros() as u8;
        let white_king_ring = self.get_king_ring(white_king_sq);
        let black_king_ring = self.get_king_ring(black_king_sq);

        let mut white_attackers = 0;
        let mut black_attackers = 0;
        let mut white_king_danger = 0;
        let mut black_king_danger = 0;

        let mut temp = board.occupied;
        while temp != 0 {
            let sq = temp.trailing_zeros() as u8;
            let piece = board.get_piece_at(sq);
            let (eval_for_piece, attackers, danger) = match piece {
                10 | 20 => (0, 0, 0), // Already calculated separately!
                11 => self.white_rook(sq, board, config, game_phase, movegen, black_king_ring),
                12 => self.white_knight(sq, board, config, game_phase, movegen, black_king_ring, white_true_outposts),
                13 => self.white_bishop(sq, board, config, game_phase, movegen, black_king_ring, white_true_outposts),
                14 => self.white_queen(sq, board, config, game_phase, movegen, black_king_ring),
                15 => self.white_king(sq, board, config, game_phase, movegen),
                21 => self.black_rook(sq, board, config, game_phase, movegen, white_king_ring),
                22 => self.black_knight(sq, board, config, game_phase, movegen, white_king_ring, black_true_outposts),
                23 => self.black_bishop(sq, board, config, game_phase, movegen, white_king_ring, black_true_outposts),
                24 => self.black_queen(sq, board, config, game_phase, movegen, white_king_ring),
                25 => self.black_king(sq, board, config, game_phase, movegen),
                _ => (0, 0, 0),
            };
            if config.print_eval_per_figure && piece > 0 {
                println!("{},\t{},\t{}", sq, piece, eval_for_piece);
            }
            eval += eval_for_piece;
            if piece < 20 && piece > 0 {
                white_attackers += attackers;
                white_king_danger += danger;
            } else if piece >= 20 {
                black_attackers += attackers;
                black_king_danger += danger;
            }
            temp &= temp - 1;
        }

        // Apply King Danger Weights (Task 1.4)
        let danger_weights = [
            0,
            config.king_danger_weight_1,
            config.king_danger_weight_2,
            config.king_danger_weight_3,
            config.king_danger_weight_4,
            config.king_danger_weight_5,
        ];
        
        let mut white_defenders = 0;
        let mut black_defenders = 0;
        
        if white_attackers > 0 || black_attackers > 0 {
            let mut temp = board.bitboards[crate::model::WHITE_KNIGHT] | board.bitboards[crate::model::WHITE_BISHOP];
            while temp != 0 {
                let sq = temp.trailing_zeros() as usize;
                let piece = board.get_piece_at(sq as u8);
                let attacks = if piece == 12 { movegen.get_knight_attacks(sq) } else { movegen.get_bishop_attacks(sq, board.occupied) };
                if (attacks & white_king_ring) != 0 { white_defenders += 1; }
                temp &= temp - 1;
            }
            
            let mut temp = board.bitboards[crate::model::BLACK_KNIGHT] | board.bitboards[crate::model::BLACK_BISHOP];
            while temp != 0 {
                let sq = temp.trailing_zeros() as usize;
                let piece = board.get_piece_at(sq as u8);
                let attacks = if piece == 22 { movegen.get_knight_attacks(sq) } else { movegen.get_bishop_attacks(sq, board.occupied) };
                if (attacks & black_king_ring) != 0 { black_defenders += 1; }
                temp &= temp - 1;
            }
        }

        if white_attackers > 0 {
            let effective_attackers = (white_attackers as i16 - (black_defenders as i16 * config.king_ring_defender_value)).max(0) as usize;
            let idx = std::cmp::min(effective_attackers, 5);
            eval += (white_king_danger * danger_weights[idx]) / 100;
        }
        if black_attackers > 0 {
            let effective_attackers = (black_attackers as i16 - (white_defenders as i16 * config.king_ring_defender_value)).max(0) as usize;
            let idx = std::cmp::min(effective_attackers, 5);
            eval -= (black_king_danger * danger_weights[idx]) / 100;
        }

        // Connected passed pawns bonus
        let mut white_connected_passed_pawns = 0;
        let mut temp_w_passed = white_passed_pawns;
        while temp_w_passed != 0 {
            let sq1 = temp_w_passed.trailing_zeros() as i32;
            let file1 = sq1 % 8;
            let rank1 = sq1 / 8;
            let mut other_passed = white_passed_pawns & !(1u64 << sq1);
            let mut is_connected = false;
            while other_passed != 0 {
                let sq2 = other_passed.trailing_zeros() as i32;
                let file2 = sq2 % 8;
                let rank2 = sq2 / 8;
                if (file1 - file2).abs() == 1 && (rank1 - rank2).abs() <= 1 {
                    is_connected = true;
                    break;
                }
                other_passed &= other_passed - 1;
            }
            if is_connected {
                white_connected_passed_pawns += 1;
            }
            temp_w_passed &= temp_w_passed - 1;
        }
        
        let mut black_connected_passed_pawns = 0;
        let mut temp_b_passed = black_passed_pawns;
        while temp_b_passed != 0 {
            let sq1 = temp_b_passed.trailing_zeros() as i32;
            let file1 = sq1 % 8;
            let rank1 = sq1 / 8;
            let mut other_passed = black_passed_pawns & !(1u64 << sq1);
            let mut is_connected = false;
            while other_passed != 0 {
                let sq2 = other_passed.trailing_zeros() as i32;
                let file2 = sq2 % 8;
                let rank2 = sq2 / 8;
                if (file1 - file2).abs() == 1 && (rank1 - rank2).abs() <= 1 {
                    is_connected = true;
                    break;
                }
                other_passed &= other_passed - 1;
            }
            if is_connected {
                black_connected_passed_pawns += 1;
            }
            temp_b_passed &= temp_b_passed - 1;
        }

        if white_connected_passed_pawns > 0 || black_connected_passed_pawns > 0 {
            let o_bonus = (white_connected_passed_pawns - black_connected_passed_pawns) as i16 * config.connected_passed_pawn_mg;
            let e_bonus = (white_connected_passed_pawns - black_connected_passed_pawns) as i16 * config.connected_passed_pawn_eg;
            eval += self.calculate_weighted_eval(o_bonus, e_bonus, game_phase);
        }

        let mut o_bishop_pair = 0;
        let mut e_bishop_pair = 0;
        if board.bitboards[crate::model::WHITE_BISHOP].count_ones() >= 2 {
            o_bishop_pair += config.bishop_pair_bonus;
            e_bishop_pair += config.bishop_pair_bonus + 15;
        }
        if board.bitboards[crate::model::BLACK_BISHOP].count_ones() >= 2 {
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
                    eval -= config.king_opposition_bonus;
                } else {
                    // White holds opposition
                    eval += config.king_opposition_bonus;
                }
            }
        }

        eval += if board.white_to_move { config.your_turn_bonus } else { -config.your_turn_bonus };

        if config.enable_positional_cap {
            let mut material_eval: i16 = 0;
            material_eval += (board.bitboards[crate::model::WHITE_PAWN].count_ones() as i16) * crate::pst::PIECE_EVAL_PAWN;
            material_eval += (board.bitboards[crate::model::WHITE_ROOK].count_ones() as i16) * crate::pst::PIECE_EVAL_ROOK;
            material_eval += (board.bitboards[crate::model::WHITE_KNIGHT].count_ones() as i16) * crate::pst::PIECE_EVAL_KNIGHT;
            material_eval += (board.bitboards[crate::model::WHITE_BISHOP].count_ones() as i16) * crate::pst::PIECE_EVAL_BISHOP;
            material_eval += (board.bitboards[crate::model::WHITE_QUEEN].count_ones() as i16) * crate::pst::PIECE_EVAL_QUEEN;
            material_eval += (board.bitboards[crate::model::WHITE_KING].count_ones() as i16) * crate::pst::PIECE_EVAL_KING;

            material_eval -= (board.bitboards[crate::model::BLACK_PAWN].count_ones() as i16) * crate::pst::PIECE_EVAL_PAWN;
            material_eval -= (board.bitboards[crate::model::BLACK_ROOK].count_ones() as i16) * crate::pst::PIECE_EVAL_ROOK;
            material_eval -= (board.bitboards[crate::model::BLACK_KNIGHT].count_ones() as i16) * crate::pst::PIECE_EVAL_KNIGHT;
            material_eval -= (board.bitboards[crate::model::BLACK_BISHOP].count_ones() as i16) * crate::pst::PIECE_EVAL_BISHOP;
            material_eval -= (board.bitboards[crate::model::BLACK_QUEEN].count_ones() as i16) * crate::pst::PIECE_EVAL_QUEEN;
            material_eval -= (board.bitboards[crate::model::BLACK_KING].count_ones() as i16) * crate::pst::PIECE_EVAL_KING;

            let positional_eval = eval - material_eval;

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
            eval = material_eval + capped_positional;
        }

        if Self::is_opposite_colored_bishops_endgame(board) {
            eval = (eval * config.opposite_bishops_draw_scale) / 100;
        }

        eval = self.adjust_eval(eval, game_phase, config);

        if config.print_eval_per_figure {
            println!("{}", eval);
        }
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

        let white_pawns = board.bitboards[crate::model::WHITE_PAWN];
        if (file < 7 && ((1u64 << (sq + 9)) & white_pawns) != 0) ||
           (file > 0 && ((1u64 << (sq + 7)) & white_pawns) != 0) {
            o_eval += config.pawn_structure;
        }



        if moves_until_promote >= 5 && (file == 3 || file == 4) {
            o_eval -= config.pawn_undeveloped_malus;
        }



        let front_mask = (0x0101010101010101u64 << file) & !((1u64 << (sq + 1)) - 1);
        let has_doubled_pawn = (white_pawns & front_mask) != 0;
        if has_doubled_pawn {
            o_eval -= config.pawn_double_malus;
            e_eval -= config.pawn_double_malus;
        }

        let is_passed = (1u64 << sq) & precalculated_passed_pawns != 0;
        if is_passed {
            let bonus = match rank {
                1 => 10,
                2 => 40,
                3 => 90,
                4 => 180,
                5 => 320,
                6 => 500,
                _ => 0,
            };
            let is_protected = (file > 0 && ((1u64 << (sq - 9)) & white_pawns) != 0) ||
                               (file < 7 && ((1u64 << (sq - 7)) & white_pawns) != 0);
            if is_protected {
                o_eval += config.protected_passed_pawn_middlegame;
                e_eval += config.protected_passed_pawn_endgame;
            }



            e_eval += bonus;
            o_eval += bonus / 3;
        }

        let adjacent_files = ADJACENT_FILES_MASK[file as usize];
        let is_isolated = (white_pawns & adjacent_files) == 0;
        if is_isolated {
            o_eval -= config.pawn_isolated_malus;
            e_eval -= config.pawn_isolated_malus + 15;
        }
        
        let behind_and_same_rank_mask = (1u64 << (rank * 8 + 8)) - 1;
        let is_backward = !is_isolated && (white_pawns & adjacent_files & behind_and_same_rank_mask) == 0;
        if is_backward {
            o_eval -= config.pawn_backward_malus;
            e_eval -= config.pawn_backward_malus + 10;
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

        let black_pawns = board.bitboards[crate::model::BLACK_PAWN];
        if (file > 0 && sq >= 9 && ((1u64 << (sq - 9)) & black_pawns) != 0) ||
           (file < 7 && sq >= 7 && ((1u64 << (sq - 7)) & black_pawns) != 0) {
            o_eval -= config.pawn_structure;
        }



        if moves_until_promote >= 5 && (file == 3 || file == 4) {
            o_eval += config.pawn_undeveloped_malus;
        }



        let back_mask = (0x0101010101010101u64 << file) & ((1u64 << sq) - 1);
        let has_doubled_pawn = (black_pawns & back_mask) != 0;
        if has_doubled_pawn {
            o_eval += config.pawn_double_malus;
            e_eval += config.pawn_double_malus;
        }

        let is_passed = (1u64 << sq) & precalculated_passed_pawns != 0;
        if is_passed {
            let bonus = match moves_until_promote {
                6 => 10,
                5 => 40,
                4 => 90,
                3 => 180,
                2 => 320,
                1 => 500,
                _ => 0,
            };
            let is_protected = (file > 0 && sq + 7 < 64 && ((1u64 << (sq + 7)) & black_pawns) != 0) ||
                               (file < 7 && sq + 9 < 64 && ((1u64 << (sq + 9)) & black_pawns) != 0);
            if is_protected {
                o_eval -= config.protected_passed_pawn_middlegame;
                e_eval -= config.protected_passed_pawn_endgame;
            }



            e_eval -= bonus;
            o_eval -= bonus / 3;
        }

        let adjacent_files = ADJACENT_FILES_MASK[file as usize];
        let is_isolated = (black_pawns & adjacent_files) == 0;
        if is_isolated {
            o_eval += config.pawn_isolated_malus;
            e_eval += config.pawn_isolated_malus + 15;
        }

        let behind_and_same_rank_mask = !((1u64 << (rank * 8)) - 1);
        let is_backward = !is_isolated && (black_pawns & adjacent_files & behind_and_same_rank_mask) == 0;
        if is_backward {
            o_eval += config.pawn_backward_malus;
            e_eval += config.pawn_backward_malus + 10;
        }

        (o_eval, e_eval)
    }

    fn white_rook(&self, sq: u8, board: &Board, config: &Config, game_phase: i16, movegen: &MoveGenService, opp_king_ring: u64) -> (i16, u8, i16) {
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
                o_eval += config.rook_open_file;
                e_eval += config.rook_open_file + 10;
            } else {
                // Half-open file
                o_eval += config.rook_half_open_file;
                e_eval += config.rook_half_open_file + 5;
            }
        }

        let white_rooks = board.bitboards[crate::model::WHITE_ROOK];
        if (white_rooks & file_mask).count_ones() >= 2 {
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
                    o_eval += config.rook_behind_enemy_passed_pawn_mg;
                    e_eval += config.rook_behind_enemy_passed_pawn_eg;
                }
            }
            file_black_pawns &= file_black_pawns - 1;
        }

        // Rook on 7th Rank
        if rank == 6 {
            o_eval += config.rook_on_seventh;
            e_eval += config.rook_on_seventh + 15;
        }

        // Rook mobility
        let attacks = movegen.get_rook_attacks(sq as usize, board.occupied);
        
        // Threat Matrix: Rook attacks Queen
        let attacked_black_queens = (attacks & board.bitboards[crate::model::BLACK_QUEEN]).count_ones() as i16;
        o_eval += attacked_black_queens * config.threat_rook_attacks_queen;
        let mobility = attacks.count_ones() as i16;
        o_eval += mobility * config.rook_mobility_factor;
        e_eval += mobility * (config.rook_mobility_factor + 3);

        // Rook attacking/defending pawns in endgame
        let attacked_pawns = (attacks & board.bitboards[crate::model::BLACK_PAWN]).count_ones() as i16;
        let defended_pawns = (attacks & board.bitboards[crate::model::WHITE_PAWN]).count_ones() as i16;
        e_eval += attacked_pawns * 15;
        e_eval += defended_pawns * 8;

        // King ring attacks
        let attacks_on_ring = (attacks & opp_king_ring).count_ones() as i16;
        let attackers = if attacks_on_ring > 0 { 1 } else { 0 };
        let danger = attacks_on_ring * config.king_ring_attack_rook;

        let eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        (eval, attackers, danger)
    }

    fn black_rook(&self, sq: u8, board: &Board, config: &Config, game_phase: i16, movegen: &MoveGenService, opp_king_ring: u64) -> (i16, u8, i16) {
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
                o_eval -= config.rook_open_file;
                e_eval -= config.rook_open_file + 10;
            } else {
                // Half-open file
                o_eval -= config.rook_half_open_file;
                e_eval -= config.rook_half_open_file + 5;
            }
        }

        let black_rooks = board.bitboards[crate::model::BLACK_ROOK];
        if (black_rooks & file_mask).count_ones() >= 2 {
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
                    o_eval -= config.rook_behind_enemy_passed_pawn_mg;
                    e_eval -= config.rook_behind_enemy_passed_pawn_eg;
                }
            }
            file_white_pawns &= file_white_pawns - 1;
        }

        // Rook on 7th Rank
        if rank == 1 {
            o_eval -= config.rook_on_seventh;
            e_eval -= config.rook_on_seventh + 15;
        }

        // Rook mobility
        let attacks = movegen.get_rook_attacks(sq as usize, board.occupied);
        
        // Threat Matrix: Rook attacks Queen
        let attacked_white_queens = (attacks & board.bitboards[crate::model::WHITE_QUEEN]).count_ones() as i16;
        o_eval -= attacked_white_queens * config.threat_rook_attacks_queen;
        let mobility = attacks.count_ones() as i16;
        o_eval -= mobility * config.rook_mobility_factor;
        e_eval -= mobility * (config.rook_mobility_factor + 3);

        // Rook attacking/defending pawns in endgame
        let attacked_pawns = (attacks & board.bitboards[crate::model::WHITE_PAWN]).count_ones() as i16;
        let defended_pawns = (attacks & board.bitboards[crate::model::BLACK_PAWN]).count_ones() as i16;
        e_eval -= attacked_pawns * 15;
        e_eval -= defended_pawns * 8;

        // King ring attacks
        let attacks_on_ring = (attacks & opp_king_ring).count_ones() as i16;
        let attackers = if attacks_on_ring > 0 { 1 } else { 0 };
        let danger = attacks_on_ring * config.king_ring_attack_rook;

        let eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        (eval, attackers, danger)
    }

    fn white_knight(&self, sq: u8, board: &Board, config: &Config, game_phase: i16, movegen: &MoveGenService, opp_king_ring: u64, friendly_true_outposts: u64) -> (i16, u8, i16) {
        let mut o_eval = 0;
        let mut e_eval = 0;
        let sq = sq as i32;
        let rank = sq / 8;
        let file = sq % 8;
        
    
        if rank == 0 || rank == 7 || file == 0 || file == 7 {
            o_eval -= config.knight_on_rim_malus;
        }

        // Evaluate knight attacks on other pieces
        let attacks = movegen.get_knight_attacks(sq as usize);
        
        // Threat Matrix: Knight attacks Queen
        let attacked_black_queens = (attacks & board.bitboards[crate::model::BLACK_QUEEN]).count_ones() as i16;
        o_eval += attacked_black_queens * config.threat_minor_attacks_queen;
        for &(target_piece, bonus_simple, bonus_tempo) in &self.attack_bonus_white {
            let target_bb_idx = Board::piece_to_bb_idx(target_piece as u8);
            let count = (attacks & board.bitboards[target_bb_idx]).count_ones() as i16;
            o_eval += count * bonus_simple;
            if board.white_to_move {
                o_eval += count * bonus_tempo;
            }
        }

        // Knight mobility
        let mobility = attacks.count_ones() as i16;
        o_eval += mobility * config.knight_mobility_factor;
        e_eval += mobility * config.knight_mobility_factor;

        // King ring attacks
        let attacks_on_ring = (attacks & opp_king_ring).count_ones() as i16;
        o_eval += attacks_on_ring * config.king_ring_attack_knight;

        let stands_on_outpost = (1u64 << sq) & friendly_true_outposts != 0;
        let attacks_outpost = (attacks & friendly_true_outposts) != 0;
        if stands_on_outpost || attacks_outpost {
            o_eval += config.knight_outpost_true_mg;
            e_eval += config.knight_outpost_true_eg;
        }

        let is_centered = (3..=5).contains(&rank) && (2..=5).contains(&file);
        if is_centered {
            e_eval += config.knight_centered;
            o_eval += config.knight_centered / 2;
        }
    
        if sq == 1 || sq == 6 {
            o_eval -= config.undeveloped_knight_malus;
        }

        if sq + 8 < 64 && ((1u64 << (sq + 8)) & board.bitboards[crate::model::BLACK_PAWN]) != 0 {
            e_eval += config.knight_blockes_pawn;
            o_eval += config.knight_blockes_pawn / 2;
        }
            // King ring attacks
        let attacks_on_ring = (attacks & opp_king_ring).count_ones() as i16;
        let attackers = if attacks_on_ring > 0 { 1 } else { 0 };
        let danger = attacks_on_ring * config.king_ring_attack_knight;

        let eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        (eval, attackers, danger)
    }

    fn black_knight(&self, sq: u8, board: &Board, config: &Config, game_phase: i16, movegen: &MoveGenService, opp_king_ring: u64, friendly_true_outposts: u64) -> (i16, u8, i16) {
        let mut o_eval = 0;
        let mut e_eval = 0;
        let sq = sq as i32;
        let rank = sq / 8;
        let file = sq % 8;
        
    
        if rank == 0 || rank == 7 || file == 0 || file == 7 {
            o_eval += config.knight_on_rim_malus;
        }
    
        // Evaluate knight attacks on other pieces
        let attacks = movegen.get_knight_attacks(sq as usize);
        
        // Threat Matrix: Knight attacks Queen
        let attacked_white_queens = (attacks & board.bitboards[crate::model::WHITE_QUEEN]).count_ones() as i16;
        o_eval -= attacked_white_queens * config.threat_minor_attacks_queen;
        for &(target_piece, bonus_simple, bonus_tempo) in &self.attack_bonus_black {
            let target_bb_idx = Board::piece_to_bb_idx(target_piece as u8);
            let count = (attacks & board.bitboards[target_bb_idx]).count_ones() as i16;
            o_eval -= count * bonus_simple;
            if !board.white_to_move {
                o_eval -= count * bonus_tempo;
            }
        }

        // Knight mobility
        let mobility = attacks.count_ones() as i16;
        o_eval -= mobility * config.knight_mobility_factor;
        e_eval -= mobility * config.knight_mobility_factor;

        let stands_on_outpost = (1u64 << sq) & friendly_true_outposts != 0;
        let attacks_outpost = (attacks & friendly_true_outposts) != 0;
        if stands_on_outpost || attacks_outpost {
            o_eval -= config.knight_outpost_true_mg;
            e_eval -= config.knight_outpost_true_eg;
        }

        let is_centered = (2..=4).contains(&rank) && (2..=5).contains(&file);
        if is_centered {
            e_eval -= config.knight_centered;
            o_eval -= config.knight_centered / 2;
        }
    
        if sq == 57 || sq == 62 {
            o_eval += config.undeveloped_knight_malus;
        }

        if sq >= 8 && ((1u64 << (sq - 8)) & board.bitboards[crate::model::WHITE_PAWN]) != 0 {
            e_eval -= config.knight_blockes_pawn;
            o_eval -= config.knight_blockes_pawn / 2;
        }
            // King ring attacks
        let attacks_on_ring = (attacks & opp_king_ring).count_ones() as i16;
        let attackers = if attacks_on_ring > 0 { 1 } else { 0 };
        let danger = attacks_on_ring * config.king_ring_attack_knight;

        let eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        (eval, attackers, danger)
    }

    fn white_bishop(&self, sq: u8, board: &Board, config: &Config, game_phase: i16, movegen: &MoveGenService, opp_king_ring: u64, friendly_true_outposts: u64) -> (i16, u8, i16) {
        let mut o_eval = 0;
        let mut e_eval = 0;
        let sq = sq as i32;
        let file = sq % 8;
        let rank = sq / 8;
        

        if sq == 2 || sq == 5 {
            o_eval -= config.undeveloped_bishop_malus;
        }

        if rank == 6 && file == 7 && ((1u64 << (sq - 9)) & board.occupied) != 0 {
            o_eval -= config.bishop_trapped_at_rim_malus;
        }
        if rank == 6 && file == 0 && ((1u64 << (sq - 7)) & board.occupied) != 0 {
            o_eval -= config.bishop_trapped_at_rim_malus;
        }

        // Bishop mobility
        let attacks = movegen.get_bishop_attacks(sq as usize, board.occupied);
        
        // Threat Matrix: Bishop attacks Rook/Queen
        let attacked_black_rooks = (attacks & board.bitboards[crate::model::BLACK_ROOK]).count_ones() as i16;
        let attacked_black_queens = (attacks & board.bitboards[crate::model::BLACK_QUEEN]).count_ones() as i16;
        o_eval += attacked_black_rooks * config.threat_minor_attacks_rook;
        o_eval += attacked_black_queens * config.threat_minor_attacks_queen;
        let mobility = attacks.count_ones() as i16;
        o_eval += mobility * config.bishop_mobility_factor;
        e_eval += mobility * config.bishop_mobility_factor;

        let stands_on_outpost = (1u64 << sq) & friendly_true_outposts != 0;
        let attacks_outpost = (attacks & friendly_true_outposts) != 0;
        if stands_on_outpost || attacks_outpost {
            o_eval += config.bishop_outpost_true_mg;
            e_eval += config.bishop_outpost_true_eg;
        }

        // King ring attacks
        let attacks_on_ring = (attacks & opp_king_ring).count_ones() as i16;
        let attackers = if attacks_on_ring > 0 { 1 } else { 0 };
        let danger = attacks_on_ring * config.king_ring_attack_bishop;

        let eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        (eval, attackers, danger)
    }

    fn black_bishop(&self, sq: u8, board: &Board, config: &Config, game_phase: i16, movegen: &MoveGenService, opp_king_ring: u64, friendly_true_outposts: u64) -> (i16, u8, i16) {
        let mut o_eval = 0;
        let mut e_eval = 0;
        let sq = sq as i32;
        let file = sq % 8;
        let rank = sq / 8;
        

        if sq == 58 || sq == 61 {
            o_eval += config.undeveloped_bishop_malus;
        }

        if rank == 1 && file == 7 && ((1u64 << (sq + 7)) & board.occupied) != 0 {
            o_eval += config.bishop_trapped_at_rim_malus;
        }
        if rank == 1 && file == 0 && ((1u64 << (sq + 9)) & board.occupied) != 0 {
            o_eval += config.bishop_trapped_at_rim_malus;
        }

        // Bishop mobility
        let attacks = movegen.get_bishop_attacks(sq as usize, board.occupied);
        
        // Threat Matrix: Bishop attacks Rook/Queen
        let attacked_white_rooks = (attacks & board.bitboards[crate::model::WHITE_ROOK]).count_ones() as i16;
        let attacked_white_queens = (attacks & board.bitboards[crate::model::WHITE_QUEEN]).count_ones() as i16;
        o_eval -= attacked_white_rooks * config.threat_minor_attacks_rook;
        o_eval -= attacked_white_queens * config.threat_minor_attacks_queen;
        let mobility = attacks.count_ones() as i16;
        o_eval -= mobility * config.bishop_mobility_factor;
        e_eval -= mobility * config.bishop_mobility_factor;

        let stands_on_outpost = (1u64 << sq) & friendly_true_outposts != 0;
        let attacks_outpost = (attacks & friendly_true_outposts) != 0;
        if stands_on_outpost || attacks_outpost {
            o_eval -= config.bishop_outpost_true_mg;
            e_eval -= config.bishop_outpost_true_eg;
        }

        // King ring attacks
        let attacks_on_ring = (attacks & opp_king_ring).count_ones() as i16;
        let attackers = if attacks_on_ring > 0 { 1 } else { 0 };
        let danger = attacks_on_ring * config.king_ring_attack_bishop;

        let eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        (eval, attackers, danger)
    }

    fn white_queen(&self, sq: u8, board: &Board, config: &Config, game_phase: i16, movegen: &MoveGenService, opp_king_ring: u64) -> (i16, u8, i16) {
        let mut o_eval = 0;

        let attackers_mask = movegen.get_attackers_mask_for_see(board, true, sq, board.occupied);
        let num_attackers = attackers_mask.count_ones() as i16;
        if num_attackers > 0 {
            o_eval -= (config.queen_in_attack * num_attackers) + if !board.white_to_move { config.queen_in_attack_with_tempo } else { 0 };
        }

        // King ring attacks for queen
        let attacks = movegen.get_rook_attacks(sq as usize, board.occupied) | movegen.get_bishop_attacks(sq as usize, board.occupied);        // King ring attacks
        let attacks_on_ring = (attacks & opp_king_ring).count_ones() as i16;
        let attackers = if attacks_on_ring > 0 { 1 } else { 0 };
        let danger = attacks_on_ring * config.king_ring_attack_queen;

        let eval = self.calculate_weighted_eval(o_eval, 0, game_phase);
        (eval, attackers, danger)
    }

    fn black_queen(&self, sq: u8, board: &Board, config: &Config, game_phase: i16, movegen: &MoveGenService, opp_king_ring: u64) -> (i16, u8, i16) {
        let mut o_eval = 0;

        let attackers_mask = movegen.get_attackers_mask_for_see(board, false, sq, board.occupied);
        let num_attackers = attackers_mask.count_ones() as i16;
        if num_attackers > 0 {
            o_eval += (config.queen_in_attack * num_attackers) + if board.white_to_move { config.queen_in_attack_with_tempo } else { 0 };
        }

        // King ring attacks for queen
        let attacks = movegen.get_rook_attacks(sq as usize, board.occupied) | movegen.get_bishop_attacks(sq as usize, board.occupied);        // King ring attacks
        let attacks_on_ring = (attacks & opp_king_ring).count_ones() as i16;
        let attackers = if attacks_on_ring > 0 { 1 } else { 0 };
        let danger = attacks_on_ring * config.king_ring_attack_queen;

        let eval = self.calculate_weighted_eval(o_eval, 0, game_phase);
        (eval, attackers, danger)
    }
 
    fn white_king(&self, sq: u8, board: &Board, config: &Config, game_phase: i16, movegen: &MoveGenService) -> (i16, u8, i16) {
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
            o_eval -= config.undeveloped_king_malus
        }

        let in_check = movegen.get_attackers_mask(board, true, sq as u8, board.occupied).count_ones() as usize;
        if in_check == 1 {
            o_eval -= config.king_in_check_malus;
            e_eval -= config.king_in_check_malus;
        } else if in_check > 1 {
            o_eval -= config.king_in_double_check_malus;
            e_eval -= config.king_in_double_check_malus;
        }

        // Graduated Center Proximity in Endgame
        if game_phase < 60 {
            let rank_dist = if rank < 3 { 3 - rank } else if rank > 4 { rank - 4 } else { 0 };
            let file_dist = if file < 3 { 3 - file } else if file > 4 { file - 4 } else { 0 };
            let dist = rank_dist + file_dist;
            e_eval -= (dist * 40) as i16;
        }

        let white_pawns = board.bitboards[crate::model::WHITE_PAWN];
        let black_pawns = board.bitboards[crate::model::BLACK_PAWN];
        let file_mask = 0x0101010101010101u64 << file;
        if (file_mask & white_pawns) == 0 {
            if (file_mask & black_pawns) == 0 {
                o_eval -= config.king_open_file_malus;
            } else {
                o_eval -= config.king_half_open_file_malus;
            }
        }
        if file > 0 && sq + 7 < 64 {
            let bit = 1u64 << (sq + 7);
            if (bit & white_pawns) != 0 { o_eval += pawn_shield_val; }
            else if sq + 15 < 64 && ((1u64 << (sq + 15)) & white_pawns) != 0 { o_eval += pawn_shield_val / 2; }
            else if (bit & board.white_pieces) != 0 { o_eval += piece_shield_val; }
        }
        if sq + 8 < 64 {
            let bit = 1u64 << (sq + 8);
            if (bit & white_pawns) != 0 { o_eval += pawn_shield_val; }
            else if sq + 16 < 64 && ((1u64 << (sq + 16)) & white_pawns) != 0 { o_eval += pawn_shield_val / 2; }
            else if (bit & board.white_pieces) != 0 { o_eval += piece_shield_val; }
        }
        if file < 7 && sq + 9 < 64 {
            let bit = 1u64 << (sq + 9);
            if (bit & white_pawns) != 0 { o_eval += pawn_shield_val; }
            else if sq + 17 < 64 && ((1u64 << (sq + 17)) & white_pawns) != 0 { o_eval += pawn_shield_val / 2; }
            else if (bit & board.white_pieces) != 0 { o_eval += piece_shield_val; }
        }

        if rank == 0 {
            e_eval -= config.king_trapp_at_baseline_malus;
        }

        let eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        (eval, 0, 0)
    }

    fn black_king(&self, sq: u8, board: &Board, config: &Config, game_phase: i16, movegen: &MoveGenService) -> (i16, u8, i16) {
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
            o_eval += config.undeveloped_king_malus
        }

        let in_check = movegen.get_attackers_mask(board, false, sq as u8, board.occupied).count_ones() as usize;
        if in_check == 1 {
            o_eval += config.king_in_check_malus;
            e_eval += config.king_in_check_malus;
        } else if in_check > 1 {
            o_eval += config.king_in_double_check_malus;
            e_eval += config.king_in_double_check_malus;
        }

        // Graduated Center Proximity in Endgame
        if game_phase < 60 {
            let rank_dist = if rank < 3 { 3 - rank } else if rank > 4 { rank - 4 } else { 0 };
            let file_dist = if file < 3 { 3 - file } else if file > 4 { file - 4 } else { 0 };
            let dist = rank_dist + file_dist;
            e_eval += (dist * 40) as i16;
        }

        let black_pawns = board.bitboards[crate::model::BLACK_PAWN];
        let white_pawns = board.bitboards[crate::model::WHITE_PAWN];
        let file_mask = 0x0101010101010101u64 << file;
        if (file_mask & black_pawns) == 0 {
            if (file_mask & white_pawns) == 0 {
                o_eval += config.king_open_file_malus;
            } else {
                o_eval += config.king_half_open_file_malus;
            }
        }
        if file > 0 && sq >= 9 {
            let bit = 1u64 << (sq - 9);
            if (bit & black_pawns) != 0 { o_eval -= pawn_shield_val; }
            else if sq >= 17 && ((1u64 << (sq - 17)) & black_pawns) != 0 { o_eval -= pawn_shield_val / 2; }
            else if (bit & board.black_pieces) != 0 { o_eval -= piece_shield_val; }
        }
        if sq >= 8 {
            let bit = 1u64 << (sq - 8);
            if (bit & black_pawns) != 0 { o_eval -= pawn_shield_val; }
            else if sq >= 16 && ((1u64 << (sq - 16)) & black_pawns) != 0 { o_eval -= pawn_shield_val / 2; }
            else if (bit & board.black_pieces) != 0 { o_eval -= piece_shield_val; }
        }
        if file < 7 && sq >= 7 {
            let bit = 1u64 << (sq - 7);
            if (bit & black_pawns) != 0 { o_eval -= pawn_shield_val; }
            else if sq >= 15 && ((1u64 << (sq - 15)) & black_pawns) != 0 { o_eval -= pawn_shield_val / 2; }
            else if (bit & board.black_pieces) != 0 { o_eval -= piece_shield_val; }
        }

        if rank == 7 {
            e_eval += config.king_trapp_at_baseline_malus;
        }

        let eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        (eval, 0, 0)
    }

    fn calculate_weighted_eval(&self, o_eval: i16, e_eval: i16, game_phase: i16) -> i16 {
        let o_eval = o_eval as i32;
        let e_eval = e_eval as i32;
        let game_phase = game_phase as i32;
        let res = ((o_eval * game_phase) + (e_eval * (256 - game_phase))) / 256;
        debug_assert!(res < 32_767 && res > -32_767);
        res as i16
    }

    /// return Value of 255 means early game and values towards 0 means endgamephase
    /// a middle value like 128 respects early and late game evaluation in the same weight
    /// All with 6 or less pieces is considered pure endgame
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
            let white_king_sq = board.bitboards[crate::model::WHITE_KING].trailing_zeros() as i32;
            let black_king_sq = board.bitboards[crate::model::BLACK_KING].trailing_zeros() as i32;
            let wk_rank = white_king_sq / 8;
            let wk_file = white_king_sq % 8;
            let bk_rank = black_king_sq / 8;
            let bk_file = black_king_sq % 8;
            
            let dist_to_own_king = (rank - wk_rank).abs().max((file - wk_file).abs());
            let dist_to_opp_king = (rank - bk_rank).abs().max((file - bk_file).abs());
            
            let own_k_bonus = ((6 - dist_to_own_king).max(0) * 15) as i16;
            e_eval += own_k_bonus;
            
            let opp_k_malus = ((6 - dist_to_opp_king).max(0) * 12) as i16;
            e_eval -= opp_k_malus;
        }

        (o_eval, e_eval)
    }

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
            let white_king_sq = board.bitboards[crate::model::WHITE_KING].trailing_zeros() as i32;
            let black_king_sq = board.bitboards[crate::model::BLACK_KING].trailing_zeros() as i32;
            let wk_rank = white_king_sq / 8;
            let wk_file = white_king_sq % 8;
            let bk_rank = black_king_sq / 8;
            let bk_file = black_king_sq % 8;
            
            let dist_to_own_king = (rank - bk_rank).abs().max((file - bk_file).abs());
            let dist_to_opp_king = (rank - wk_rank).abs().max((file - wk_file).abs());
            
            let own_k_bonus = ((6 - dist_to_own_king).max(0) * 15) as i16;
            e_eval -= own_k_bonus;
            
            let opp_k_malus = ((6 - dist_to_opp_king).max(0) * 12) as i16;
            e_eval += opp_k_malus;
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
        equal_eval("3qk1r1/ppppp1pp/3bbp1n/8/r7/R2BBP1N/PPPPP1PP/3QK1R1 w Kk - 0 1");
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
        // Figure values test for white
        eval_between("rnbqkbnr/pppp1ppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 80, 190);
        eval_between("rnbqkb1r/pppp1ppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 300, 450);
        eval_between("rn1qkb1r/pppp1ppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 600, 750);
        eval_between("r2qkb1r/pppp1ppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 850, 1100);
        eval_between("3qkb2/pppp1ppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQ - 0 1", 1800, 2100);
        eval_between("4k3/pppp1ppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQha - 0 1", 3000, 3800);

        // Figure values test for black
        eval_between("rnbqkbnr/pppppppp/8/8/8/8/PPPPP1PP/RNBQKBNR b KQkq - 0 1", -250, 50);
        eval_between("rnbqkbnr/pppppppp/8/8/8/8/PPPPP1PP/RNBQKB1R b KQkq - 0 1", -450, -350);
        eval_between("rnbqkbnr/pppppppp/8/8/8/8/PPPP1PPP/RN1QKB1R b KQkq - 0 1", -800, -600);
        eval_between("rnbqkbnr/pppppppp/8/8/8/8/PPPP1PPP/R2QKB1R b KQkq - 0 1", -1200, -850);
        eval_between("rnbqkbnr/pppppppp/8/8/8/8/PPPP1PPP/3QKB2 b - - 0 1", -2300, -1800);
        eval_between("rnbqkbnr/pppppppp/8/8/8/8/PPPP1PPP/4K3 b kq - 0 1", -3800, -2900);
    }

    #[test]
    fn position_unequel_test() {
        // position unequel
        eval_between("rnbqkb1r/pppppppp/8/5n2/8/5N2/PPPPPPPP/RNBQKB1R w KQkq - 0 1", -50, -10);
        eval_between("rnbqkb1r/pppppppp/5n2/8/5N2/8/PPPPPPPP/RNBQKB1R w KQkq - 0 1", 10, 50);
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
        let eval1 = eval_service.calc_eval(&board, config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX);

        let board = fen_service.set_fen("rnb1k1n1/pp4p1/2p3Nr/3B3p/q7/1RP3P1/3NPP1P/3QK2R b Kq - 0 19");
        let eval2 = eval_service.calc_eval(&board, config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX);

        let board = fen_service.set_fen("rnb1k1n1/pp4p1/6Nr/3p3p/q7/1RP3P1/3NPPBP/3QK2R w Kq - 3 19");
        let eval3 = eval_service.calc_eval(&board, config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX);

        let board = fen_service.set_fen("rnb1k3/pp2n1p1/7r/3p3p/q4N2/1RP3P1/3NPP1P/3QK2R w Kq - 2 21");
        let eval4 = eval_service.calc_eval(&board, config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX);

        println!("{}", eval1);
        println!("{}", eval2);
        println!("{}", eval3);
        println!("{}", eval4);
    }

    #[test]
    fn unequal_position_test() {
        eval_between("8/8/8/8/2k5/6K1/8/8 w - - 0 1", -120, -60);
    }

    #[test]
    fn knight_position_test() {
        fib("rnbqkbnr/pppppppp/8/8/8/5N2/PPPPPPPP/RNBQKB1R b KQkq - 1 1", "rnbqkbnr/pppppppp/8/8/8/7N/PPPPPPPP/RNBQKB1R b KQkq - 1 1");
        fib("rnbqkbnr/pppppppp/8/8/8/5N2/PPPPPPPP/RNBQKB1R w KQkq - 0 1", "r1bqkbnr/pppppppp/n7/8/8/5N2/PPPPPPPP/RNBQKB1R w KQkq - 0 1");
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
        let eval1 = eval.calc_eval(&board1, &config, &movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX);
        let eval2 = eval.calc_eval(&board2, &config, &movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX);

        println!("FIB: eval1={} eval2={} diff={} | fen1='{}' fen2='{}'", eval1, eval2, eval1 - eval2, fen1, fen2);

        if eval1 < eval2 - 10 {
            println!("-->> eval1 is unexpectedly much less than eval2: {}", eval1 - eval2);
            assert!(false);
        }
    }

    fn equal_eval(fen: &str) {
        let fen_service = Service::new().fen;
        let mut eval_service = Service::new().eval;
        eval_service._set_custom_config(&Config::_for_evel_equal_tests());
        let movegen = Service::new().move_gen;

        let config = &Config::_for_evel_equal_tests();
        let board = &fen_service.set_fen(fen);
        let eval = eval_service.calc_eval(board, config, &movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX);
        assert!(eval.abs() <= 10, "Eval {} is not close to 0", eval);
    }

    fn eval_between(fen: &str, lower: i16, higher: i16) {
        print!("Test: {} | ", fen);
        let fen_service = Service::new().fen;
        let eval_service = Service::new().eval;
        let movegen = Service::new().move_gen;

        let config = &Config::_for_evel_equal_tests();
        let board = &fen_service.set_fen(fen);
        let eval = eval_service.calc_eval(board, config, &movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX);
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
        eval_service.calc_eval(board, &config, &movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX);
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
        config_normal.aggressiveness = crate::config::Aggressiveness::Normal;
        config_normal.your_turn_bonus = 1000; // Enormous positional bonus to force capping
        let eval_normal = eval_service.calc_eval(&board, &config_normal, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX);
        // Soft cap calculation: 150 + (1000 - 150) / 5 = 150 + 170 = 320
        assert_eq!(eval_normal, 320, "Normal aggressiveness eval should be soft capped at 320");

        // 2. Aggressive Test (Cap = 250)
        let mut config_aggressive = Config::new();
        config_aggressive.aggressiveness = crate::config::Aggressiveness::Aggressive;
        config_aggressive.your_turn_bonus = 1000;
        let eval_aggressive = eval_service.calc_eval(&board, &config_aggressive, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX);
        // Soft cap calculation: 250 + (1000 - 250) / 5 = 250 + 150 = 400
        assert_eq!(eval_aggressive, 400, "Aggressive eval should be soft capped at 400");

        // 3. HighAggressive Test (Cap = 400)
        let mut config_high = Config::new();
        config_high.aggressiveness = crate::config::Aggressiveness::HighAggressive;
        config_high.your_turn_bonus = 1000;
        let eval_high = eval_service.calc_eval(&board, &config_high, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX);
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
            let eval_with = eval_service.calc_eval(&board, &config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX);

            config.connected_passed_pawn_mg = 0;
            config.connected_passed_pawn_eg = 0;
            let eval_without = eval_service.calc_eval(&board, &config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX);
            
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
            let eval_with = eval_service.calc_eval(&board, &config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX);

            config.knight_outpost_true_mg = 0;
            config.knight_outpost_true_eg = 0;
            let eval_without = eval_service.calc_eval(&board, &config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX);

            // Phase = 1 (1 Knight = 1/24 * 255 = 10) -> mostly endgame (eg weight is 246/256)
            // Expected bonus: weighted outpost bonus ~ 30
            let diff = eval_with - eval_without;
            assert!(diff >= 25 && diff <= 35, "True outpost occupancy bonus not applied correctly, diff={}", diff);

            // Knight attacking true outpost (Knight on c4, outpost at e5)
            let board_att = fen_service.set_fen("8/8/8/8/2NP4/k7/8/K7 w - - 0 1");
            config.knight_outpost_true_mg = 60;
            config.knight_outpost_true_eg = 30;
            let eval_with_att = eval_service.calc_eval(&board_att, &config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX);

            config.knight_outpost_true_mg = 0;
            config.knight_outpost_true_eg = 0;
            let eval_without_att = eval_service.calc_eval(&board_att, &config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX);

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
            let eval_ks = eval_service.calc_eval(&board_ks_mg, &config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX);
            let eval_qs = eval_service.calc_eval(&board_qs_mg, &config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX);

            config.king_pawn_shield_kingside = 0;
            config.king_pawn_shield_queenside = 0;

            // Evaluate without shields
            let eval_ks_no = eval_service.calc_eval(&board_ks_mg, &config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX);
            let eval_qs_no = eval_service.calc_eval(&board_qs_mg, &config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX);

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
            let eval_unscaled = eval_service.calc_eval(&board, &config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX);

            config.opposite_bishops_draw_scale = 50;
            let eval_scaled = eval_service.calc_eval(&board, &config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX);

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
            let eval_with = eval_service.calc_eval(&board, &config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX);

            config.rook_behind_enemy_passed_pawn_mg = 0;
            config.rook_behind_enemy_passed_pawn_eg = 0;
            let eval_without = eval_service.calc_eval(&board, &config, movegen, &crate::pawn_hash::PawnHashTable::new(16), i16::MIN, i16::MAX);

            // phase is 2 rooks = 4/24 * 255 = 42 -> mostly endgame
            let diff = eval_with - eval_without;
            assert!(diff >= 75 && diff <= 100, "Rook behind enemy passed pawn bonus not applied correctly, diff={}", diff);
        }
    }
}