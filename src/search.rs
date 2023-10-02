use std::collections::VecDeque;
use crate::Board;
use crate::config::Config;
use crate::stats::Stats;

use crate::Turn;

mod search_basic;
mod search_alphabeta;
mod search_zobrist;
mod search_quite;


#[derive(Clone)]
pub enum SearchAlgo {
    Basic,
    AlphaBeta,
    Zobrist,
    Quiescence
}

pub fn get_best_move_as_min_max_result(mut board: &mut Board, depth: i32, white: bool, stats: &mut Stats, config: &Config) -> MinMaxResult {
    let min_max_raw_result: Vec<(Option<Turn>, i16, VecDeque<Option<Turn>>)> = search_quite::get_moves(board, depth, white, stats, config);
    MinMaxResult::new(min_max_raw_result)
}

pub fn get_best_move(mut board: &mut Board, depth: i32, white: bool, stats: &mut Stats, config: &Config) -> (Option<Turn>, i16, VecDeque<Option<Turn>>) {
    let min_max_result: Vec<(Option<Turn>, i16, VecDeque<Option<Turn>>)> = search_quite::get_moves(board, depth, white, stats, config);
    min_max_result.get(0).unwrap().clone()
}

pub struct MinMaxResult {
    pub moves: Vec<(VecDeque<Option<Turn>>, i16)>,
}

impl MinMaxResult {
    pub fn new(min_max_raw_result: Vec<(Option<Turn>, i16, VecDeque<Option<Turn>>)>) -> MinMaxResult {
        let mut moves = vec![];
        for min_max_move in &min_max_raw_result {
            moves.push((min_max_move.clone().2, min_max_move.1));
        }
        MinMaxResult {
            moves,
        }
    }
}