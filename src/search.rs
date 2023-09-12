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

pub fn get_best_move(mut board: &mut Board, depth: i32, white: bool, stats: &mut Stats, config: &Config) -> (Option<Turn>, i16) {
    return match config.search_algo {
        SearchAlgo::Quiescence => search_quite::get_moves(board, depth, white, stats, config),
        SearchAlgo::Zobrist => search_zobrist::get_moves(board, depth, white, stats, config),
        SearchAlgo::AlphaBeta => search_alphabeta::get_moves(board.clone(), depth, white, stats, config),
        SearchAlgo::Basic => search_basic::get_moves(board, depth, white, stats, config),
    }
}
