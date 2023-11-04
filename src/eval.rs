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

#[derive(Clone)]
#[derive(PartialEq)]
pub enum EvalAlgo {
    Classic,
    Pieces,
}

pub fn calc_eval(board: &Board, turn: &Turn, config: &Config) -> i16 {
    if config.eval_algo == EvalAlgo::Pieces {
        eval_new::calc_eval(board, config)
    } else {
        eval_legacy::calc_eval(board, turn, config)
    }
}

pub fn calc_eval_legacy(board: &Board, turn: &Turn, config: &Config) -> i16 {
    eval_legacy::calc_eval(board, turn, config)
}

pub fn calc_eval_material(board: &Board, config: &Config) -> i16 {
    let mut eval = 0;
    let pieces_list = board.get_list_of_pieces();
    for piece in pieces_list {
        eval += config.get_eval_value_for_piece(piece as i8);
    }
    eval
}

/// Returns the evaluation value of a piece as value and the idx on Board as Key
///
/// (21, -500) means piece on field a8 has a evaluation value of -500
///
/// the key 0 contains the overall evaluation of the board
///
/// # Examples
///
/// eval!(eval_map)
///
/// eval_idx!(eval_map, idx)
///
/// eval_map(idx).unwrap()
pub fn calc_eval_piece_map(board: &Board, config: &Config) -> HashMap<usize, i16> {
    eval_new::calc_eval_piece_map(board, config)
}