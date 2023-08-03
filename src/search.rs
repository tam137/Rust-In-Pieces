
use crate::Turn;
use crate::Board;
use crate::Stats;
use crate::board::GameState;
use crate::eval;
use crate::config::Config;
use rand::Rng;


pub fn get_best_move(board: &mut Board, depth: i32, white: bool, stats: &mut Stats, config: &Config) -> (Option<Turn>, i16) {
    let mut best_eval = if white { i16::min_value() } else { i16::max_value() };
    let mut best_move: Option<Turn> = None;
    let turns = board.get_turn_list(white, false);

    for turn in turns {
        let mut child_board = board.clone();
        child_board.do_turn(&turn);
        let eval = minimax(&mut child_board, depth - 1, !white, i16::min_value(), i16::max_value(), stats, &turn, config);
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



pub fn minimax(board: &mut Board, depth: i32, white: bool, mut alpha: i16, mut beta: i16, stats: &mut Stats, turn: &Turn, config: &Config) -> i16 {
    if depth <= 0 {
        if config.use_quiescence {
            let fuzzy_eval = rand::thread_rng().gen_range(0.. config.eval_fuzzy + 1) - config.eval_fuzzy / 2;
            return quiesce(board, depth-1, white, alpha, beta, stats, turn, config) + fuzzy_eval;
        } else {
            stats.add_eval_nodes(1);
            let fuzzy_eval = rand::thread_rng().gen_range(0.. config.eval_fuzzy + 1) - config.eval_fuzzy / 2;
            return eval::calc_eval(board, turn, config) + fuzzy_eval;
        }
    }

    if white {
        let mut max_eval = i16::min_value();
        let turns = board.get_turn_list(white, false);
        if turns.len() == 0 {
            if board.get_state() == &GameState::BlackWin { return i16::min_value() + 1 }
            else if board.get_state() == &GameState::Draw { return 0 }
            else { panic!("no defined game end") }
        }
        stats.add_created_nodes(turns.len());
        for turn in turns {
            stats.add_calculated_nodes(1);
            let mut child_board = board.clone();
            child_board.do_turn(&turn);
            let eval = minimax(&mut child_board, depth - 1, false, alpha, beta, stats, &turn, config);
            max_eval = max_eval.max(eval);
            alpha = alpha.max(max_eval);
            if beta <= alpha {
                break;
            }
        }
        return max_eval;
    } else {
        let mut min_eval = i16::max_value();
        let turns = board.get_turn_list(white, false);
        if turns.len() == 0 {
            if board.get_state() == &GameState::WhiteWin { return i16::max_value() - 1 }
            else if board.get_state() == &GameState::Draw { return 0 }
            else { panic!("no defined game end") }
        }
        stats.add_created_nodes(turns.len());
        for turn in turns {
            stats.add_calculated_nodes(1);
            let mut child_board = board.clone();
            child_board.do_turn(&turn);
            let eval = minimax(&mut child_board, depth - 1, true, alpha, beta, stats, &turn, config);
            min_eval = min_eval.min(eval);
            beta = beta.min(min_eval);
            if beta <= alpha {
                break;
            }
        }
        return min_eval;
    }



    fn quiesce(board: &mut Board, depth: i32, white: bool, mut alpha: i16, mut beta: i16, stats: &mut Stats, turn: &Turn, config: &Config) -> i16 {
        let stand_pat = eval::calc_eval(board, turn, config);
        
        if stand_pat >= beta && white {
            return beta;
        }
        
        if stand_pat <= alpha && !white {
            return alpha;
        }
        
        alpha = alpha.max(stand_pat);
        beta = beta.min(stand_pat);
        let mut max_eval = i16::min_value();
        let mut min_eval = i16::max_value();
        let caps = board.get_turn_list(white, true);
        stats.add_created_nodes(caps.len());
        for cap in caps {
            stats.add_calculated_nodes(1);
            let mut child_board = board.clone();
            child_board.do_turn(&cap);
            let eval = -quiesce(&mut child_board, depth - 1, !white, -beta, -alpha, stats, &cap, config);
            if white {
                max_eval = max_eval.max(eval);
                alpha = alpha.max(max_eval);
                if beta <= alpha {
                    break;
                }
            } else {
                min_eval = min_eval.min(eval);
                beta = beta.min(min_eval);
                if beta <= alpha {
                    break;
                }
            }
        }
        if white {
            return alpha;
        } else {
            return beta;
        }
    }
}