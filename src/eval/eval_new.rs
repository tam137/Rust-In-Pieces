use std::collections::HashMap;

use crate::board::Board;
use crate::config::Config;

#[macro_export]
macro_rules! fields {
    ($($x:expr),*) => {
        {
            vec![$($x),*]
        }
    };
}


pub fn calc_eval(board: &Board, config: &Config) -> i16 {
    let mut eval: i16 = 0;
    let field = board.get_field();
    for idx in 21..99 {
        let piece = field[idx];
        let eval_for_piece: i16 = match piece {
            10 => white_pawn(idx, board, config),
            11 => white_rook(idx, board, config),
            12 => white_knight(idx, board, config),
            13 => white_bishop(idx, board, config),
            14 => white_queen(idx, board, config),
            20 => black_pawn(idx, board, config),
            21 => black_rook(idx, board, config),
            22 => black_knight(idx, board, config),
            23 => black_bishop(idx, board, config),
            24 => black_queen(idx, board, config),
            _ => 0,
        };
        eval = eval + eval_for_piece;
    }
    eval
}

pub fn calc_eval_piece_map(board: &Board, config: &Config) -> HashMap<usize, i16> {
    let mut eval: i16 = 0;
    let mut eval_map: HashMap<usize, i16> = HashMap::default();
    let field = board.get_field();

    for idx in 21..99 {
        let piece = field[idx];
        let eval_for_piece: i16 = match piece {
            10 => {
                let piece_eval = white_pawn(idx, board, config);
                eval_map.insert(idx, piece_eval);
                piece_eval
            },
            11 => {
                let piece_eval = white_rook(idx, board, config);
                eval_map.insert(idx, piece_eval);
                piece_eval
            },
            12 => {
                let piece_eval = white_knight(idx, board, config);
                eval_map.insert(idx, piece_eval);
                piece_eval
            },
            13 => {
                let piece_eval = white_bishop(idx, board, config);
                eval_map.insert(idx, piece_eval);
                piece_eval
            },
            14 => {
                let piece_eval = white_queen(idx, board, config);
                eval_map.insert(idx, piece_eval);
                piece_eval
            },
            20 => {
                let piece_eval = black_pawn(idx, board, config);
                eval_map.insert(idx, piece_eval);
                piece_eval
            },
            21 => {
                let piece_eval = black_rook(idx, board, config);
                eval_map.insert(idx, piece_eval);
                piece_eval
            },
            22 => {
                let piece_eval = black_knight(idx, board, config);
                eval_map.insert(idx, piece_eval);
                piece_eval
            },
            23 => {
                let piece_eval = black_bishop(idx, board, config);
                eval_map.insert(idx, piece_eval);
                piece_eval
            },
            24 => {
                let piece_eval = black_queen(idx, board, config);
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

fn white_pawn(idx: usize, board: &Board, config: &Config) -> i16 {
    let mut eval = config.piece_eval_pawn;
    let moves_until_promote = idx / 10 - 2;
    let on_rank = 8 - moves_until_promote;
    match moves_until_promote {
        1 => eval = eval + config.pawn_on_last_rank_bonus,
        2 => eval = eval + config.pawn_on_before_last_rank_bonus,
        3 => eval = eval + config.pawn_on_before_before_last_rank_bonus,
        _ => ()
    }
    let field = board.get_field();
    if (on_rank >= 3) && (on_rank <= 5) {
        if field[idx-11] == 12 || field[idx-9] == 12 {
            eval = eval + config.pawn_supports_knight_outpost;
        }
    }
    if field[idx-9] == 10 || field[idx+1] == 10 || field[idx-11] == 10 {
        eval = eval + config.pawn_structure;
    }
    eval
}


fn white_rook(idx: usize, board: &Board, config: &Config) -> i16 {
    let mut eval = config.piece_eval_rook;
    eval
}


fn white_knight(idx: usize, board: &Board, config: &Config) -> i16 {
    let mut eval = config.piece_eval_knight;
    let on_rank = 8 - (idx / 10 - 2);
    let on_file = idx % 10;
    if on_rank == 1 || on_rank == 8 || on_file == 1 || on_file == 8 {
        eval -= config.knight_on_rim_malus;
    }
    let f = board.get_field();
    if f[idx-21]==21||f[idx-19]==21||f[idx-12]==21||f[idx-8]==21||f[idx+21]==21||f[idx+19]==21||f[idx+12]==21||f[idx+8]==21 {
        eval = eval + config.knight_attacks_rook;
    }
    if f[idx-21]==24||f[idx-19]==24||f[idx-12]==24||f[idx-8]==24||f[idx+21]==24||f[idx+19]==24||f[idx+12]==24||f[idx+8]==24 {
        eval = eval + config.knight_attacks_queen;
    }
    eval
}


fn white_bishop(idx: usize, mut board: &Board, config: &Config) -> i16 {
    let mut eval = config.piece_eval_bishop;
    let turns = board.generate_moves_list_for_piece(true, idx).len() / 2;
    eval = eval + turns as i16 * config.bishop_move_freedom;
    eval
}


fn white_queen(idx: usize, board: &Board, config: &Config) -> i16 {
    let mut eval = config.piece_eval_queen;
    let turns = board.generate_moves_list_for_piece(true, idx).len() / 2;
    eval = eval + turns as i16 * config.queen_move_freedom;
    eval
}


fn black_pawn(idx: usize, board: &Board, config: &Config) -> i16 {
    let mut eval = -config.piece_eval_pawn;
    let moves_until_promote = 9 - (idx / 10);
    let on_rank = 8 - moves_until_promote;
    match moves_until_promote {
        1 => eval = eval - config.pawn_on_last_rank_bonus,
        2 => eval = eval - config.pawn_on_before_last_rank_bonus,
        3 => eval = eval - config.pawn_on_before_before_last_rank_bonus,
        _ => ()
    }
    let field = board.get_field();
    if (on_rank >= 3) && (on_rank <= 5) {
        if field[idx+11] == 22 || field[idx+9] == 22 {
            eval = eval - config.pawn_supports_knight_outpost;
        }
    }
    if field[idx+9] == 20 || field[idx+1] == 20 || field[idx+11] == 20 {
        eval = eval - config.pawn_structure;
    }
    eval
}


fn black_rook(idx: usize, board: &Board, config: &Config) -> i16 {
    let mut eval = -config.piece_eval_rook;
    eval
}


fn black_knight(idx: usize, board: &Board, config: &Config) -> i16 {
    let mut eval = -config.piece_eval_knight;
    let on_rank = 8 - (idx / 10 - 2);
    let on_file = idx % 10;
    if on_rank == 1 || on_rank == 8 || on_file == 1 || on_file == 8 {
        eval += config.knight_on_rim_malus;
    }
    let f = board.get_field();
    if f[idx-21]==11||f[idx-19]==11||f[idx-12]==11||f[idx-8]==11||f[idx+21]==11||f[idx+19]==11||f[idx+12]==11||f[idx+8]==11 {
        eval = eval - config.knight_attacks_rook;
    }
    if f[idx-21]==14||f[idx-19]==14||f[idx-12]==14||f[idx-8]==14||f[idx+21]==14||f[idx+19]==14||f[idx+12]==14||f[idx+8]==14 {
        eval = eval - config.knight_attacks_queen;
    }
    eval
}


fn black_bishop(idx: usize, mut board: &Board, config: &Config) -> i16 {
    let mut eval = -config.piece_eval_bishop;
    let turns = board.generate_moves_list_for_piece(false, idx).len() / 2;
    eval = eval - turns as i16 * config.bishop_move_freedom;
    eval
}


fn black_queen(idx: usize, board: &Board, config: &Config) -> i16 {
    let mut eval = -config.piece_eval_queen;
    let turns = board.generate_moves_list_for_piece(false, idx).len() / 2;
    eval = eval - turns as i16 * config.queen_move_freedom;
    eval
}
