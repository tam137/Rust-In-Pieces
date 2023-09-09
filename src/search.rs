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

pub fn get_best_move(board: &mut Board, depth: i32, white: bool, stats: &mut Stats, config: &Config) -> (Option<Turn>, i16) {
    return match config.search_algo {
        SearchAlgo::Zobrist => search_zobrist::get_moves(board.clone(), depth, white, stats, config),
        SearchAlgo::AlphaBeta => search_alphabeta::get_moves(board.clone(), depth, white, stats, config),
        SearchAlgo::Basic => search_basic::get_moves(board, depth, white, stats, config),

    }
}



//
// fn quiescence_search(board: &mut Board, depth: i32, mut alpha: i16, mut beta: i16, white: bool, stats: &mut Stats, turn: &Turn, config: &Config) -> i16 {
//     let mut eval = eval::calc_eval(board, turn, config);
//
//     if eval >= beta {
//         return beta;
//     }
//
//     alpha = alpha.max(eval);
//
//     if depth <= 0 || depth > config.search_depth_quite {
//         return eval;
//     }
//
//     let turns = board.get_turn_list(white, true);
//
//     if turns.is_empty() {
//         return eval;
//     }
//
//     for turn in turns {
//         let mut child_board = board.clone();
//         child_board.do_turn(&turn);
//
//         let child_eval = quiescence_search(&mut child_board, depth - 1, alpha, beta, !white, stats, &turn, config);
//
//         if white {
//             eval = eval.max(child_eval);
//             alpha = alpha.max(eval);
//         } else {
//             eval = eval.min(child_eval);
//             beta = beta.min(eval);
//         }
//
//         if beta <= alpha {
//             break;
//         }
//     }
//
//     eval
// }

