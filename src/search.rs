use crate::{Board, search_zobrist};
use crate::config::Config;
use crate::search_basic;
use crate::search_alphabeta;
use crate::Stats;
use crate::Turn;


#[derive(Clone)]
pub enum SearchAlgo {
    Basic,
    AlphaBeta,
    Zobrist,
}

pub fn get_best_move(mut board: &mut Board, depth: i32, white: bool, stats: &mut Stats, config: &Config) -> (Option<Turn>, i16) {
    return match config.search_algo {
        SearchAlgo::Zobrist => search_zobrist::get_moves(board, depth, white, stats, config),
        SearchAlgo::AlphaBeta => search_alphabeta::get_moves(board.clone(), depth, white, stats, config),
        SearchAlgo::Basic => search_basic::get_moves(board, depth, white, stats, config),
    }
}