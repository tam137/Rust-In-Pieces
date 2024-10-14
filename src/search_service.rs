use std::collections::{VecDeque};
use crate::config::Config;
use crate::model::{Board, GameStatus, Stats, Turn};
use crate::service::Service;


pub struct SearchService;

impl SearchService {

    pub fn new() -> Self {
        SearchService
    }

    pub fn get_moves(&self, board: &mut Board, depth: i32, white: bool, stats: &mut Stats, config: &Config, service: &Service)
                     -> Vec<(Option<Turn>, i16, VecDeque<Option<Turn>>)> {

        let mut best_eval = if white { i16::MIN } else { i16::MAX };

        let turns = service.move_gen.generate_valid_moves_list(board, stats, service);
        stats.add_created_nodes(turns.len());
        board.current_best_eval = *(service.eval.calc_eval_piece_map(board, config).get(&0).unwrap());

        let mut sorted_moves: Vec<(Option<Turn>, i16, VecDeque<Option<Turn>>)> = vec![];

        let mut alpha: i16 = i16::MIN;
        let mut beta: i16 = i16::MAX;

        for turn in turns {
            let mi = board.do_move(&turn);
            let min_max_result = self.minimax(board, depth - 1, !white, alpha, beta, stats, config, service);
            let min_max_eval = min_max_result.1;
            board.undo_move(&turn, mi);
            if white {
                if min_max_eval > best_eval {
                    best_eval = min_max_eval;
                    alpha = min_max_eval;
                    let mut best_move_row = min_max_result.2;
                    best_move_row.insert(0, Some(turn.clone()));
                    sorted_moves.push((Some(turn), min_max_eval, best_move_row));
                    board.current_best_eval = best_eval;
                }
            } else {
                if min_max_eval < best_eval {
                    best_eval = min_max_eval;
                    beta = min_max_eval;
                    let mut best_move_row = min_max_result.2;
                    best_move_row.insert(0, Some(turn.clone()));
                    sorted_moves.push((Some(turn), min_max_eval, best_move_row));
                    board.current_best_eval = best_eval;
                }
            }
        }
        if white {
            sorted_moves.sort_by(|a, b| b.1.cmp(&a.1)); // Highest first for white
            sorted_moves
        } else {
            sorted_moves.sort_by(|a, b| a.1.cmp(&b.1)); // Lowest first for black
            sorted_moves
        }
    }

    fn minimax(&self, board: &mut Board, depth: i32, white: bool, mut alpha: i16, mut beta: i16, stats: &mut Stats, config: &Config, service: &Service)
        ->(Option<Turn>, i16, VecDeque<Option<Turn>>) {

        let mut turns: Vec<Turn> = Default::default();
        let mut best_move_row: VecDeque<Option<Turn>> = VecDeque::new();
        let eval: (Option<Turn>, i16, VecDeque<Option<Turn>>) = self.check_hash_or_calculate_eval(board, stats, config, service);

        if depth <= 0 {
            let stand_pat_cut = if white {
                board.current_best_eval < eval.1
            } else {
                board.current_best_eval > eval.1
            };

            // if !turn.gives_chess && stand_pat_cut {
            //     return eval;
            // }
            //
            // if turn.gives_chess {
            //     turns = board.get_turn_list(white, false, stats);
            // } else {
            //     turns = board.get_turn_list(white, true, stats);
            // }

            if stand_pat_cut {
                return eval;
            }

            if turns.is_empty() {
                return eval
            }
        } else {
            turns = service.move_gen.generate_valid_moves_list(board, stats, service);
        }

        let mut eval = if white { i16::MIN } else { i16::MAX };
        let mut best_move: Option<Turn> = None;

        stats.add_created_nodes(turns.len());

        if turns.len() == 0 {
            return match board.game_status {
                GameStatus::WhiteWin => (None, i16::MAX - 1, best_move_row),
                GameStatus::BlackWin => (None, i16::MIN + 1, best_move_row),
                GameStatus::Draw => (None, 0, Default::default()),
                _ => panic!("no defined game end"), // TODO define proper ends
            };
        }

        for turn in turns {
            stats.add_calculated_nodes(1);
            let mi = board.do_move(&turn);
            let min_max_result = self.minimax(board, depth - 1, !white, alpha, beta, stats, config, service);
            let min_max_eval = min_max_result.1;
            board.undo_move(&turn, mi);

            if white {
                if eval < min_max_eval {
                    eval = min_max_eval;
                    alpha = min_max_eval;
                    best_move_row = min_max_result.2;
                    best_move_row.insert(0, Some(turn.clone()));
                    best_move = Some(turn);
                }
            } else {
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


    pub fn check_hash_or_calculate_eval(&self, board: &mut Board, stats: &mut Stats, config: &Config, service: &Service) -> (Option<Turn>, i16, VecDeque<Option<Turn>>) {
        stats.add_eval_nodes(1);
        let empty_vec: VecDeque<Option<Turn>> = VecDeque::new();
        (None, service.eval.calc_eval(board, config), empty_vec)

        /*
        return if config.use_zobrist {

            let board_hash = board.hash();
            match board.get_eval_for_hash(&board_hash) {
                Some(eval) => {
                    stats.add_zobrist_hit(1);
                    (None, *eval, empty_vec)
                },
                None => {
                    let eval = service.eval.calc_eval(board, config);
                    board.set_new_hash(&board_hash, eval);
                    (None, eval, empty_vec)
                }
            }
        } else {
            (None, service.eval.calc_eval(board, config), empty_vec)
        }
    */
    }
}