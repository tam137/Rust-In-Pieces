use crate::config::Config;
use crate::model::{Board, DataMap, DataMapKey, RIP_MISSED_DM_KEY};
use crate::move_gen_service::MoveGenService;


pub struct EvalService {
    knight_moves: [i16; 8],
    attack_bonus_white: [(i16, i16, i16); 2],
    attack_bonus_black: [(i16, i16, i16); 2],
}

impl EvalService {

    pub fn new(config: &Config) -> Self {
        Self {
            knight_moves: [-21, -19, -12, -8, 21, 19, 12, 8],
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

    pub fn calc_eval(&self, board: &Board, config: &Config, movegen: &MoveGenService, local_map: &DataMap) -> i16 {
        let mut eval: i16 = 0;
        let game_phase = self.get_game_phase(board) as i16;

        let field = &board.field;
        for idx in 21..99 {
            let piece = field[idx];
            let eval_for_piece: i16 = match piece {
                10 => self.white_pawn(idx, board, config, field, game_phase),
                11 => self.white_rook(idx, board, config, field, game_phase),
                12 => self.white_knight(idx, board, config, field, game_phase),
                13 => self.white_bishop(idx, board, config, field, game_phase, movegen),
                14 => self.white_queen(idx, board, config, field, game_phase, movegen),
                15 => self.white_king(idx, board, config, field, game_phase, movegen),
                20 => self.black_pawn(idx, board, config, field, game_phase),
                21 => self.black_rook(idx, board, config, field, game_phase),
                22 => self.black_knight(idx, board, config, field, game_phase),
                23 => self.black_bishop(idx, board, config, field, game_phase, movegen),
                24 => self.black_queen(idx, board, config, field, game_phase, movegen),
                25 => self.black_king(idx, board, config, field, game_phase, movegen),
                _ => 0,
            };
            if config.print_eval_per_figure && piece > 0 {
                println!("{},\t{},\t{}", idx, piece, eval_for_piece);
            }
            eval = eval + eval_for_piece;
        }

        // TODO Tests
        let gives_chess_eval = if *local_map.get_data(DataMapKey::WhiteGivesCheck).expect(RIP_MISSED_DM_KEY) {
            config.gives_check_bonus
        } else if *local_map.get_data(DataMapKey::BlackGivesCheck).expect(RIP_MISSED_DM_KEY) {
            -config.gives_check_bonus
        } else { 0 };

        eval += self.calculate_weighted_eval(gives_chess_eval, 0, game_phase);


        eval = eval + if board.white_to_move { config.your_turn_bonus } else { -config.your_turn_bonus };
        eval = self.adjust_eval(eval, game_phase, config);

        if config.print_eval_per_figure {
            println!("{}", eval);
        }
        eval
    }

    fn white_pawn(&self, idx: usize, board: &Board, config: &Config, f: &[i32; 120], game_phase: i16) -> i16 {
        let mut o_eval = 0;
        let mut e_eval = 0;
        let moves_until_promote = idx / 10 - 2;
        let on_rank = 8 - moves_until_promote;

        if (on_rank >= 3) && (on_rank <= 5) {
            if f[idx-11] == 12 || f[idx-9] == 12 {
                o_eval = o_eval + config.pawn_supports_knight_outpost;
            }
        }

        if idx==54 || idx==55 || idx==64 || idx==65 {
            o_eval = o_eval + config.pawn_centered;
        }

        match moves_until_promote {
            1 => e_eval = e_eval + config.pawn_on_last_rank_bonus,
            2 => e_eval = e_eval + config.pawn_on_before_last_rank_bonus,
            3 => e_eval = e_eval + config.pawn_on_before_before_last_rank_bonus,
            _ => ()
        }

        match moves_until_promote {
            1 => o_eval = o_eval + config.pawn_on_last_rank_bonus / 2,
            2 => o_eval = o_eval + config.pawn_on_before_last_rank_bonus / 2,
            3 => o_eval = o_eval + config.pawn_on_before_before_last_rank_bonus / 2,
            _ => ()
        }

        if f[idx-9] == 10 || f[idx-11] == 10 {
            o_eval = o_eval + config.pawn_structure;
        }

        if f[idx-9] == 13 || f[idx-11] == 13 {
            e_eval = e_eval + config.pawn_defends_bishop;
        }

        if moves_until_promote >= 5 {
            o_eval = o_eval - config.pawn_undeveloped_malus;
        }

        if (f[idx-9] >= 21 && f[idx-9] <= 25) || (f[idx-11] >= 21 && f[idx-11] <= 25) {
            o_eval += config.pawn_attacks_opponent_fig + if board.white_to_move {
                config.pawn_attacks_opponent_fig_with_tempo
            } else {
                0 
            };
            e_eval += config.pawn_attacks_opponent_fig / 2;
        }

        if f[idx-10] == 10 || f[idx-20] == 10 || f[idx-30] == 10 {
            o_eval -= config.pawn_double_malus;
            e_eval -= config.pawn_double_malus / 2;
        }

        let eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        eval + config.piece_eval_pawn
    }

    fn black_pawn(&self, idx: usize, board: &Board, config: &Config, f: &[i32; 120], game_phase: i16) -> i16 {
        let mut o_eval = 0;
        let mut e_eval = 0;
        let moves_until_promote = 9 - (idx / 10);
        let on_rank = 8 - moves_until_promote;

        if (on_rank >= 3) && (on_rank <= 5) {
            if f[idx+11] == 22 || f[idx+9] == 22 {
                o_eval = o_eval - config.pawn_supports_knight_outpost;
            }
        }

        if idx==54 || idx==55 || idx==64 || idx==65 {
            o_eval = o_eval - config.pawn_centered;
        }

        match moves_until_promote {
            1 => e_eval = e_eval - config.pawn_on_last_rank_bonus,
            2 => e_eval = e_eval - config.pawn_on_before_last_rank_bonus,
            3 => e_eval = e_eval - config.pawn_on_before_before_last_rank_bonus,
            _ => ()
        }

        match moves_until_promote {
            1 => o_eval = o_eval - config.pawn_on_last_rank_bonus / 2,
            2 => o_eval = o_eval - config.pawn_on_before_last_rank_bonus / 2,
            3 => o_eval = o_eval - config.pawn_on_before_before_last_rank_bonus / 2,
            _ => ()
        }

        if f[idx+9] == 20 || f[idx+11] == 20 {
            o_eval = o_eval - config.pawn_structure;
        }

        if f[idx+9] == 23 || f[idx+11] == 23 {
            e_eval = e_eval - config.pawn_defends_bishop;
        }

        if moves_until_promote >= 5 {
            o_eval = o_eval + config.pawn_undeveloped_malus;
        }

        if (f[idx+9] >= 11 && f[idx+9] <= 15) || (f[idx+11] >= 11 && f[idx+11] <= 15) {
            o_eval -= config.pawn_attacks_opponent_fig + if !board.white_to_move {
                config.pawn_attacks_opponent_fig_with_tempo
            } else {
                0
            };
            e_eval -= config.pawn_attacks_opponent_fig / 2;
        }

        if f[idx+10] == 20 || f[idx+20] == 20 || f[idx+30] == 20 {
            o_eval += config.pawn_double_malus;
            e_eval += config.pawn_double_malus / 2;
        }

        let eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        eval - config.piece_eval_pawn
    }


    fn white_rook(&self, _idx: usize, _board: &Board, config: &Config, _f: &[i32; 120], _game_phase: i16) -> i16 {
        let eval = config.piece_eval_rook;
        eval
    }

    fn black_rook(&self, _idx: usize, _board: &Board, config: &Config, _f: &[i32; 120], _game_phase: i16) -> i16 {
        let eval = -config.piece_eval_rook;
        eval
    }


    fn white_knight(&self, idx: usize, board: &Board, config: &Config, f: &[i32; 120], game_phase: i16) -> i16 {
        let mut o_eval = 0;
        let mut e_eval = 0;
        let on_rank = 8 - (idx / 10 - 2);
        let on_file = idx % 10;
    
        if on_rank == 1 || on_rank == 8 || on_file == 1 || on_file == 8 {
            o_eval -= config.knight_on_rim_malus;
        }

        // Evaluate knight attacks on other pieces
        for &offset in &self.knight_moves {
            let target_idx = idx as i32 + offset as i32;
            assert!(target_idx >= 0 && (target_idx as usize) < f.len());
            if let Some(&piece) = f.get(target_idx as usize) {
                for &(target_piece, bonus_simple, bonus_tempo) in &self.attack_bonus_white {
                    if piece == target_piece.into() {
                        o_eval += bonus_simple;
                        if board.white_to_move {
                            o_eval += bonus_tempo;
                        }
                        break;
                    }
                }
            }
        }

        if idx==43||idx==44||idx==45||idx==46||
            idx==53||idx==54||idx==55||idx==56||
            idx==63||idx==64||idx==65||idx==66 {
            e_eval += config.knight_centered;
            o_eval += config.knight_centered / 2;
        }
    
        if idx == 92 || idx == 97 {
            o_eval -= config.undeveloped_knight_malus;
        }

        if f[idx-10] == 20 {
            e_eval += config.knight_blockes_pawn;
            o_eval += config.knight_blockes_pawn / 2;
        }
    
        let eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        eval + config.piece_eval_knight
    }
    

    fn black_knight(&self, idx: usize, board: &Board, config: &Config, f: &[i32; 120], game_phase: i16) -> i16 {
        let mut o_eval = 0;
        let mut e_eval = 0;
        let on_rank = 8 - (idx / 10 - 2);
        let on_file = idx % 10;
    
        if on_rank == 1 || on_rank == 8 || on_file == 1 || on_file == 8 {
            o_eval += config.knight_on_rim_malus;
        }
    
        // Evaluate knight attacks on other pieces
        for &offset in &self.knight_moves {
            let target_idx = idx as i32 + offset as i32;
            assert!(target_idx >= 0 && (target_idx as usize) < f.len());
            if let Some(&piece) = f.get(target_idx as usize) {
                for &(target_piece, bonus_simple, bonus_tempo) in &self.attack_bonus_black {
                    if piece == target_piece.into() {
                        o_eval -= bonus_simple;
                        if !board.white_to_move {
                            o_eval -= bonus_tempo;
                        }
                        break;
                    }
                }
            }
        }
        

        if  idx==53||idx==54||idx==55||idx==56||
            idx==63||idx==64||idx==65||idx==66||
            idx==73||idx==74||idx==75||idx==76 {
            e_eval -= config.knight_centered;
            o_eval -= config.knight_centered / 2;
        }
    
        if idx == 22 || idx == 27 {
            o_eval += config.undeveloped_knight_malus;
        }

        if f[idx+10] == 10 {
            e_eval -= config.knight_blockes_pawn;
            o_eval -= config.knight_blockes_pawn / 2;
        }
    
        let eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        eval - config.piece_eval_knight
    }   


    fn white_bishop(&self, idx: usize, _board: &Board, config: &Config, f: &[i32; 120], game_phase: i16, _movegen: &MoveGenService) -> i16 {
        let mut o_eval = 0;
        let e_eval = 0;

        if idx == 93 || idx == 96 {
            o_eval = o_eval - config.undeveloped_bishop_malus;
        }

        //let moves = movegen.generate_moves_list_for_piece(board, idx as i32);
        //o_eval += moves.len() as i16 / 2 * config.move_freedom_bonus as i16;

        if idx % 10 == 8 && f[idx+9] != 0 { // TODO make this static to avoid modul
            o_eval = o_eval - config.bishop_trapped_at_rim_malus;
        }
        if idx % 10 == 1 && f[idx+11] != 0 {
            o_eval = o_eval - config.bishop_trapped_at_rim_malus;
        }

        let eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        eval + config.piece_eval_bishop
    }

    fn black_bishop(&self, idx: usize, _board: &Board, config: &Config, f: &[i32; 120], game_phase: i16, _movegen: &MoveGenService) -> i16 {
        let mut o_eval = 0;
        let e_eval = 0;

        if idx == 23 || idx == 26 {
            o_eval = o_eval + config.undeveloped_bishop_malus;
        }

        //let moves = movegen.generate_moves_list_for_piece(board, idx as i32);
        //o_eval -= moves.len() as i16 / 2 * config.move_freedom_bonus as i16;

        if idx % 10 == 8 && f[idx-11] != 0 { // TODO make this static to avoid modul
            o_eval = o_eval + config.bishop_trapped_at_rim_malus;
        }
        if idx % 10 == 1 && f[idx-9] != 0 {
            o_eval = o_eval + config.bishop_trapped_at_rim_malus;
        }

        let eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        eval - config.piece_eval_bishop
    }


    fn white_queen(&self, _idx: usize, board: &Board, config: &Config, _f: &[i32; 120], game_phase: i16, _movegen: &MoveGenService) -> i16 {
        let mut o_eval = 0;
        let e_eval = 0;

        //let moves = movegen.generate_moves_list_for_piece(board, idx as i32);
        //e_eval += moves.len() as i16 / 2 * config.move_freedom_bonus as i16;

        let in_attack = _movegen.get_attack_idx_list(_f, true, _idx as i32);
        if in_attack.len() > 0 {
            o_eval -= (config.queen_in_attack * in_attack.len() as i16) + if !board.white_to_move { config.queen_in_attack_with_tempo } else { 0 };
        }

        let eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        eval + config.piece_eval_queen
    }

    fn black_queen(&self, _idx: usize, board: &Board, config: &Config, _f: &[i32; 120], game_phase: i16, _movegen: &MoveGenService) -> i16 {
        let mut o_eval = 0;
        let e_eval = 0;

        //let moves = movegen.generate_moves_list_for_piece(board, idx as i32);
        //e_eval -= moves.len() as i16 / 2 * config.move_freedom_bonus as i16;

        let in_attack = _movegen.get_attack_idx_list(_f, false, _idx as i32);
        if in_attack.len() > 0 {
            o_eval += (config.queen_in_attack * in_attack.len() as i16) + if board.white_to_move { config.queen_in_attack_with_tempo } else { 0 };
        }

        let eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        eval - config.piece_eval_queen
    }
 
    fn white_king(&self, idx: usize, board: &Board, config: &Config, f: &[i32; 120], game_phase: i16, movegen: &MoveGenService) -> i16 {
        let mut o_eval = 0;
        let mut e_eval = 0;

        if idx == 94 || idx == 95 || idx == 96 || idx == 84 || idx == 85 || idx == 86 {
            o_eval -= config.undeveloped_king_malus
        }

        let in_check = movegen.get_attack_idx_list(&board.field, true, idx as i32).len();
        if in_check == 1 {
            o_eval -= config.king_in_check_malus;
            e_eval -= config.king_in_check_malus;
        } else if in_check > 1 {
            o_eval -= config.king_in_double_check_malus;
            e_eval -= config.king_in_double_check_malus;
        }

        if idx==43||idx==44||idx==45||idx==46||
           idx==53||idx==54||idx==55||idx==56||
           idx==63||idx==64||idx==65||idx==66||
           idx==73||idx==74||idx==75||idx==76 {
            e_eval += config.king_centered;
        }

        o_eval = o_eval + if f[idx-9]/10==1 { config.king_shield } else { 0 };
        o_eval = o_eval + if f[idx-10]/10==1 { config.king_shield } else { 0 };
        o_eval = o_eval + if f[idx-11]/10==1 { config.king_shield } else { 0 };

        if idx / 10 == 9 && idx - 9 != 0 && idx - 10 != 0 && idx - 11 != 0 {
            e_eval = e_eval - config.king_trapp_at_baseline_malus;
        }

        let eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        eval + config.piece_eval_king
    }

    fn black_king(&self, idx: usize, board: &Board, config: &Config, f: &[i32; 120], game_phase: i16, movegen: &MoveGenService) -> i16 {
        let mut o_eval = 0;
        let mut e_eval = 0;

        if idx == 24 || idx == 25 || idx == 26 || idx == 34 || idx == 35 || idx == 36 {
            o_eval += config.undeveloped_king_malus
        }

        let in_check = movegen.get_attack_idx_list(&board.field, false, idx as i32).len();
        if in_check == 1 {
            o_eval += config.king_in_check_malus;
            e_eval += config.king_in_check_malus;
        } else if in_check > 1 {
            o_eval += config.king_in_double_check_malus;
            e_eval += config.king_in_double_check_malus;
        }

        if idx==43||idx==44||idx==45||idx==46||
        idx==53||idx==54||idx==55||idx==56||
        idx==63||idx==64||idx==65||idx==66||
        idx==73||idx==74||idx==75||idx==76 {
         e_eval -= config.king_centered;
     }

        o_eval = o_eval - if f[idx+9]/20==1 { config.king_shield } else { 0 };
        o_eval = o_eval - if f[idx+10]/20==1 { config.king_shield } else { 0 };
        o_eval = o_eval - if f[idx+11]/20==1 { config.king_shield } else { 0 };

        if idx / 10 == 2 && idx + 9 != 0 && idx + 10 != 0 && idx + 11 != 0 {
            e_eval = e_eval + config.king_trapp_at_baseline_malus;
        }

        let eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        eval - config.piece_eval_king
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
        let field = board.field;
        let mut phase = 0;
        for idx in 21..99 {
            if field[idx] > 0 { phase = phase + 8; } else { continue };
        }
        let phase = phase - 48;
        let phase = if phase < 0 { 0 } else { (phase as f64 * 1.23) as u32 };
        phase
    }

    /// adjust eval when exchange pieces with advantage
    fn adjust_eval(&self, eval: i16, game_phase: i16, config: &Config) -> i16 {
        if game_phase + 100 < 255 && (eval <= -200 || eval >= 200) {
            let mut mult: f32 = 255 as f32 / (game_phase + 100) as f32;
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

}


#[cfg(test)]
mod tests {
    use crate::config::Config;
    use crate::global_map_handler;
    use crate::service::Service;

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
        eval_between("3qkb2/pppp1ppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQ - 0 1", 1800, 2000);
        eval_between("4k3/pppp1ppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQha - 0 1", 3000, 3300);

        // Figure values test for black
        eval_between("rnbqkbnr/pppppppp/8/8/8/8/PPPPP1PP/RNBQKBNR b KQkq - 0 1", -150, 50);
        eval_between("rnbqkbnr/pppppppp/8/8/8/8/PPPPP1PP/RNBQKB1R b KQkq - 0 1", -450, -350);
        eval_between("rnbqkbnr/pppppppp/8/8/8/8/PPPP1PPP/RN1QKB1R b KQkq - 0 1", -800, -600);
        eval_between("rnbqkbnr/pppppppp/8/8/8/8/PPPP1PPP/R2QKB1R b KQkq - 0 1", -1200, -900);
        eval_between("rnbqkbnr/pppppppp/8/8/8/8/PPPP1PPP/3QKB2 b - - 0 1", -2300, -1900);
        eval_between("rnbqkbnr/pppppppp/8/8/8/8/PPPP1PPP/4K3 b kq - 0 1", -3300, -3000);
    }

    #[test]
    fn position_unequel_test() {
        // position unequel
        eval_between("rnbqkb1r/pppppppp/8/5n2/8/5N2/PPPPPPPP/RNBQKB1R w KQkq - 0 1", -50, -10);
        eval_between("rnbqkb1r/pppppppp/5n2/8/5N2/8/PPPPPPPP/RNBQKB1R w KQkq - 0 1", 10, 50);
        eval_between("1k6/8/8/4P3/8/4p3/8/1K6 w - - 0 1", -150, -10);
        eval_between("1k6/3p4/8/4P3/8/4p3/3P4/1K6 w - - 0 1", -150, -10);
        eval_between("1k6/3p4/8/4P3/8/4p3/3P4/1K6 w - - 0 1", -150, -10);
    }

    #[test]
    fn compare_eval_test() {
        let fen_service = Service::new().fen;
        let eval_service = Service::new().eval;
        let movegen = &Service::new().move_gen;
        let config = &Config::new();
        let local_map = &global_map_handler::_get_default_local_map();

        let board = fen_service.set_fen("rnb1k1n1/pp4p1/2p3Nr/3p3p/q7/1RP3P1/3NPPBP/3QK2R w Kq - 3 19");
        let eval1 = eval_service.calc_eval(&board, config, movegen, local_map);

        let board = fen_service.set_fen("rnb1k1n1/pp4p1/2p3Nr/3B3p/q7/1RP3P1/3NPP1P/3QK2R b Kq - 0 19");
        let eval2 = eval_service.calc_eval(&board, config, movegen, local_map);

        let board = fen_service.set_fen("rnb1k1n1/pp4p1/6Nr/3p3p/q7/1RP3P1/3NPPBP/3QK2R w Kq - 3 19");
        let eval3 = eval_service.calc_eval(&board, config, movegen, local_map);

        let board = fen_service.set_fen("rnb1k3/pp2n1p1/7r/3p3p/q4N2/1RP3P1/3NPP1P/3QK2R w Kq - 2 21");
        let eval4 = eval_service.calc_eval(&board, config, movegen, local_map);

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
        fib("rnbqkbnr/ppppp1pp/8/8/6p1/6N1/PPPPPPPP/RNBQKB1R w KQkq - 0 1", "rnbqkbnr/ppppp1pp/8/8/6p1/4N3/PPPPPPPP/RNBQKB1R w KQkq - 0 1");
         
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
        // TODO add tests for black
    }


    #[test]
    fn game_phase_test() {
        let fen = Service::new().fen;
        let eval = Service::new().eval;

        // init board
        let board = fen.set_init_board();
        assert!(eval.get_game_phase(&board) > 254);
        assert!(eval.get_game_phase(&board) < 256);

        // 7 pieces board
        let board = fen.set_fen("8/8/2kq4/3ppp2/8/8/5N2/4K3 w - - 0 1");
        assert!(eval.get_game_phase(&board) > 8);
        assert!(eval.get_game_phase(&board) < 14);

        // 6 pieces board
        let board = fen.set_fen("8/8/2kq4/4pp2/8/8/5N2/4K3 w - - 0 1");
        assert_eq!(0, eval.get_game_phase(&board));

        // 3 pieces board
        let board = fen.set_fen("8/8/2k5/8/8/8/5N2/4K3 w - - 0 1");
        assert_eq!(0, eval.get_game_phase(&board));
    }

    #[test]
    pub fn double_pawn_test() {
        equal_eval("2k5/3p1p2/3p4/5p2/5P2/3P4/3P1P2/2K5 w - - 0 1");
        eval_between("2k5/5p2/5p2/8/8/8/4PP2/2K5 w - - 0 1", 0, 20);
        eval_between("2k5/4pp2/8/8/8/5P2/5P2/2K5 w - - 0 1", -20, 0);
        fib("2k5/4pp2/8/8/8/4P3/5P2/2K5 w - - 0 1", "2k5/4pp2/8/8/8/5P2/5P2/2K5 w - - 0 1");
        fib("2k5/5p2/4p3/8/8/8/4PP2/2K5 w - - 0 1", "2k5/5p2/4p3/8/5P2/8/5P2/2K5 w - - 0 1");
    }

    #[test]
    pub fn print_eval_for_fig_test() {
        //print_eval_for_fig("rnbqkbnr/1p3ppp/p7/2p5/1P1p4/N4N2/P2PPPPP/R1BQKB1R b KQkq - 1 7");
        //print_eval_for_fig("rnbqkbnr/1p3ppp/8/1pp5/PP1p4/5N2/3PPPPP/R1BQKB1R w KQkq - 0 8");
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

        /*
        assert_eq!(0, eval_service.adjust_eval(0, 255, config));
        assert_eq!(-199, eval_service.adjust_eval(-199, 150, config));
        assert_eq!(199, eval_service.adjust_eval(199, 150, config));

        assert_eq!(-200, eval_service.adjust_eval(-200, 150, config));
        assert_eq!(200, eval_service.adjust_eval(200, 150, config));
        */
    }

    /// first is better
    fn fib(fen1: &str, fen2: &str) {
        let fen = Service::new().fen;
        let eval = Service::new().eval;
        let movegen = Service::new().move_gen;
        let config = Config::for_tests();
        let local_map = &global_map_handler::_get_default_local_map();

        let board1 = fen.set_fen(fen1);
        let board2 = fen.set_fen(fen2);
        let eval1 = eval.calc_eval(&board1, &config, &movegen, local_map);
        let eval2 = eval.calc_eval(&board2, &config, &movegen, local_map);

        if !(eval1 > eval2) {
            println!("-->> eval is not gt: {}", eval1 - eval2);
            assert!(false);
        }
    }


    fn equal_eval(fen: &str) {
        let fen_service = Service::new().fen;
        let mut eval_service = Service::new().eval;
        eval_service._set_custom_config(&Config::_for_evel_equal_tests());
        let movegen = Service::new().move_gen;
        let local_map = &global_map_handler::_get_default_local_map();

        let config = &Config::_for_evel_equal_tests();
        let board = &fen_service.set_fen(fen);
        let eval = eval_service.calc_eval(board, config, &movegen, local_map);
        assert_eq!(0, eval);
    }

    fn eval_between(fen: &str, lower: i16, higher: i16) {
        print!("Test: {} | ", fen);
        let fen_service = Service::new().fen;
        let eval_service = Service::new().eval;
        let movegen = Service::new().move_gen;
        let local_map = &global_map_handler::_get_default_local_map();

        let config = &Config::_for_evel_equal_tests();
        let board = &fen_service.set_fen(fen);
        let eval = eval_service.calc_eval(board, config, &movegen, local_map);
        println!("Eval: {}", eval);
        assert!(eval >= lower);
        assert!(eval <= higher);
    }

    fn _print_eval_for_fig(fen: &str) {
        let fen_service = Service::new().fen;
        let eval_service = Service::new().eval;
        let movegen = Service::new().move_gen;
        let local_map = &global_map_handler::_get_default_local_map();

        let board = &fen_service.set_fen(fen);
        let mut config = Config::new();
        config.print_eval_per_figure = true;
        eval_service.calc_eval(board, &config, &movegen, local_map);
        println!("------------");
    }


}