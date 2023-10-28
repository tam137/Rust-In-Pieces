use std::collections::HashMap;

use crate::Board;
use crate::config::Config;
use crate::Turn;

mod eval_new;
mod eval_legacy;

#[derive(Eq, Hash, PartialEq)]
pub enum SemiResultKeys {
    PiecesOnBoard,
}

pub fn calc_eval(board: &Board, turn: &Turn, config: &Config) -> i16 {
    //eval_legacy::calc_eval(board, turn, config)
    *eval_new::calc_eval(board, config).get(&0).unwrap()
}

pub fn calc_eval_material(board: &Board, config: &Config, semi_results: &mut HashMap<SemiResultKeys, i32>) -> i16 {
    let mut eval = 0;
    let pieces_list = board.get_list_of_pieces();
    for piece in pieces_list {
        eval += config.get_eval_value_for_piece(piece as i8);
    }
    eval
}

pub fn calc_eval_piece_map(board: &Board, config: &Config) -> HashMap<usize, i16> {
    eval_new::calc_eval(board, config)
}