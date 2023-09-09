use rand::Rng;
use crate::{Board, eval, turn};
use crate::board::GameState;
use crate::config::Config;
use crate::Stats;
use crate::Turn;

pub fn get_moves(mut board: &mut Board, depth: i32, white: bool, stats: &mut Stats, config: &Config) -> (Option<Turn>, i16) {
    let mut best_eval = if white { i16::MIN } else { i16::MAX };
    let mut best_move: Option<Turn> = None;

    let turns = board.get_turn_list(white, false);
    stats.add_created_nodes(turns.len());

    let mut alpha: i16 = i16::MIN;
    let mut beta: i16 = i16::MAX;

    for turn in turns {
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
        if (config.use_zobrist) {
            let board_hash = board.get_hash();
            match board.get_eval_for_hash(&board_hash) {
                Some(eval) => {
                    return *eval;
                },
                None => {
                    let eval =  eval::calc_eval(board, turn, config);
                    board.set_new_hash(&board_hash, eval);
                    return eval;
                }
            }
        } else {
            return eval::calc_eval(board, turn, config);
        }
    }

    let mut eval = if white { i16::MIN } else { i16::MAX };
    let turns = board.get_turn_list(white, false);
    stats.add_created_nodes(turns.len());

    if turns.len() == 0 {
        return match board.get_state() {
            &GameState::WhiteWin => i16::MAX - 1,
            &GameState::BlackWin => i16::MIN + 1,
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