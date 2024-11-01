use std::collections::HashMap;
use crate::config::Config;
use crate::model::Board;


pub struct EvalService;

impl EvalService {

    pub fn new() -> Self {
        EvalService
    }

    pub fn calc_eval(&self, board: &Board, config: &Config) -> i16 {
        let mut eval: i16 = 0;
        let game_phase = self.get_game_phase(board) as i16;
        // oder service.eval.get_game_phase(board) as i16
        let field = &board.field;
        for idx in 21..99 {
            let piece = field[idx];
            let eval_for_piece: i16 = match piece {
                10 => self.white_pawn(idx, board, config, field, game_phase),
                11 => self.white_rook(idx, board, config, field, game_phase),
                12 => self.white_knight(idx, board, config, field, game_phase),
                13 => self.white_bishop(idx, board, config, field, game_phase),
                14 => self.white_queen(idx, board, config, field, game_phase),
                15 => self.white_king(idx, board, config, field, game_phase),
                20 => self.black_pawn(idx, board, config, field, game_phase),
                21 => self.black_rook(idx, board, config, field, game_phase),
                22 => self.black_knight(idx, board, config, field, game_phase),
                23 => self.black_bishop(idx, board, config, field, game_phase),
                24 => self.black_queen(idx, board, config, field, game_phase),
                25 => self.black_king(idx, board, config, field, game_phase),
                _ => 0,
            };
            eval = eval + eval_for_piece;
        }
        eval
    }

    pub(crate) fn calc_eval_piece_map(&self, board: &Board, config: &Config) -> HashMap<usize, i16> {
        let mut eval: i16 = 0;
        let mut eval_map: HashMap<usize, i16> = HashMap::default();
        let game_phase = self.get_game_phase(board) as i16;
        let field = &board.field;

        for idx in 21..99 {
            let piece = field[idx];
            let eval_for_piece: i16 = match piece {
                10 => {
                    let piece_eval = self.white_pawn(idx, board, config, field, game_phase);
                    eval_map.insert(idx, piece_eval);
                    piece_eval
                },
                11 => {
                    let piece_eval = self.white_rook(idx, board, config, field, game_phase);
                    eval_map.insert(idx, piece_eval);
                    piece_eval
                },
                12 => {
                    let piece_eval = self.white_knight(idx, board, config, field, game_phase);
                    eval_map.insert(idx, piece_eval);
                    piece_eval
                },
                13 => {
                    let piece_eval = self.white_bishop(idx, board, config, field, game_phase);
                    eval_map.insert(idx, piece_eval);
                    piece_eval
                },
                14 => {
                    let piece_eval = self.white_queen(idx, board, config, field, game_phase);
                    eval_map.insert(idx, piece_eval);
                    piece_eval
                },
                15 => {
                    let piece_eval = self.white_king(idx, board, config, field, game_phase);
                    eval_map.insert(idx, piece_eval);
                    piece_eval
                },
                20 => {
                    let piece_eval = self.black_pawn(idx, board, config, field, game_phase);
                    eval_map.insert(idx, piece_eval);
                    piece_eval
                },
                21 => {
                    let piece_eval = self.black_rook(idx, board, config, field, game_phase);
                    eval_map.insert(idx, piece_eval);
                    piece_eval
                },
                22 => {
                    let piece_eval = self.black_knight(idx, board, config, field, game_phase);
                    eval_map.insert(idx, piece_eval);
                    piece_eval
                },
                23 => {
                    let piece_eval = self.black_bishop(idx, board, config, field, game_phase);
                    eval_map.insert(idx, piece_eval);
                    piece_eval
                },
                24 => {
                    let piece_eval = self.black_queen(idx, board, config, field, game_phase);
                    eval_map.insert(idx, piece_eval);
                    piece_eval
                },
                25 => {
                    let piece_eval = self.black_king(idx, board, config, field, game_phase);
                    eval_map.insert(idx, piece_eval);
                    piece_eval
                },
                _ => 0,
            };
            eval = eval + eval_for_piece;
        }
        eval_map.insert(0, eval);
        eval_map
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

        if f[idx-9] == 10 || f[idx+1] == 10 || f[idx-11] == 10 {
            e_eval = e_eval + config.pawn_structure;
        }

        if moves_until_promote >= 5 {
            o_eval = o_eval - config.pawn_undeveloped_malus;
        }

        let mut eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
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

        if f[idx+9] == 20 || f[idx+1] == 20 || f[idx+11] == 20 {
            e_eval = e_eval - config.pawn_structure;
        }

        if moves_until_promote >= 5 {
            o_eval = o_eval + config.pawn_undeveloped_malus;
        }

        let mut eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        eval - config.piece_eval_pawn
    }


    fn white_rook(&self, idx: usize, board: &Board, config: &Config, f: &[i32; 120], game_phase: i16) -> i16 {
        let mut eval = config.piece_eval_rook;
        eval
    }

    fn black_rook(&self, idx: usize, board: &Board, config: &Config, f: &[i32; 120], game_phase: i16) -> i16 {
        let mut eval = -config.piece_eval_rook;
        eval
    }


    fn white_knight(&self, idx: usize, board: &Board, config: &Config, f: &[i32; 120], game_phase: i16) -> i16 {
        let mut o_eval = 0;
        let mut e_eval = 0;
        let on_rank = 8 - (idx / 10 - 2);
        let on_file = idx % 10;

        if on_rank == 1 || on_rank == 8 || on_file == 1 || on_file == 8 {
            o_eval = o_eval - config.knight_on_rim_malus;
        }

        if f[idx-21]==21||f[idx-19]==21||f[idx-12]==21||f[idx-8]==21||f[idx+21]==21||f[idx+19]==21||f[idx+12]==21||f[idx+8]==21 {
            o_eval = o_eval + config.knight_attacks_rook;
        }

        if f[idx-21]==24||f[idx-19]==24||f[idx-12]==24||f[idx-8]==24||f[idx+21]==24||f[idx+19]==24||f[idx+12]==24||f[idx+8]==24 {
            o_eval = o_eval + config.knight_attacks_queen;
        }

        if idx == 92 || idx == 97 {
            o_eval = o_eval - config.undeveloped_knight_malus;
        }

        let mut eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        eval + config.piece_eval_knight
    }

    fn black_knight(&self, idx: usize, board: &Board, config: &Config, f: &[i32; 120], game_phase: i16) -> i16 {
        let mut o_eval = 0;
        let mut e_eval = 0;
        let on_rank = 8 - (idx / 10 - 2);
        let on_file = idx % 10;

        if on_rank == 1 || on_rank == 8 || on_file == 1 || on_file == 8 {
            o_eval = o_eval + config.knight_on_rim_malus;
        }

        if f[idx-21]==11||f[idx-19]==11||f[idx-12]==11||f[idx-8]==11||f[idx+21]==11||f[idx+19]==11||f[idx+12]==11||f[idx+8]==11 {
            o_eval = o_eval - config.knight_attacks_rook;
        }

        if f[idx-21]==14||f[idx-19]==14||f[idx-12]==14||f[idx-8]==14||f[idx+21]==14||f[idx+19]==14||f[idx+12]==14||f[idx+8]==14 {
            o_eval = o_eval - config.knight_attacks_queen;
        }

        if idx == 22 || idx == 27 {
            o_eval = o_eval + config.undeveloped_knight_malus;
        }

        let mut eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        eval - config.piece_eval_knight
    }


    fn white_bishop(&self, idx: usize, mut board: &Board, config: &Config, f: &[i32; 120], game_phase: i16) -> i16 {
        let mut o_eval = 0;
        let mut e_eval = 0;

        if idx == 23 || idx == 26 {
            o_eval = o_eval - config.undeveloped_bishop_malus;
        }

        let mut eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        eval + config.piece_eval_bishop
    }

    fn black_bishop(&self, idx: usize, mut board: &Board, config: &Config, f: &[i32; 120], game_phase: i16) -> i16 {
        let mut o_eval = 0;
        let mut e_eval = 0;

        if idx == 93 || idx == 96 {
            o_eval = o_eval + config.undeveloped_bishop_malus;
        }

        let mut eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        eval - config.piece_eval_bishop
    }


    fn white_queen(&self, idx: usize, board: &Board, config: &Config, f: &[i32; 120], game_phase: i16) -> i16 {
        let mut o_eval = 0;
        let mut e_eval = 0;

        let mut eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        eval + config.piece_eval_queen
    }

    fn black_queen(&self, idx: usize, board: &Board, config: &Config, f: &[i32; 120], game_phase: i16) -> i16 {
        let mut o_eval = 0;
        let mut e_eval = 0;

        let mut eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        eval - config.piece_eval_queen
    }


    fn white_king(&self, idx: usize, board: &Board, config: &Config, f: &[i32; 120], game_phase: i16) -> i16 {
        let mut o_eval = 0;
        let mut e_eval = 0;

        o_eval = o_eval + if f[idx-9]/10==1 { config.king_shield } else { 0 };
        o_eval = o_eval + if f[idx-10]/10==1 { config.king_shield } else { 0 };
        o_eval = o_eval + if f[idx-11]/10==1 { config.king_shield } else { 0 };

        let mut eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
        eval + config.piece_eval_king
    }

    fn black_king(&self, idx: usize, board: &Board, config: &Config, f: &[i32; 120], game_phase: i16) -> i16 {
        let mut o_eval = 0;
        let mut e_eval = 0;

        o_eval = o_eval - if f[idx+9]/20==1 { config.king_shield } else { 0 };
        o_eval = o_eval - if f[idx+10]/20==1 { config.king_shield } else { 0 };
        o_eval = o_eval - if f[idx+11]/20==1 { config.king_shield } else { 0 };

        let mut eval = self.calculate_weighted_eval(o_eval, e_eval, game_phase);
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
    pub fn get_game_phase(&self, board: &Board) -> u32 {
        let field = board.field;
        let mut phase = 0;
        for idx in 21..99 {
            if field[idx] > 0 { phase = phase + 8; } else { continue };
        }
        phase
    }

}


#[cfg(test)]
mod tests {
    use crate::config::Config;
    use crate::service::Service;

    #[test]
    fn get_eval_even_test() {
        equal_eval("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        equal_eval("rnbqkbnr/1ppp1pp1/8/8/8/8/1PPP1PP1/RNBQKBNR w KQkq - 0 1");
        equal_eval("rnbqk1n1/pppp1ppp/4p3/8/8/4P3/PPPP1PPP/RNBQK1N1 w HQhq - 0 1");
        equal_eval("rnk2bnr/pppppppp/8/8/8/8/PPPPPPPP/RNK2BNR w KQkq - 0 1");
        equal_eval("3qk1r1/ppppp1pp/3bbp1n/8/r7/R2BBP1N/PPPPP1PP/3QK1R1 w Kk - 0 1");
        equal_eval("r1b1k2r/ppp1p1p1/5P1p/2npN1B1/2NPn1b1/5p1P/PPP1P1P1/R1B1K2R w Qq - 0 1");
    }

    #[test]
    fn eval_fig_value_test() {
        // Figure values test for white
        eval_between("rnbqkbnr/pppp1ppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 50, 150);
        eval_between("rnbqkb1r/pppp1ppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 350, 450);
        eval_between("rn1qkb1r/pppp1ppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 600, 800);
        eval_between("r2qkb1r/pppp1ppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 850, 1050);
        eval_between("3qkb2/pppp1ppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQ - 0 1", 1750, 1950);
        eval_between("4k3/pppp1ppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQha - 0 1", 2900, 3200);

        // Figure values test for black
        eval_between("rnbqkbnr/pppppppp/8/8/8/8/PPPPP1PP/RNBQKBNR b KQkq - 0 1", -150, 50);
        eval_between("rnbqkbnr/pppppppp/8/8/8/8/PPPPP1PP/RNBQKB1R b KQkq - 0 1", -450, -350);
        eval_between("rnbqkbnr/pppppppp/8/8/8/8/PPPP1PPP/RN1QKB1R b KQkq - 0 1", -800, -600);
        eval_between("rnbqkbnr/pppppppp/8/8/8/8/PPPP1PPP/R2QKB1R b KQkq - 0 1", -1050, -850);
        eval_between("rnbqkbnr/pppppppp/8/8/8/8/PPPP1PPP/3QKB2 b - - 0 1", -1950, -1750);
        eval_between("rnbqkbnr/pppppppp/8/8/8/8/PPPP1PPP/4K3 b kq - 0 1", -3200, -2900);
    }


    fn equal_eval(fen: &str) {
        let fen_service = Service::new().fen;
        let eval_service = Service::new().eval;

        let config = &Config::new();
        let board = &fen_service.set_fen(fen);
        let eval = eval_service.calc_eval(board, config);
        assert_eq!(0, eval);
    }

    fn eval_between(fen: &str, lower: i16, higher: i16) {
        let fen_service = Service::new().fen;
        let eval_service = Service::new().eval;

        let config = &Config::new();
        let board = &fen_service.set_fen(fen);
        let eval = eval_service.calc_eval(board, config);
        assert!(eval >= lower);
        assert!(eval <= higher);
    }


}