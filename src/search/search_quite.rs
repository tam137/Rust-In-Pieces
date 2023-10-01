use std::collections::HashMap;
use std::ptr::null;
use crate::{Board, eval};
use crate::board::GameState;
use crate::config::Config;
use crate::Stats;
use crate::Turn;

pub fn get_moves(board: &mut Board, depth: i32, white: bool, stats: &mut Stats, config: &Config) -> (Option<Turn>, i16) {
    let mut best_eval = if white { i16::MIN } else { i16::MAX };
    let mut best_move: Option<Turn> = None;

    let turns = board.get_turn_list(white, false, stats);
    stats.add_created_nodes(turns.len());
    board.set_current_best(eval::calc_eval_material(board, config, &mut HashMap::new()));

    let best_move_row: Option<Turn> = None;

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
                best_move = Some(turn);
            }
        } else {
            if min_max_eval < best_eval {
                best_eval = min_max_eval;
                beta = min_max_eval;
                best_move = Some(turn);
            }
        }
        board.set_current_best(best_eval);
    }
    (best_move, best_eval)
}

fn minimax(board: &mut Board, depth: i32, white: bool, mut alpha: i16, mut beta: i16, stats: &mut Stats, turn: &Turn, config: &Config) -> (Option<Turn>, i16) {

    if depth <= 0 {
        let hit_turns = board.get_turn_list(white, true, stats);
        if hit_turns.is_empty() {
            return check_hash_or_calculate_eval(board, stats, turn, config);
        } else {
            stats.add_created_nodes(hit_turns.len());
            for turn in hit_turns {
                board.do_turn(&turn);
                quiescence(board, depth, !white, alpha, beta, stats, &turn, config);
                board.do_undo_turn(&turn);
            }
            return check_hash_or_calculate_eval(board, stats, turn, config);
        }
    }

    let mut eval = if white { i16::MIN } else { i16::MAX };
    let mut best_move: Option<Turn> = None;
    let turns = board.get_turn_list(white, false, stats);
    stats.add_created_nodes(turns.len());

    if turns.len() == 0 {
        return match board.get_state() {
            &GameState::WhiteWin => (None, i16::MAX - 1),
            &GameState::BlackWin => (None, i16::MIN + 1),
            &GameState::Draw => (None, 0),
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
                best_move = Some(turn);
            }
        }
        else {
            if eval > min_max_eval {
                eval = min_max_eval;
                beta = min_max_eval;
                best_move = Some(turn);
            }
        }
        if beta <= alpha {
            break;
        }
    }
    return (best_move, eval);
}


fn quiescence(board: &mut Board, depth: i32, white: bool, mut alpha: i16, mut beta: i16, stats: &mut Stats, turn: &Turn, config: &Config) -> (Option<Turn>, i16) {
    let mut eval = if white { i16::MIN } else { i16::MAX };
    let mut best_move: Option<Turn> = None;
    let hit_turns = board.get_turn_list(white, true, stats);
    let mut eval_result= check_hash_or_calculate_eval(board, stats, turn, config);

    if hit_turns.is_empty() || depth <= -config.search_depth_quite {
        return eval_result;
    } else {
        stats.add_created_nodes(hit_turns.len());
        let stand_pat_cut = if white {
            board.get_current_best() < eval
        } else {
            board.get_current_best() > eval
        };

        if stand_pat_cut {
            return eval_result;
        }

        for turn in hit_turns {
            board.do_turn(&turn);
            let quite_result = quiescence(board, depth - 1, !white, alpha, beta, stats, &turn, config);
            let quite_eval = quite_result.1;
            board.do_undo_turn(&turn);

            if white {
                if eval < quite_eval {
                    eval = quite_eval;
                    alpha = quite_eval;
                    best_move = Some(turn);
                }
            }
            else {
                if eval > quite_eval {
                    eval = quite_eval;
                    beta = quite_eval;
                    best_move = Some(turn);
                }
            }
            if beta <= alpha {
                break;
            }
        }
    }
    return (best_move, eval);

}


fn check_hash_or_calculate_eval(board: &mut Board, stats: &mut Stats, turn: &Turn, config: &Config) -> (Option<Turn>, i16) {
    stats.add_eval_nodes(1);
    if config.use_zobrist {
        let board_hash = board.get_hash();
        match board.get_eval_for_hash(&board_hash) {
            Some(eval) => {
                stats.add_zobrist_hit(1);
                return (None, *eval);
            },
            None => {
                let eval = eval::calc_eval(board, turn, config);
                board.set_new_hash(&board_hash, eval);
                return (None, eval);
            }
        }
    } else {
        return (None, eval::calc_eval(board, turn, config));
    }
}