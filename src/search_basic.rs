use rand::Rng;
use crate::{Board, eval};
use crate::board::GameState;
use crate::config::Config;
use crate::Stats;
use crate::Turn;

pub fn get_moves(board: &mut Board, depth: i32, white: bool, stats: &mut Stats, config: &Config) -> (Option<Turn>, i16) {
    let mut best_eval = if white { i16::min_value() } else { i16::max_value() };
    let mut best_move: Option<Turn> = None;
    let mut board_clone = board.clone();
    let turns = board_clone.get_turn_list(white, false, &mut Stats::new());

    for turn in turns {

        board_clone.do_turn(&turn);
        let eval = minimax(&mut board_clone, depth - 1, !white, i16::min_value(), i16::max_value(), stats, &turn, config);
        if white {
            if eval > best_eval {
                best_eval = eval;
                best_move = Some(turn);
            }
        } else {
            if eval < best_eval {
                best_eval = eval;
                best_move = Some(turn);
            }
        }
    }
    (best_move, best_eval)
}


fn minimax(board: &mut Board, depth: i32, white: bool, mut alpha: i16, mut beta: i16, stats: &mut Stats, turn: &Turn, config: &Config) -> i16 {
    if depth <= 0 {
        stats.add_eval_nodes(1);
        return eval::calc_eval(board, turn, config);
    }

    let mut eval = if white { i16::min_value() } else { i16::max_value() };
    let turns = board.get_turn_list(white, false, &mut Stats::new());

    if turns.len() == 0 {
        return match board.get_state() {
            &GameState::WhiteWin => i16::max_value() - 1,
            &GameState::BlackWin => i16::min_value() + 1,
            &GameState::Draw => 0,
            _ => panic!("no defined game end"),
        };
    }

    stats.add_created_nodes(turns.len());

    for turn in turns {
        stats.add_calculated_nodes(1);
        let mut child_board = board.clone();
        child_board.do_turn(&turn);

        let child_eval = minimax(&mut child_board, depth - 1, !white, alpha, beta, stats, &turn, config);

        if white {
            eval = eval.max(child_eval);
            alpha = alpha.max(eval);
        } else {
            eval = eval.min(child_eval);
            beta = beta.min(eval);
        }

        if beta <= alpha {
            break;
        }
    }

    eval
}