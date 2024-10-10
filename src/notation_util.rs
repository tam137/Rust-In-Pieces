use crate::model::Turn;
use std::collections::HashMap;

pub struct NotationUtil;

impl NotationUtil {
    /// Converts a notation field (like "e2") to an index on the 10x12 board.
    pub fn get_index_from_notation_field(notation: &str) -> Result<i32, String> {
        let err_msg = format!("Invalid notation: {}", notation);

        if notation.len() != 2 {
            return Err(err_msg);
        }

        let col = match notation.chars().nth(0) {
            Some('a') => 1,
            Some('b') => 2,
            Some('c') => 3,
            Some('d') => 4,
            Some('e') => 5,
            Some('f') => 6,
            Some('g') => 7,
            Some('h') => 8,
            _ => return Err(err_msg),
        };

        let row = 10 - notation.chars().nth(1).unwrap().to_digit(10).unwrap_or(0) as i32;
        if row < 2 || row > 9 {
            return Err(err_msg);
        }
        Ok((row * 10) + col)
    }

    /// Converts a notation move (like "e2e4") to a `Turn` object.
    pub fn get_turn_from_notation(notation_move: &str) -> Turn {
        let from = NotationUtil::get_index_from_notation_field(&notation_move[0..2])
            .expect("Invalid notation for from position");
        let to = NotationUtil::get_index_from_notation_field(&notation_move[2..4])
            .expect("Invalid notation for to position");
        let mut promotion = 0;

        // Promotion logic for white
        if notation_move.len() == 5 && notation_move.chars().nth(3) == Some('8') {
            promotion = match notation_move.chars().nth(4) {
                Some('n') => 12,
                _ => 14, // default to queen
            };
        }

        // Promotion logic for black
        if notation_move.len() == 5 && notation_move.chars().nth(3) == Some('1') {
            promotion = match notation_move.chars().nth(4) {
                Some('n') => 22,
                _ => 24, // default to queen
            };
        }
        Turn::new(from, to, 0, promotion, 0)
    }

    /// Converts a space-separated list of notation moves (like "e2e4 e7e5") to a list of `Turn` objects.
    pub fn get_turn_list_from_notation(notation_move_list: &str) -> Vec<Turn> {
        let mut turn_list = Vec::new();
        let algebraic_move_list: Vec<&str> = notation_move_list.split_whitespace().collect();

        for algebraic_move in algebraic_move_list {
            let turn = NotationUtil::get_turn_from_notation(algebraic_move);
            turn_list.push(turn);
        }

        turn_list
    }

    /// Finds a specific move in the move list based on the notation.
    pub fn get_turn_from_list(move_list: &Vec<Turn>, notation: &str) -> Turn {
        let mut target_turn = NotationUtil::get_turn_from_notation(notation);

        // Handle promotion
        if notation.len() == 5 {
            match notation.chars().nth(4) {
                Some('q') => target_turn.promotion = 14,
                Some('n') => target_turn.promotion = 12,
                _ => panic!("Invalid promotion"),
            }

            if target_turn.to / 90 == 1 {
                target_turn.promotion = target_turn.promotion + 10; // for black promotion
            }
        }

        for move_turn in move_list {
            if move_turn.from == target_turn.from
                && move_turn.to == target_turn.to
                && move_turn.promotion == target_turn.promotion
            {
                return move_turn.clone(); // Return the found move
            }
        }
        panic!("Turn not found in the move list for notation: {}", notation);
    }
}
