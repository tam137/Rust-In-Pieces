use rand::Rng;
use crate::{Board, eval, turn};
use crate::board::GameState;
use crate::config::Config;
use crate::Stats;
use crate::Turn;

pub fn get_moves(mut board: Board, depth: i32, white: bool, stats: &mut Stats, config: &Config) -> (Option<Turn>, i16) {
    let mut best_eval = if white { i16::min_value() } else { i16::max_value() };
    let mut best_move: Option<Turn> = None;

    let turns = board.get_turn_list(white, false, &mut Stats::new());
    stats.add_created_nodes(turns.len());

    let mut alpha: i16 = i16::min_value();
    let mut beta: i16 = i16::max_value();

    for turn in turns {
        //let mut child_board = board.clone();
        board.do_turn(&turn);
        let eval = minimax(&mut board, depth - 1, !white, alpha, beta, stats, &turn, config);
        board.do_undo_turn(&turn);
        if white {
            if eval > best_eval {
                best_eval = eval;
                alpha = eval;
                best_move = Some(turn);
            }
        } else {
            if eval < best_eval {
                best_eval = eval;
                beta = eval;
                best_move = Some(turn);
            }
        }
    }
    (best_move, best_eval)
}

fn minimax(board: &mut Board, depth: i32, white: bool, mut alpha: i16, mut beta: i16, stats: &mut Stats, turn: &Turn, config: &Config) -> i16 {
    if depth <= 0 {
        stats.add_eval_nodes(1);
        let fuzzy_eval = rand::thread_rng().gen_range(0.. config.eval_fuzzy + 1) - config.eval_fuzzy / 2;
        return eval::calc_eval(board, turn, config) + fuzzy_eval;
    }

    let mut eval = if white { i16::min_value() } else { i16::max_value() };
    let turns = board.get_turn_list(white, false, &mut Stats::new());
    stats.add_created_nodes(turns.len());

    if turns.len() == 0 {
        return match board.get_state() {
            &GameState::WhiteWin => i16::max_value() - 1,
            &GameState::BlackWin => i16::min_value() + 1,
            &GameState::Draw => 0,
            _ => panic!("no defined game end"),
        };
    }

    for turn in turns {
        stats.add_calculated_nodes(1);
        board.do_turn(&turn);
        let child_eval = minimax(board, depth - 1, !white, alpha, beta, stats, &turn, config);
        board.do_undo_turn(&turn);

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