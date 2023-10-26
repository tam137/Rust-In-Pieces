use crate::board::Board;
use crate::config::Config;
use crate::turn::Turn;

pub fn calc_eval(board: &Board, turn: &Turn, config: &Config) -> i16 {
    let mut eval: i16 = 0;

    for idx  in 21..99 {
        let piece = board.get_field()[idx];
        let eval_for_piece = match piece {
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

fn white_pawn(idx: usize, board: &Board, config: &Config) -> i16 {
    0
}

fn white_rook(idx: usize, board: &Board, config: &Config) -> i16 {
    0
}

fn white_knight(idx: usize, board: &Board, config: &Config) -> i16 {
    0
}

fn white_bishop(idx: usize, board: &Board, config: &Config) -> i16 {
    0
}

fn white_queen(idx: usize, board: &Board, config: &Config) -> i16 {
    0
}


fn black_pawn(idx: usize, board: &Board, config: &Config) -> i16 {
    0
}

fn black_rook(idx: usize, board: &Board, config: &Config) -> i16 {
    0
}

fn black_knight(idx: usize, board: &Board, config: &Config) -> i16 {
    0
}

fn black_bishop(idx: usize, board: &Board, config: &Config) -> i16 {
    0
}

fn black_queen(idx: usize, board: &Board, config: &Config) -> i16 {
    0
}