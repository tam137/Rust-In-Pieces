
use crate::Board;
use crate::Turn;
use crate::config::Config;
use std::collections::HashMap;

#[derive(Eq, Hash, PartialEq)]
pub enum SemiResultKeys {
    PiecesOnBoard,
}

pub fn calc_eval(board: &Board, turn: &Turn, config: &Config) -> i16 {

    let mut semi_results: HashMap<SemiResultKeys, i32> = HashMap::new();

    let pieces_on_field = board.get_pieces_on_field();
    semi_results.insert(SemiResultKeys::PiecesOnBoard, pieces_on_field);

    let mut eval = 0i16;
    eval += calc_eval_material(board, config, &mut semi_results);
    eval += move_possibilities(board, turn, config, &mut semi_results);
    eval += calc_pawn_progress(board, config, &mut semi_results);
    eval += calc_developed_pieces(board, config, &mut semi_results);
    eval += calc_early_queen(board, config, &mut semi_results);
    eval += calc_casteling(board, config, &mut semi_results);
    eval += calc_push_to_king(board, config, &mut semi_results);

    eval
}


fn get_semi(key: SemiResultKeys, semi_results: &HashMap<SemiResultKeys, i32>) -> i32 {
    semi_results.get(&key)
                .expect("semi eval result key not found")
                .clone()
}


pub fn calc_eval_material(board: &Board, config: &Config, semi_results: &mut HashMap<SemiResultKeys, i32>) -> i16 {

    let mut eval = 0;
    
    let mut piece_values: HashMap<i32, i16> = HashMap::new();
    piece_values.insert(10, 100);
    piece_values.insert(11, 500);
    piece_values.insert(12, 300);
    piece_values.insert(13, 325);
    piece_values.insert(14, 900);
    piece_values.insert(15, 10000);
    piece_values.insert(20, -100);
    piece_values.insert(21, -500);
    piece_values.insert(22, -300);
    piece_values.insert(23, -325);
    piece_values.insert(24, -900);
    piece_values.insert(25, -10000);


    let field = board.get_field();
    for &piece in field.iter() {
        if let Some(&value) = piece_values.get(&piece) {
            eval += value;
        } else {
            continue;
        }
    }
    eval
}


pub fn move_possibilities(board: &Board, turn: &Turn, config: &Config, semi_results: &mut HashMap<SemiResultKeys, i32>) -> i16 {
    let move_possibilities_white = if board.is_white_field(turn.to) {
        (turn.post_my.len() / 2) as i32
    } else {
        (turn.post_villain.len() / 2) as i32
    };

    let move_possibilities_black = if board.is_white_field(turn.to) {
        (turn.post_villain.len() / 2) as i32
    } else {
        (turn.post_my.len() / 2) as i32
    };

    let white_bonus = (move_possibilities_white * config.move_freedom_bonus) as i16;
    let black_penalty = (move_possibilities_black * config.move_freedom_bonus) as i16;

    white_bonus - black_penalty
}


pub fn calc_pawn_progress(board: &Board, config: &Config, semi_results: &mut HashMap<SemiResultKeys, i32>) -> i16 {
    let mut eval = 0;
    for i in 21.. 99 {
        if board.get_field()[i] == -11 { continue }
        if board.get_field()[i] == 10 { 
            if i / 10 == 3 { eval += config.pawn_on_last_rank_bonus}
            else if i / 10 == 4 { eval += config.pawn_on_before_last_rank_bonus}
            else if i / 10 == 5 { eval += config.pawn_on_before_before_last_rank_bonus}
        }
        else if board.get_field()[i] == 20 { 
            if i / 10 == 8 { eval -= config.pawn_on_last_rank_bonus}
            else if i / 10 == 7 { eval -= config.pawn_on_before_last_rank_bonus}
            else if i / 10 == 6 { eval -= config.pawn_on_before_before_last_rank_bonus}
        }
    }
    eval
}


pub fn calc_developed_pieces(board: &Board, config: &Config, semi_results: &mut HashMap<SemiResultKeys, i32>) -> i16 {
    let mut eval = 0;
    if get_semi(SemiResultKeys::PiecesOnBoard, &*semi_results) >= 30 {
        // develop pieces!
        if board.get_field()[92] == 12 { eval -= config.undeveloped_knight_malus }
        if board.get_field()[97] == 12 { eval -= config.undeveloped_knight_malus }
        if board.get_field()[22] == 22 { eval += config.undeveloped_knight_malus }
        if board.get_field()[27] == 22 { eval += config.undeveloped_knight_malus }

        if board.get_field()[93] == 13 { eval -= config.undeveloped_bishop_malus }
        if board.get_field()[96] == 13 { eval -= config.undeveloped_bishop_malus }
        if board.get_field()[23] == 23 { eval += config.undeveloped_bishop_malus }
        if board.get_field()[26] == 23 { eval += config.undeveloped_bishop_malus }

        if board.get_field()[84] == 10 { eval -= config.undeveloped_center_pawn_malus }
        if board.get_field()[85] == 10 { eval -= config.undeveloped_center_pawn_malus }
        if board.get_field()[34] == 20 { eval += config.undeveloped_center_pawn_malus }
        if board.get_field()[35] == 20 { eval += config.undeveloped_center_pawn_malus }
    }
    eval
}

pub fn calc_early_queen(board: &Board, config: &Config, semi_results: &mut HashMap<SemiResultKeys, i32>) -> i16 {
    let mut eval = 0;
    if get_semi(SemiResultKeys::PiecesOnBoard, &*semi_results) >= 26 {
        // early queen
        if board.get_field()[24] != 24 && board.get_field()[25] != 24 &&
        board.get_field()[35] != 24 && board.get_field()[34] != 24 && board.get_field()[33] != 24
        { eval += config.early_queen_malus; }

        if board.get_field()[94] != 14 && board.get_field()[95] != 14 &&
        board.get_field()[85] != 14 && board.get_field()[84] != 14 && board.get_field()[83] != 14 
        { eval -= config.early_queen_malus; }
    }
    eval
}


pub fn calc_casteling(board: &Board, config: &Config, semi_results: &mut HashMap<SemiResultKeys, i32>) -> i16 {
    let mut eval = 0;
    if get_semi(SemiResultKeys::PiecesOnBoard, &*semi_results) >= 26 {
        if board.get_field()[97] == 15 {
            eval += config.short_castle_bonus
        }
        if board.get_field()[93] == 15 { eval += config.long_castle_bonus }
        if board.get_field()[27] == 25 { eval -= config.short_castle_bonus }
        if board.get_field()[23] == 25 { eval -= config.long_castle_bonus }
    }
    eval
}

pub fn calc_push_to_king(board: &Board, config: &Config, semi_results: &mut HashMap<SemiResultKeys, i32>) -> i16 {
    let mut idx_black = 0;
    let mut idx_white = 0;
    let mut eval = 0;

    for idx in 21..99 {
        if board.get_field()[idx] == 15 { idx_white = idx as i16 };
        if board.get_field()[idx] == 25 { idx_black = idx as i16};
    }

    for idx in 21..99 as i16 {
        let fig = board.get_field()[idx as usize];

        match fig {
            11..= 14 => {
                let mut horizontal = (idx_black % 10 - idx % 10).abs();
                let mut vertical = (idx_black / 10 - idx / 10).abs();
                let fig_distance = horizontal + vertical;
                let offset = (15 - fig_distance) * config.max_push_bonus;
                eval = eval + offset;
            },
            21..= 24 => {
                let mut horizontal = (idx_white % 10 - idx % 10).abs();
                let mut vertical = (idx_white / 10 - idx / 10).abs();
                let fig_distance = horizontal + vertical;
                let offset = (15 - fig_distance) * config.max_push_bonus;
                eval = eval - offset;
            },
            _ => {}
        }
    }
    eval
}