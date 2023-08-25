
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
        stats.add_eval_nodes(1);
        let fuzzy_eval = rand::thread_rng().gen_range(0.. config.eval_fuzzy + 1) - config.eval_fuzzy / 2;
        if config.use_quiescence {
            return quiescence_search(board, config.search_depth_quite, alpha, beta, white, stats, turn, config) + fuzzy_eval;
        } else {
            return eval::calc_eval(board, turn, config) + fuzzy_eval;
        }
    }    

    let mut eval = if white { i16::min_value() } else { i16::max_value() };
    let turns = board.get_turn_list(white, false);
    
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



pub fn quiescence_search(board: &mut Board, depth: i32, mut alpha: i16, mut beta: i16, white: bool, stats: &mut Stats, turn: &Turn, config: &Config) -> i16 {
    let mut eval = eval::calc_eval(board, turn, config);

    if eval >= beta {
        return beta;
    }

    alpha = alpha.max(eval);

    if depth <= 0 || depth > config.search_depth_quite {
        return eval;
    }

    let turns = board.get_turn_list(white, true);
    
    if turns.is_empty() {
        return eval;
    }

    for turn in turns {
        let mut child_board = board.clone();
        child_board.do_turn(&turn);

        let child_eval = quiescence_search(&mut child_board, depth - 1, alpha, beta, !white, stats, &turn, config);

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

