

use crate::Board;
use crate::Turn;
use crate::config::Config;
use std::collections::HashMap;

pub fn calc_eval(board: &Board, turn: &Turn, config: &Config) -> i16 {
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

    let pieces_on_field = board.get_pieces_on_field();

    let mut eval: i16 = 0;
    let move_possibilities_white = if board.is_white_field(turn.to) { (turn.post_my.len() / 2) as i32 }  else { (turn.post_villain.len() / 2) as i32 } ;
    let move_possibilities_black = if board.is_white_field(turn.to) { (turn.post_villain.len() / 2) as i32 }  else { (turn.post_my.len() / 2) as i32 } ;

    eval += (move_possibilities_white * config.move_freedom_bonus) as i16 - (move_possibilities_black * config.move_freedom_bonus) as i16;

    let field = board.get_field();
    for &piece in field.iter() {
        if let Some(&value) = piece_values.get(&piece) {
            eval += value;
        } else {
            continue;
        }
    }



    // pawn progress
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

    if pieces_on_field >= 30 {
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

    if pieces_on_field > 26 {
        // early queen
        if board.get_field()[24] != 24 && board.get_field()[25] != 24 &&
        board.get_field()[35] != 24 && board.get_field()[34] != 24 && board.get_field()[33] != 24
        { eval += config.early_queen_malus; }

        if board.get_field()[94] != 14 && board.get_field()[95] != 14 &&
        board.get_field()[85] != 14 && board.get_field()[84] != 14 && board.get_field()[83] != 14 
        { eval -= config.early_queen_malus; }

        // I like castleing
        if board.get_field()[97] == 15 {
            eval += config.short_castle_bonus
        }
        if board.get_field()[93] == 15 { eval += config.long_castle_bonus }
        if board.get_field()[27] == 25 { eval -= config.short_castle_bonus }
        if board.get_field()[23] == 25 { eval -= config.long_castle_bonus }
    }

    
    

    eval as i16
}