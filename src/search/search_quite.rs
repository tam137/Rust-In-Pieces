use std::collections::{HashMap, VecDeque};

use crate::{Board, eval};
use crate::board::GameState;
use crate::config::Config;
use crate::Stats;
use crate::Turn;

pub fn get_moves(board: &mut Board, depth: i32, white: bool, stats: &mut Stats, config: &Config) -> Vec<(Option<Turn>, i16, VecDeque<Option<Turn>>)> {
    let mut best_eval = if white { i16::MIN } else { i16::MAX };
    let mut best_move: Option<Turn> = None;

    let turns = board.get_turn_list(white, false, stats);
    stats.add_created_nodes(turns.len());
    board.set_current_best(eval::calc_eval_material(board, config, &mut HashMap::new()));

    let mut best_move_row: VecDeque<Option<Turn>> = Default::default();
    let mut sorted_moves: Vec<(Option<Turn>, i16, VecDeque<Option<Turn>>)> = vec![];

    let mut alpha: i16 = i16::MIN;
    let mut beta: i16 = i16::MAX;

    for turn in turns {
        board.do_turn(&turn);
        let min_max_result = minimax(board, depth - 1, !white, alpha, beta, stats, &turn, config);
        let min_max_eval = min_max_result.1;
        board.do_undo_turn(&turn);
        if white {
            if min_max_eval > best_eval {
                best_eval = min_max_eval;
                alpha = min_max_eval;
                let mut best_move_row = min_max_result.2;
                best_move_row.insert(0, Some(turn.clone()));
                sorted_moves.push((Some(turn), min_max_eval, best_move_row));
                board.set_current_best(best_eval);
            }
        } else {
            if min_max_eval < best_eval {
                best_eval = min_max_eval;
                beta = min_max_eval;
                let mut best_move_row = min_max_result.2;
                best_move_row.insert(0, Some(turn.clone()));
                sorted_moves.push((Some(turn), min_max_eval, best_move_row));
                board.set_current_best(best_eval);
            }
        }
    }
    if white {
        sorted_moves.sort_by_key(|k| k.1);
        sorted_moves
    } else {
        sorted_moves.sort_by(|a, b| b.1.cmp(&a.1));
        sorted_moves
    }

}

fn minimax(board: &mut Board, depth: i32, white: bool, mut alpha: i16, mut beta: i16, stats: &mut Stats, turn: &Turn, config: &Config) -> (Option<Turn>, i16, VecDeque<Option<Turn>>) {

    let mut turns = Default::default();
    let mut best_move_row: VecDeque<Option<Turn>> = VecDeque::new();
    let eval = check_hash_or_calculate_eval(board, stats, turn, config);

    if depth <= 0 {
        turns = board.get_turn_list(white, true, stats);

        let stand_pat_cut = if white {
            board.get_current_best() < eval.1
        } else {
            board.get_current_best() > eval.1
        };

        if stand_pat_cut {
            return eval;
        }
        if turns.is_empty() {
            return eval
        }
    } else {
         turns = board.get_turn_list(white, false, stats);
    }

    let mut eval = if white { i16::MIN } else { i16::MAX };
    let mut best_move: Option<Turn> = None;

    stats.add_created_nodes(turns.len());

    if turns.len() == 0 {
        return match board.get_state() {
            &GameState::WhiteWin => (None, i16::MAX - 1, best_move_row),
            &GameState::BlackWin => (None, i16::MIN + 1, best_move_row),
            &GameState::Draw => (None, 0, Default::default()),
            _ => panic!("no defined game end"),
        };
    }

    for turn in turns {
        stats.add_calculated_nodes(1);
        board.do_turn(&turn);
        let min_max_result = minimax(board, depth - 1, !white, alpha, beta, stats, &turn, config);
        let min_max_eval = min_max_result.1;
        board.do_undo_turn(&turn);

        if white {
            if eval < min_max_eval {
                eval = min_max_eval;
                alpha = min_max_eval;
                best_move_row = min_max_result.2;
                best_move_row.insert(0, Some(turn.clone()));
                best_move = Some(turn);
            }
        }
        else {
            if eval > min_max_eval {
                eval = min_max_eval;
                beta = min_max_eval;
                best_move_row = min_max_result.2;
                best_move_row.insert(0, Some(turn.clone()));
                best_move = Some(turn);
            }
        }
        if beta <= alpha {
            break;
        }
    }
    return (best_move, eval, best_move_row);
}



fn check_hash_or_calculate_eval(board: &mut Board, stats: &mut Stats, turn: &Turn, config: &Config) -> (Option<Turn>, i16, VecDeque<Option<Turn>>) {
    stats.add_eval_nodes(1);
    let empty_vec: VecDeque<Option<Turn>> = VecDeque::new();
    if config.use_zobrist {
        let board_hash = board.get_hash();
        match board.get_eval_for_hash(&board_hash) {
            Some(eval) => {
                stats.add_zobrist_hit(1);
                return (None, *eval, empty_vec);
            },
            None => {
                let eval = eval::calc_eval(board, turn, config);
                board.set_new_hash(&board_hash, eval);
                return (None, eval, empty_vec);
            }
        }
    } else {
        return (None, eval::calc_eval(board, turn, config), empty_vec);
    }
}