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

    let mut alpha: i16 = i16::MIN;
    let mut beta: i16 = i16::MAX;

    for turn in turns {
        board.do_turn(&turn);
        let eval = minimax(board, depth - 1, !white, alpha, beta, stats, &turn, config);
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
        board.set_current_best(best_eval);
    }
    (best_move, best_eval)
}

fn minimax(board: &mut Board, depth: i32, white: bool, mut alpha: i16, mut beta: i16, stats: &mut Stats, turn: &Turn, config: &Config) -> i16 {

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
    let turns = board.get_turn_list(white, false, stats);
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
    return eval;
}


fn quiescence(board: &mut Board, depth: i32, white: bool, mut alpha: i16, mut beta: i16, stats: &mut Stats, turn: &Turn, config: &Config) -> i16 {
    let mut eval = if white { i16::MIN } else { i16::MAX };
    let hit_turns = board.get_turn_list(white, true, stats);
    let mut eval= check_hash_or_calculate_eval(board, stats, turn, config);

    if hit_turns.is_empty() || depth <= -config.search_depth_quite {
        return eval;
    } else {
        stats.add_created_nodes(hit_turns.len());
        let stand_pat_cut = if white {
            board.get_current_best() < eval
        } else {
            board.get_current_best() > eval
        };

        if stand_pat_cut {
            return eval;
        }

        for turn in hit_turns {
            board.do_turn(&turn);
            let child_eval = quiescence(board, depth - 1, !white, alpha, beta, stats, &turn, config);
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
    }
    return eval;

}


fn check_hash_or_calculate_eval(board: &mut Board, stats: &mut Stats, turn: &Turn, config: &Config) -> i16 {
    stats.add_eval_nodes(1);
    if config.use_zobrist {
        let board_hash = board.get_hash();
        match board.get_eval_for_hash(&board_hash) {
            Some(eval) => {
                stats.add_zobrist_hit(1);
                return *eval;
            },
            None => {
                let eval = eval::calc_eval(board, turn, config);
                board.set_new_hash(&board_hash, eval);
                return eval;
            }
        }
    } else {
        return eval::calc_eval(board, turn, config);
    }
}