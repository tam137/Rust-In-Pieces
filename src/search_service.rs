use std::collections::{VecDeque};
use crate::config::Config;
use crate::model::{Board, GameStatus, SearchResult, Stats, Turn, Variant};
use crate::service::Service;


pub struct SearchService;

impl SearchService {

    pub fn new() -> Self {
        SearchService
    }

    pub fn get_moves(&self, board: &mut Board, depth: i32, white: bool, stats: &mut Stats, config: &Config, service: &Service) -> SearchResult {

        let mut best_eval = if white { i16::MIN } else { i16::MAX };

        let turns = service.move_gen.generate_valid_moves_list(board, stats, service);
        stats.add_created_nodes(turns.len());
        board.current_best_eval = service.eval.calc_eval(board, config);

        let mut search_result: SearchResult = SearchResult::default();

        let mut alpha: i16 = i16::MIN;
        let mut beta: i16 = i16::MAX;

        for turn in turns {
            let mi = board.do_move(&turn);
            let min_max_result = self.minimax(board, &turn, depth - 1, !white, alpha, beta, stats, config, service);
            let min_max_eval = min_max_result.1;
            board.undo_move(&turn, mi);
            if white {
                if min_max_eval > best_eval {
                    best_eval = min_max_eval;
                    alpha = min_max_eval;
                    let mut best_move_row = min_max_result.2;
                    best_move_row.insert(0, Some(turn.clone()));
                    search_result.add_variant(Variant { best_move: Some(turn), move_row: best_move_row, eval: min_max_eval });
                    board.current_best_eval = best_eval;
                }
            } else {
                if min_max_eval < best_eval {
                    best_eval = min_max_eval;
                    beta = min_max_eval;
                    let mut best_move_row = min_max_result.2;
                    best_move_row.insert(0, Some(turn.clone()));
                    search_result.add_variant(Variant { best_move: Some(turn), move_row: best_move_row, eval: min_max_eval });
                    board.current_best_eval = best_eval;
                }
            }
        }
        if white {
            search_result.variants.sort_by(|a, b| b.eval.cmp(&a.eval)); // Highest first for white
            search_result
        } else {
            search_result.variants.sort_by(|a, b| a.eval.cmp(&b.eval)); // Lowest first for black
            search_result
        }
    }

    fn minimax(&self, board: &mut Board, turn: &Turn, depth: i32, white: bool, mut alpha: i16, mut beta: i16, stats: &mut Stats, config: &Config, service: &Service)
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
                if service.move_gen.generate_valid_moves_list(board, stats, service).is_empty() {
                    return match board.game_status {
                        GameStatus::WhiteWin => (None, i16::MAX - 1, best_move_row),
                        GameStatus::BlackWin => (None, i16::MIN + 1, best_move_row),
                        GameStatus::Draw => (None, 0, Default::default()),
                        _ => panic!("no defined game end"), // TODO define proper ends
                    };
                }
                return eval
            } else {
                turns = service.move_gen.generate_valid_moves_list_capture(board, stats, service);
                if turns.is_empty() {
                    // check for mate or draw
                    if service.move_gen.generate_valid_moves_list(board, stats, service).is_empty() {
                        return match board.game_status {
                            GameStatus::WhiteWin => (None, i16::MAX - 1, best_move_row),
                            GameStatus::BlackWin => (None, i16::MIN + 1, best_move_row),
                            GameStatus::Draw => (None, 0, Default::default()),
                            _ => panic!("no defined game end"), // TODO define proper ends
                        };
                    }
                    return eval
                }
            }
        } else {
            turns = service.move_gen.generate_valid_moves_list(board, stats, service);
        }

        let mut eval = if white { i16::MIN } else { i16::MAX };
        let mut best_move: Option<Turn> = None;

        stats.add_created_nodes(turns.len());

        if turns.len() == 0 { // TODO do not loose game in quite mode with zero moves
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
            let min_max_result = self.minimax(board, &turn, depth - 1, !white, alpha, beta, stats, config, service);
            let min_max_eval = min_max_result.1;
            board.undo_move(&turn, mi);

            if white {
                if eval < min_max_eval {
                    //board.current_best_eval = eval;
                    eval = min_max_eval;
                    alpha = min_max_eval;
                    best_move_row = min_max_result.2;
                    best_move_row.insert(0, Some(turn.clone()));
                    best_move = Some(turn);
                }
            } else {
                if eval > min_max_eval {
                    //board.current_best_eval = eval;
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


#[cfg(test)]
mod tests {
    use crate::{config::Config, eval_service, service::Service, Stats};

    #[test]
    #[ignore]
    fn white_matt_tests() {
        let fen_service = Service::new().fen;
        let search_service = Service::new().search;
        let config = &Config::new();
        
        let mut board = fen_service.set_fen("8/3K4/8/8/5RR1/8/k7/8 w - - 0 1");
        let result = search_service.get_moves(&mut board, 6, true, &mut Stats::new(), &config, &Service::new());
        assert_eq!(result.get_eval(), 32766);
        assert_eq!(result.get_best_move_algebraic(), "f4f3");

        let mut board = fen_service.set_fen("r1q1r1k1/ppppppp1/n1b4p/7N/2B1P2N/2B2Q1P/PPPP1PP1/R3R1K1 w Qq - 0 1");
        let result = search_service.get_moves(&mut board, 4, true, &mut Stats::new(), &config, &Service::new());
        assert_eq!(result.get_eval(), 32766);
        assert_eq!(result.get_best_move_algebraic(), "f3f7");
        

        let mut board = fen_service.set_fen("6rk/R2R4/7P/8/p1B2P2/2P4P/P5K1/8 w - - 5 39");
        let result = search_service.get_moves(&mut board, 6, true, &mut Stats::new(), &config, &Service::new());
        assert_eq!(result.get_eval(), 32766);
        assert_eq!(result.get_best_move_algebraic(), "c4g8");
    }


    #[test]
    #[ignore]
    fn black_matt_tests() {
        let fen_service = Service::new().fen;
        let search_service = Service::new().search;
        let config = &Config::new();
        
        let mut board = fen_service.set_fen("8/1p6/p1P5/2p5/K1p2P2/P2kPn1P/1r6/8 b - - 3 43");
        let result = search_service.get_moves(&mut board, 6, false, &mut Stats::new(), &config, &Service::new());
        assert_eq!(result.get_eval(), -32767);
        assert_eq!(result.get_best_move_algebraic(), "b7b6");

        
        let mut board = fen_service.set_fen("8/8/8/2k5/8/5p1r/1K6/8 b - - 0 1");
        let result = search_service.get_moves(&mut board, 8, false, &mut Stats::new(), &config, &Service::new());
        assert_eq!(result.get_eval(), -32767);
        assert_eq!(result.get_best_move_algebraic(), "f3f2");
        

        let mut board = fen_service.set_fen("8/5pkp/p5p1/4p3/1P3P2/P3P1KP/2q3P1/3r4 b - - 0 37");
        let result = search_service.get_moves(&mut board, 6, false, &mut Stats::new(), &config, &Service::new());
        assert_eq!(result.get_eval(), -32767);
        assert_eq!(result.get_best_move_algebraic(), "d1g1");
    }


    #[test]
    #[ignore]
    fn black_find_hit_move() {
        let fen_service = Service::new().fen;
        let search_service = Service::new().search;
        let eval_service = Service::new().eval;
        let config = &Config::new();
        
        let mut board = fen_service.set_fen("2r2rk1/1b2bppp/pqn1pn2/8/1PBB4/P3PN2/5PPP/RN1Q1RK1 b - - 2 14");
        let result = search_service.get_moves(&mut board, 2, false, &mut Stats::new(), &config, &Service::new());
        //result.print_all_variants();
        //assert!(result.get_eval() < -100);
        assert_eq!(result.get_best_move_algebraic(), "c6d4");
        
        let mut board = fen_service.set_fen("5rrq/4n3/p2pb3/pp1k4/2pP2N1/N1P1Q2P/1P1RKPP1/7R b - - 0 1");
        let result = search_service.get_moves(&mut board, 2, false, &mut Stats::new(), &config, &Service::new());
        //result.print_all_variants();
        assert!(result.get_eval() < 250);
        //assert_eq!(result.get_best_move_algebraic(), "g8g4");

        let mut board = fen_service.set_fen("4k3/5pp1/2r3np/2Ppp3/3BP3/7P/5PP1/3RR1K1 b - - 0 1");
        let result = search_service.get_moves(&mut board, 2, false, &mut Stats::new(), &config, &Service::new());
        //result.print_all_variants();
        //assert!(result.get_eval() < -100);
        //assert_eq!(result.get_best_move_algebraic(), "d5e4");

        let mut board = fen_service.set_fen("6k1/5pp1/5rnp/2Npb3/3PP3/r1P1R2P/5PP1/4BR1K b - - 0 1");
        let result = search_service.get_moves(&mut board, 2, false, &mut Stats::new(), &config, &Service::new());
        println!("{}", eval_service.calc_eval(&board, config));
        result.print_all_variants();
        //assert!(result.get_eval() < -50);
        //assert_eq!(result.get_best_move_algebraic(), "e5d4");

    }


    #[test]
    fn white_find_hit_move() {
        let fen_service = Service::new().fen;
        let search_service = Service::new().search;
        let eval_service = Service::new().eval;
        let config = &Config::new();
        
        let mut board = fen_service.set_fen("3r2nk/6pp/3p4/4p3/3BP3/8/3R2PP/6NK w - - 0 1");
        let result = search_service.get_moves(&mut board, 2, true, &mut Stats::new(), &config, &Service::new());
        //result.print_all_variants();
        assert_eq!(result.get_best_move_algebraic(), "d4e5");

        let mut board = fen_service.set_fen("7k/6pp/3p4/4n3/3QP3/8/3R2PP/7K w - - 0 1");
        let result = search_service.get_moves(&mut board, 2, true, &mut Stats::new(), &config, &Service::new());
        //result.print_all_variants();
        assert_eq!(result.get_best_move_algebraic(), "d4e5");

        let mut board = fen_service.set_fen("7k/6pp/3p1p2/4r3/n2QP3/8/3R2PP/7K w - - 0 1");
        let result = search_service.get_moves(&mut board, 2, true, &mut Stats::new(), &config, &Service::new());
        result.print_all_variants();
        assert_eq!(result.get_best_move_algebraic(), "d4a4");
        


    }

}
