use std::collections::HashMap;
use crate::config::Config;
use crate::model::Board;


pub fn calc_eval(board: &Board, config: &Config) -> i16 {
    let mut eval: i16 = 0;
    let game_phase = get_game_phase(board) as i16;
    let field = &board.field;
    for idx in 21..99 {
        let piece = field[idx];
        let eval_for_piece: i16 = match piece {
            10 => white_pawn(idx, board, config, field, game_phase),
            11 => white_rook(idx, board, config, field, game_phase),
            12 => white_knight(idx, board, config, field, game_phase),
            13 => white_bishop(idx, board, config, field, game_phase),
            14 => white_queen(idx, board, config, field, game_phase),
            15 => white_king(idx, board, config, field, game_phase),
            20 => black_pawn(idx, board, config, field, game_phase),
            21 => black_rook(idx, board, config, field, game_phase),
            22 => black_knight(idx, board, config, field, game_phase),
            23 => black_bishop(idx, board, config, field, game_phase),
            24 => black_queen(idx, board, config, field, game_phase),
            25 => black_king(idx, board, config, field, game_phase),
            _ => 0,
        };
        eval = eval + eval_for_piece;
    }
    eval
}

pub fn calc_eval_piece_map(board: &Board, config: &Config) -> HashMap<usize, i16> {
    let mut eval: i16 = 0;
    let mut eval_map: HashMap<usize, i16> = HashMap::default();
    let game_phase = get_game_phase(board) as i16;
    let field = &board.field;

    for idx in 21..99 {
        let piece = field[idx];
        let eval_for_piece: i16 = match piece {
            10 => {
                let piece_eval = white_pawn(idx, board, config, field, game_phase);
                eval_map.insert(idx, piece_eval);
                piece_eval
            },
            11 => {
                let piece_eval = white_rook(idx, board, config, field, game_phase);
                eval_map.insert(idx, piece_eval);
                piece_eval
            },
            12 => {
                let piece_eval = white_knight(idx, board, config, field, game_phase);
                eval_map.insert(idx, piece_eval);
                piece_eval
            },
            13 => {
                let piece_eval = white_bishop(idx, board, config, field, game_phase);
                eval_map.insert(idx, piece_eval);
                piece_eval
            },
            14 => {
                let piece_eval = white_queen(idx, board, config, field, game_phase);
                eval_map.insert(idx, piece_eval);
                piece_eval
            },
            15 => {
                let piece_eval = white_king(idx, board, config, field, game_phase);
                eval_map.insert(idx, piece_eval);
                piece_eval
            },
            20 => {
                let piece_eval = black_pawn(idx, board, config, field, game_phase);
                eval_map.insert(idx, piece_eval);
                piece_eval
            },
            21 => {
                let piece_eval = black_rook(idx, board, config, field, game_phase);
                eval_map.insert(idx, piece_eval);
                piece_eval
            },
            22 => {
                let piece_eval = black_knight(idx, board, config, field, game_phase);
                eval_map.insert(idx, piece_eval);
                piece_eval
            },
            23 => {
                let piece_eval = black_bishop(idx, board, config, field, game_phase);
                eval_map.insert(idx, piece_eval);
                piece_eval
            },
            24 => {
                let piece_eval = black_queen(idx, board, config, field, game_phase);
                eval_map.insert(idx, piece_eval);
                piece_eval
            },
            25 => {
                let piece_eval = black_king(idx, board, config, field, game_phase);
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

fn white_pawn(idx: usize, board: &Board, config: &Config, f: &[i32; 120], game_phase: i16) -> i16 {
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

    let mut eval = calculate_weighted_eval(o_eval, e_eval, game_phase);
    eval + config.piece_eval_pawn
}

fn black_pawn(idx: usize, board: &Board, config: &Config, f: &[i32; 120], game_phase: i16) -> i16 {
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

    let mut eval = calculate_weighted_eval(o_eval, e_eval, game_phase);
    eval - config.piece_eval_pawn
}


fn white_rook(idx: usize, board: &Board, config: &Config, f: &[i32; 120], game_phase: i16) -> i16 {
    let mut eval = config.piece_eval_rook;
    eval
}

fn black_rook(idx: usize, board: &Board, config: &Config, f: &[i32; 120], game_phase: i16) -> i16 {
    let mut eval = -config.piece_eval_rook;
    eval
}


fn white_knight(idx: usize, board: &Board, config: &Config, f: &[i32; 120], game_phase: i16) -> i16 {
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

    let mut eval = calculate_weighted_eval(o_eval, e_eval, game_phase);
    eval + config.piece_eval_knight
}

fn black_knight(idx: usize, board: &Board, config: &Config, f: &[i32; 120], game_phase: i16) -> i16 {
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

    let mut eval = calculate_weighted_eval(o_eval, e_eval, game_phase);
    eval - config.piece_eval_knight
}


fn white_bishop(idx: usize, mut board: &Board, config: &Config, f: &[i32; 120], game_phase: i16) -> i16 {
    let mut o_eval = 0;
    let mut e_eval = 0;

    if idx == 23 || idx == 26 {
        o_eval = o_eval - config.undeveloped_bishop_malus;
    }

    let mut eval = calculate_weighted_eval(o_eval, e_eval, game_phase);
    eval + config.piece_eval_bishop
}

fn black_bishop(idx: usize, mut board: &Board, config: &Config, f: &[i32; 120], game_phase: i16) -> i16 {
    let mut o_eval = 0;
    let mut e_eval = 0;

    if idx == 93 || idx == 96 {
        o_eval = o_eval + config.undeveloped_bishop_malus;
    }

    let mut eval = calculate_weighted_eval(o_eval, e_eval, game_phase);
    eval - config.piece_eval_bishop
}


fn white_queen(idx: usize, board: &Board, config: &Config, f: &[i32; 120], game_phase: i16) -> i16 {
    let mut o_eval = 0;
    let mut e_eval = 0;
    let diagonals = get_diagonals(idx);

    for &field in diagonals.0.iter().chain(&diagonals.1) {
        if f[field] == 23 {
            o_eval = o_eval - config.queen_in_bishop_line_malus;
        }
    }

    let mut eval = calculate_weighted_eval(o_eval, e_eval, game_phase);
    eval + config.piece_eval_queen
}

fn black_queen(idx: usize, board: &Board, config: &Config, f: &[i32; 120], game_phase: i16) -> i16 {
    let mut o_eval = 0;
    let mut e_eval = 0;
    let diagonals = get_diagonals(idx);

    for &field in diagonals.0.iter().chain(&diagonals.1) {
        if f[field] == 13 {
            o_eval = o_eval + config.queen_in_bishop_line_malus;
        }
    }

    let mut eval = calculate_weighted_eval(o_eval, e_eval, game_phase);
    eval - config.piece_eval_queen
}


fn white_king(idx: usize, board: &Board, config: &Config, f: &[i32; 120], game_phase: i16) -> i16 {
    let mut o_eval = 0;
    let mut e_eval = 0;

    o_eval = o_eval + if f[idx-9]/10==1 { config.king_shield } else { 0 };
    o_eval = o_eval + if f[idx-10]/10==1 { config.king_shield } else { 0 };
    o_eval = o_eval + if f[idx-11]/10==1 { config.king_shield } else { 0 };

    let mut eval = calculate_weighted_eval(o_eval, e_eval, game_phase);
    eval + config.piece_eval_king
}

fn black_king(idx: usize, board: &Board, config: &Config, f: &[i32; 120], game_phase: i16) -> i16 {
    let mut o_eval = 0;
    let mut e_eval = 0;

    o_eval = o_eval - if f[idx+9]/20==1 { config.king_shield } else { 0 };
    o_eval = o_eval - if f[idx+10]/20==1 { config.king_shield } else { 0 };
    o_eval = o_eval - if f[idx+11]/20==1 { config.king_shield } else { 0 };

    let mut eval = calculate_weighted_eval(o_eval, e_eval, game_phase);
    eval - config.piece_eval_king
}


fn calculate_weighted_eval(o_eval: i16, e_eval: i16, game_phase: i16) -> i16 {
    let o_eval = o_eval as i32;
    let e_eval = e_eval as i32;
    let game_phase = game_phase as i32;
    let res = ((o_eval * game_phase) + (e_eval * (256 - game_phase))) / 256;
    debug_assert!(res < 32_767 && res > -32_767);
    res as i16
}

/// return Value of 255 means early game and values towards 0 means endgamephase
/// a middle value like 128 respects early and late game evaluation in the same weight
pub fn get_game_phase(board: &Board) -> u32 {
    let field = board.field;
    let mut phase = 0;
    for idx in 21..99 {
        if field[idx] > 0 { phase = phase + 8; } else { continue };
    }
    phase
}

pub fn get_diagonals(idx: usize) -> ([usize; 8], [usize; 8]) {
    let file = (idx % 10) as i32;
    let rank = (10 - (idx / 10)) as i32;

    let diagonal_8a = [21, 0, 0, 0, 0, 0, 0, 0];
    let diagonal_7b = [31, 22, 0, 0, 0, 0, 0, 0];
    let diagonal_6c = [41, 32, 23, 0, 0, 0, 0, 0];
    let diagonal_5d = [51, 42, 33, 24, 0, 0, 0, 0];
    let diagonal_4e = [61, 52, 43, 34, 25, 0, 0, 0];
    let diagonal_3f = [71, 62, 53, 44, 35, 26, 0, 0];
    let diagonal_2g = [81, 72, 63, 54, 45, 36, 27, 0];
    let diagonal_1h = [91, 82, 73, 64, 55, 46, 37, 28];
    let diagonal_a8 = [91, 82, 73, 64, 55, 46, 37, 28];
    let diagonal_b7 = [92, 83, 74, 65, 56, 47, 38, 0];
    let diagonal_c6 = [93, 84, 75, 66, 57, 48, 0, 0];
    let diagonal_d5 = [94, 85, 76, 67, 58, 0, 0, 0];
    let diagonal_e4 = [95, 86, 77, 68, 0, 0, 0, 0];
    let diagonal_f3 = [96, 87, 78, 0, 0, 0, 0, 0];
    let diagonal_g2 = [97, 88, 0, 0, 0, 0, 0, 0];
    let diagonal_h1 = [98, 0, 0, 0, 0, 0, 0, 0];

    let diagonal_8h = [28, 0, 0, 0, 0, 0, 0, 0];
    let diagonal_7g = [38, 27, 0, 0, 0, 0, 0, 0];
    let diagonal_6f = [48, 37, 26, 0, 0, 0, 0, 0];
    let diagonal_5e = [58, 47, 36, 25, 0, 0, 0, 0];
    let diagonal_4d = [68, 57, 46, 35, 24, 0, 0, 0];
    let diagonal_3c = [78, 67, 56, 45, 34, 23, 0, 0];
    let diagonal_2b = [88, 77, 66, 55, 44, 33, 22, 0];
    let diagonal_1a = [98, 87, 76, 65, 54, 43, 32, 21];
    let diagonal_g7 = [97, 86, 75, 64, 53, 42, 31, 0];
    let diagonal_f6 = [96, 85, 74, 63, 52, 41, 0, 0];
    let diagonal_e5 = [95, 84, 73, 62, 51, 0, 0, 0];
    let diagonal_d4 = [94, 83, 72, 61, 0, 0, 0, 0];
    let diagonal_c3 = [93, 82, 71, 0, 0, 0, 0, 0];
    let diagonal_b2 = [92, 81, 0, 0, 0, 0, 0, 0];
    let diagonal_a1 = [91, 0, 0, 0, 0, 0, 0, 0];

    let horizontale_1 = [91, 92, 93, 94, 95, 96, 97, 98];
    let horizontale_2 = [81, 82, 83, 84, 85, 86, 87, 88];
    let horizontale_3 = [71, 72, 73, 74, 75, 76, 77, 78];
    let horizontale_4 = [61, 62, 63, 64, 65, 66, 67, 68];
    let horizontale_5 = [51, 52, 53, 54, 55, 56, 57, 58];
    let horizontale_6 = [41, 42, 43, 44, 45, 46, 47, 48];
    let horizontale_7 = [31, 32, 33, 34, 35, 36, 37, 38];
    let horizontale_8 = [21, 22, 23, 24, 25, 26, 27, 28];

    let vertikale_a = [91, 81, 71, 61, 51, 41, 31, 21];
    let vertikale_b = [92, 82, 72, 62, 52, 42, 32, 22];
    let vertikale_c = [93, 83, 73, 63, 53, 43, 33, 23];
    let vertikale_d = [94, 84, 74, 64, 54, 44, 34, 24];
    let vertikale_e = [95, 85, 75, 65, 55, 45, 35, 25];
    let vertikale_f = [96, 86, 76, 66, 56, 46, 36, 26];
    let vertikale_g = [97, 87, 77, 67, 57, 47, 37, 27];
    let vertikale_h = [98, 88, 78, 68, 58, 48, 38, 28];


    let first = match file - rank {
        0 => diagonal_a8,
        1 => diagonal_b7,
        2 => diagonal_c6,
        3 => diagonal_d5,
        4 => diagonal_e4,
        5 => diagonal_f3,
        6 => diagonal_g2,
        7 => diagonal_h1,
        -1 => diagonal_2g,
        -2 => diagonal_3f,
        -3 => diagonal_4e,
        -4 => diagonal_5d,
        -5 => diagonal_6c,
        -6 => diagonal_7b,
        -7 => diagonal_8a,
        _ => { [0; 8] }
    };

    let second = match file + rank {
        16 => diagonal_8h,
        15 => diagonal_7g,
        14 => diagonal_6f,
        13 => diagonal_5e,
        12 => diagonal_4d,
        11 => diagonal_3c,
        10 => diagonal_2b,
        9 => diagonal_1a,
        8 => diagonal_g7,
        7 => diagonal_f6,
        6 => diagonal_e5,
        5 => diagonal_d4,
        4 => diagonal_c3,
        3 => diagonal_b2,
        2 => diagonal_a1,
        _ => { [0; 8] }
    };
    (first, second)
}



#[cfg(test)]
mod tests {
    use crate::config::Config;
    use crate::eval::calc_eval;
    use crate::fen_service::FenServiceImpl;

    #[test]
    fn get_eval_even_test() {
        equal_eval("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        equal_eval("rnbqkbnr/1ppp1pp1/8/8/8/8/1PPP1PP1/RNBQKBNR w KQkq - 0 1");
        equal_eval("rnbqk1n1/pppp1ppp/4p3/8/8/4P3/PPPP1PPP/RNBQK1N1 w HQhq - 0 1");
        equal_eval("rnk2bnr/pppppppp/8/8/8/8/PPPPPPPP/RNK2BNR w KQkq - 0 1");
        equal_eval("3qk1r1/ppppp1pp/3bbp1n/8/r7/R2BBP1N/PPPPP1PP/3QK1R1 w Kk - 0 1");
        equal_eval("r1b1k2r/ppp1p1p1/5P1p/2npN1B1/2NPn1b1/5p1P/PPP1P1P1/R1B1K2R w Qq - 0 1");
    }


    fn equal_eval(fen: &str) {
        let fen_service = FenServiceImpl;
        let config = &Config::new();
        let board = &fen_service.set_fen(fen);
        let eval = calc_eval(board, config);
        assert_eq!(0, eval);
    }


}