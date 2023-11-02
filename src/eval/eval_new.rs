use std::collections::{HashMap, HashSet};

use crate::board::Board;
use crate::config::Config;

#[macro_export]
macro_rules! fields {
    ($($x:expr),*) => {
        {
            use std::collections::HashSet;
            let mut set = HashSet::new();
            $(
                set.insert($x);
            )*
            set
        }
    };
}


pub fn calc_eval(board: &Board, config: &Config) -> HashMap<usize, i16> {
    let mut eval: i16 = 0;
    let pieces_map = board.get_pieces_map();
    let mut eval_map: HashMap<usize, i16> = HashMap::default();

    for idx in 21..99 {
        let piece = board.get_field()[idx];
        let eval_for_piece: i16 = match piece {
            10 => {
                let piece_eval = white_pawn(idx, board, config, &pieces_map);
                eval_map.insert(idx, piece_eval);
                piece_eval
            },
            11 => {
                let piece_eval = white_rook(idx, board, config, &pieces_map);
                eval_map.insert(idx, piece_eval);
                piece_eval
            },
            12 => {
                let piece_eval = white_knight(idx, board, config, &pieces_map);
                eval_map.insert(idx, piece_eval);
                piece_eval
            },
            13 => {
                let piece_eval = white_bishop(idx, board, config, &pieces_map);
                eval_map.insert(idx, piece_eval);
                piece_eval
            },
            14 => {
                let piece_eval = white_queen(idx, board, config, &pieces_map);
                eval_map.insert(idx, piece_eval);
                piece_eval
            },
            20 => {
                let piece_eval = black_pawn(idx, board, config, &pieces_map);
                eval_map.insert(idx, piece_eval);
                piece_eval
            },
            21 => {
                let piece_eval = black_rook(idx, board, config, &pieces_map);
                eval_map.insert(idx, piece_eval);
                piece_eval
            },
            22 => {
                let piece_eval = black_knight(idx, board, config, &pieces_map);
                eval_map.insert(idx, piece_eval);
                piece_eval
            },
            23 => {
                let piece_eval = black_bishop(idx, board, config, &pieces_map);
                eval_map.insert(idx, piece_eval);
                piece_eval
            },
            24 => {
                let piece_eval = black_queen(idx, board, config, &pieces_map);
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

fn white_pawn(idx: usize, board: &Board, config: &Config, pieces_map: &HashMap<i32, Vec<usize>>) -> i16 {
    let mut eval = config.piece_eval_pawn;
    let moves_until_promote = idx / 10 - 2;
    let on_rank = 8 - moves_until_promote;
    match moves_until_promote {
        1 => eval = eval + config.pawn_on_last_rank_bonus,
        2 => eval = eval + config.pawn_on_before_last_rank_bonus,
        3 => eval = eval + config.pawn_on_before_before_last_rank_bonus,
        _ => ()
    }
    if (on_rank >= 3) && (on_rank <= 5) {
        eval = eval + on_fields_figure(fields!(idx-11, idx-9), 12, pieces_map) * config.pawn_supports_knight_outpost;
    }
    eval + on_fields_figure(fields!(idx-9, idx+1, idx+11), 10, pieces_map) * config.pawn_structure
}

fn white_rook(idx: usize, board: &Board, config: &Config, pieces_map: &HashMap<i32, Vec<usize>>) -> i16 {
    let mut eval = config.piece_eval_rook;
    if let Some(rooks) = pieces_map.get(&11) {
        if rooks.len() == 2 && rooks[0]/10 == rooks[1]/10 {
            eval += config.rooks_on_same_rank;
        }
    }
    eval
}

fn white_knight(idx: usize, board: &Board, config: &Config, pieces_map: &HashMap<i32, Vec<usize>>) -> i16 {
    let mut eval = config.piece_eval_knight;
    let on_rank = 8 - (idx / 10 - 2);
    let on_file = idx % 10;
    if on_rank == 1 || on_rank == 8 || on_file == 1 || on_file == 8 {
        eval -= config.knight_on_rim_malus;
    }
    eval += on_fields_figure(fields!(idx-21, idx-19, idx-12, idx-8, idx+21, idx+19, idx+12, idx+8), 21, pieces_map) * config.knight_attacks_rook;
    eval += on_fields_figure(fields!(idx-21, idx-19, idx-12, idx-8, idx+21, idx+19, idx+12, idx+8), 24, pieces_map) * config.knight_attacks_queen;
    eval
}

fn white_bishop(idx: usize, mut board: &Board, config: &Config, pieces_map: &HashMap<i32, Vec<usize>>) -> i16 {
    let mut eval = config.piece_eval_bishop;
    let turns = if board.king_in_chess(false) {
        board.generate_moves_list_for_piece(true, idx).len() / 2
    } else {
        board.clone().get_turn_list_for_piece_on_idx(true, false, idx).len()
    };
    eval = eval + turns as i16 * config.bishop_move_freedom;
    eval
}

fn white_queen(idx: usize, board: &Board, config: &Config, pieces_map: &HashMap<i32, Vec<usize>>) -> i16 {
    config.piece_eval_queen
}


fn black_pawn(idx: usize, board: &Board, config: &Config, pieces_map: &HashMap<i32, Vec<usize>>) -> i16 {
    let mut eval = -config.piece_eval_pawn;
    let moves_until_promote = 9 - (idx / 10);
    let on_rank = 8 - moves_until_promote;
    match moves_until_promote {
        1 => eval = eval - config.pawn_on_last_rank_bonus,
        2 => eval = eval - config.pawn_on_before_last_rank_bonus,
        3 => eval = eval - config.pawn_on_before_before_last_rank_bonus,
        _ => ()
    }
    if (on_rank >= 3) && (on_rank <= 5) {
        eval = eval - on_fields_figure(fields!(idx+11, idx+9), 22, pieces_map) * config.pawn_supports_knight_outpost;
    }
    eval - on_fields_figure(fields!(idx-9, idx+1, idx+11), 20, pieces_map) * config.pawn_structure
}

fn black_rook(idx: usize, board: &Board, config: &Config, pieces_map: &HashMap<i32, Vec<usize>>) -> i16 {
    let mut eval = -config.piece_eval_rook;
    if let Some(rooks) = pieces_map.get(&21) {
        if rooks.len() == 2 && rooks[0]/10 == rooks[1]/10 {
            eval -= config.rooks_on_same_rank;
        }
    }
    eval
}

fn black_knight(idx: usize, board: &Board, config: &Config, pieces_map: &HashMap<i32, Vec<usize>>) -> i16 {
    let mut eval = -config.piece_eval_knight;
    let on_rank = 8 - (idx / 10 - 2);
    let on_file = idx % 10;
    if on_rank == 1 || on_rank == 8 || on_file == 1 || on_file == 8 {
        eval += config.knight_on_rim_malus;
    }
    eval -= on_fields_figure(fields!(idx-21, idx-19, idx-12, idx-8, idx+21, idx+19, idx+12, idx+8), 11, pieces_map) * config.knight_attacks_rook;
    eval -= on_fields_figure(fields!(idx-21, idx-19, idx-12, idx-8, idx+21, idx+19, idx+12, idx+8), 14, pieces_map) * config.knight_attacks_queen;
    eval
}

fn black_bishop(idx: usize, mut board: &Board, config: &Config, pieces_map: &HashMap<i32, Vec<usize>>) -> i16 {
    let mut eval = -config.piece_eval_bishop;
    let turns = if board.king_in_chess(true) {
        board.generate_moves_list_for_piece(false, idx).len() / 2
    } else {
        board.clone().get_turn_list_for_piece_on_idx(false, false, idx).len()
    };
    eval = eval - turns as i16 * config.bishop_move_freedom;
    eval
}

fn black_queen(idx: usize, board: &Board, config: &Config, pieces_map: &HashMap<i32, Vec<usize>>) -> i16 {
    -config.piece_eval_queen
}


fn on_fields_figure(fields: HashSet<usize>, piece: usize, pieces_map: &HashMap<i32, Vec<usize>>) -> i16 {
    let mut offset = 0;
    if let Some(piece_positions) = pieces_map.get(&(piece as i32)) {
        for field in fields {
            if (field >= 100) || field <= 20 { continue }
            if piece_positions.contains(&field) {
                offset += 1;
            }
        }
    }
    offset
}


fn calc_eval_material(config: &Config, pieces_map: &HashMap<i32, Vec<usize>>) -> i16 {
    let mut eval = 0;
    let pieces_list = pieces_map.keys();
    for piece in pieces_list {
        eval += config.get_eval_value_for_piece(*piece as i8);
    }
    eval
}

