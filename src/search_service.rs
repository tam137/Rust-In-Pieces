use std::collections::VecDeque;
use std::collections::HashMap;
use crate::config::Config;
use crate::model::DataMapKey;
use crate::model::QuiescenceSearchMode;
use crate::model::{Board, GameStatus, SearchResult, Stats, Turn, Variant};
use crate::service::Service;


pub struct SearchService;

impl SearchService {

    pub fn new() -> Self {
        SearchService
    }

    pub fn get_moves(&self, board: &mut Board, depth: i32, white: bool, stats: &mut Stats, config: &Config,
        service: &Service, data_map: &HashMap<DataMapKey, i32>) -> SearchResult {

        let mut best_eval = if white { i16::MIN } else { i16::MAX };

        let turns = service.move_gen.generate_valid_moves_list(board, stats, service);

        let mut search_result: SearchResult = SearchResult::default();

        let mut alpha: i16 = i16::MIN;
        let mut beta: i16 = i16::MAX;

        let mut turn_counter = 0;

        for turn in &turns {
            turn_counter += 1;
            let mi = board.do_move(&turn);   
            let min_max_result = self.minimax(board, &turn, depth - 1, !white,
                alpha, beta, stats, config, service, &data_map);
            let min_max_eval = min_max_result.1;
            board.undo_move(&turn, mi);
            if white {
                if min_max_eval > best_eval {
                    best_eval = min_max_eval;
                    alpha = min_max_eval;
                    let mut best_move_row = min_max_result.2;
                    best_move_row.insert(0, Some(turn.clone()));
                    search_result.add_variant(Variant { best_move: Some(turn.clone()), move_row: best_move_row, eval: min_max_eval });
                    stats.best_turn_nr = turn_counter;
                }
            } else {
                if min_max_eval < best_eval {
                    best_eval = min_max_eval;
                    beta = min_max_eval;
                    let mut best_move_row = min_max_result.2;
                    best_move_row.insert(0, Some(turn.clone()));
                    search_result.add_variant(Variant { best_move: Some(turn.clone()), move_row: best_move_row, eval: min_max_eval });
                    stats.best_turn_nr = turn_counter;
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

    fn minimax(&self, board: &mut Board, turn: &Turn, depth: i32, white: bool,
        mut alpha: i16, mut beta: i16, stats: &mut Stats, config: &Config, service: &Service, data_map: &HashMap<DataMapKey, i32>)
        ->(Option<Turn>, i16, VecDeque<Option<Turn>>) {

        let mut turns: Vec<Turn> = Default::default();
        let mut best_move_row: VecDeque<Option<Turn>> = VecDeque::new();
        let eval: (Option<Turn>, i16, VecDeque<Option<Turn>>) = (None, turn.eval, Default::default());

/*
        if depth <= 0 && turn.from == 61 && turn.to == 72 && turn.capture == 11 && board.cached_hash == 6026442690037892337 {
            println!("stop");
        }
 */
        if depth <= 0 {

            let mut stand_pat_cut = true;

            
            if config.quiescence_search_mode == QuiescenceSearchMode::Alpha1 {
                stand_pat_cut = if white {
                    beta >= eval.1 || (turn.capture == 0 && !turn.gives_check)
                } else {
                    alpha <= eval.1 || (turn.capture == 0 && !turn.gives_check)
                };
            }
                
             

            if config.quiescence_search_mode == QuiescenceSearchMode::Alpha2 {
                stand_pat_cut = if white {
                    beta < eval.1 || (turn.capture == 0 && !turn.gives_check)
                } else {
                    alpha > eval.1 || (turn.capture == 0 && !turn.gives_check)
                };
            }
                

            if config.quiescence_search_mode == QuiescenceSearchMode::Alpha3 {
                stand_pat_cut = if white {
                    data_map.get(&DataMapKey::WhiteTrashhold).expect("RIP white_trashhold missed") < &(eval.1 as i32) || (turn.capture == 0 && !turn.gives_check)
                } else {
                    data_map.get(&DataMapKey::BlackTrashhold).expect("RIP black_trashhold missed") > &(eval.1 as i32) || (turn.capture == 0 && !turn.gives_check)
                };
            }
            

            /*
            if stand_pat_cut && turn.gives_check {
                turns = service.move_gen.generate_valid_moves_list(board, stats, service);
            }
            */          

            if stand_pat_cut && turns.is_empty(){
                // check for mate or draw or leave quitesearch
                if service.move_gen.generate_valid_moves_list(board, stats, service).is_empty() {
                    return match board.game_status {
                        GameStatus::WhiteWin => (None, i16::MAX - 1, best_move_row),
                        GameStatus::BlackWin => (None, i16::MIN + 1, best_move_row),
                        GameStatus::Draw => (None, 0, Default::default()),
                        _ => panic!("RIP no defined game end"),
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
                            _ => panic!("RIP no defined game end"),
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

        if turns.len() == 0 {
            return match board.game_status {
                GameStatus::WhiteWin => (None, i16::MAX - 1, best_move_row),
                GameStatus::BlackWin => (None, i16::MIN + 1, best_move_row),
                GameStatus::Draw => (None, 0, Default::default()),
                _ => panic!("RIP no defined game end"),
            };
        }

        let mut turn_counter = 0;

        for turn in &turns {
            turn_counter += 1;
            stats.add_calculated_nodes(1);
            let mi = board.do_move(&turn);
            let min_max_result = self.minimax(board, &turn, depth - 1, !white,
                alpha, beta, stats, config, service, &data_map);
            let min_max_eval = min_max_result.1;
            board.undo_move(&turn, mi);

            if white {
                if eval < min_max_eval {
                    eval = min_max_eval;
                    alpha = min_max_eval;
                    best_move_row = min_max_result.2;
                    best_move_row.insert(0, Some(turn.clone()));
                    best_move = Some(turn.clone());
                    if config.in_debug && turn_counter > 30 {
                        stats.add_turn_nr_gt_trashhold(1);
                        stats.add_log(format!("{}, move {} was the {} lvl:{}",
                        service.fen.get_fen(board), &turn.to_algebraic(), turn_counter, config.search_depth - depth));
                    };
                }
            } else {
                if eval > min_max_eval {
                    eval = min_max_eval;
                    beta = min_max_eval;
                    best_move_row = min_max_result.2;
                    best_move_row.insert(0, Some(turn.clone()));
                    best_move = Some(turn.clone());
                    if config.in_debug && turn_counter > 30 {
                        stats.add_turn_nr_gt_trashhold(1);
                        stats.add_log(format!("{}, move {} was the {} lvl:{}",
                        service.fen.get_fen(board), &turn.to_algebraic(), turn_counter, config.search_depth - depth));
                    };
                }
            }
            if beta <= alpha {
                break;
            }
        }
        return (best_move, eval, best_move_row);
    }
}


#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{config::Config, service::Service, Stats};

    #[test]
    #[ignore]
    fn white_matt_tests() {
        let fen_service = Service::new().fen;
        let search_service = Service::new().search;
        let config = &Config::new();
        
        let mut board = fen_service.set_fen("8/3K4/8/8/5RR1/8/k7/8 w - - 0 1");
        let result = search_service.get_moves(&mut board, 6, true, &mut Stats::new(), &config, &Service::new(), &HashMap::default());
        assert_eq!(result.get_eval(), 32766);
        assert_eq!(result.get_best_move_algebraic(), "f4f3");

        let mut board = fen_service.set_fen("r1q1r1k1/ppppppp1/n1b4p/7N/2B1P2N/2B2Q1P/PPPP1PP1/R3R1K1 w Qq - 0 1");
        let result = search_service.get_moves(&mut board, 4, true, &mut Stats::new(), &config, &Service::new(), &HashMap::default());
        assert_eq!(result.get_eval(), 32766);
        assert_eq!(result.get_best_move_algebraic(), "f3f7");
        

        let mut board = fen_service.set_fen("6rk/R2R4/7P/8/p1B2P2/2P4P/P5K1/8 w - - 5 39");
        let result = search_service.get_moves(&mut board, 6, true, &mut Stats::new(), &config, &Service::new(), &HashMap::default());
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
        let result = search_service.get_moves(&mut board, 6, false, &mut Stats::new(), &config, &Service::new(), &HashMap::default());
        assert_eq!(result.get_eval(), -32767);
        assert_eq!(result.get_best_move_algebraic(), "b7b6");

        
        let mut board = fen_service.set_fen("8/8/8/2k5/8/5p1r/1K6/8 b - - 0 1");
        let result = search_service.get_moves(&mut board, 8, false, &mut Stats::new(), &config, &Service::new(), &HashMap::default());
        assert_eq!(result.get_eval(), -32767);
        assert_eq!(result.get_best_move_algebraic(), "f3f2");
        

        let mut board = fen_service.set_fen("8/5pkp/p5p1/4p3/1P3P2/P3P1KP/2q3P1/3r4 b - - 0 37");
        let result = search_service.get_moves(&mut board, 6, false, &mut Stats::new(), &config, &Service::new(), &HashMap::default());
        assert_eq!(result.get_eval(), -32767);
        assert_eq!(result.get_best_move_algebraic(), "d1g1");
    }


    #[test]
    fn black_find_hit_move() {
        let fen_service = Service::new().fen;
        let search_service = Service::new().search;
        //let eval_service = Service::new().eval;
        let config = &Config::new();
        
        let mut board = fen_service.set_fen("2r2rk1/1b2bppp/pqn1pn2/8/1PBB4/P3PN2/5PPP/RN1Q1RK1 b - - 2 14");
        let result = search_service.get_moves(&mut board, 2, false, &mut Stats::new(), &config, &Service::new(), &HashMap::default());
        result.print_all_variants();
        assert!(result.get_eval() < -100);
        assert_eq!(result.get_best_move_algebraic(), "c6d4");

        let mut board = fen_service.set_fen("6k1/5pp1/5rnp/2Npb3/3PP3/r1P1R2P/5PP1/4BR1K b - - 0 1");
        let result = search_service.get_moves(&mut board, 2, false, &mut Stats::new(), &config, &Service::new(), &HashMap::default());
        //result.print_all_variants();
        assert!(result.get_eval() > 0);
        // assert_eq!(result.get_best_move_algebraic(), "e5d4"); // TODO activate

    }


    #[test]
    fn white_find_hit_move() {
        let fen_service = Service::new().fen;
        let search_service = Service::new().search;
        let config = &Config::new();

        let mut board = fen_service.set_fen("3r2nk/6pp/3p4/4p3/3BP3/8/3R2PP/6NK w - - 0 1");
        let result = search_service.get_moves(&mut board, 2, true, &mut Stats::new(), &config, &Service::new(), &HashMap::default());
        result.print_all_variants();
        assert_eq!(result.get_best_move_algebraic(), "d4e5");
        
        let mut board = fen_service.set_fen("7k/6pp/3p4/4n3/3QP3/8/3R2PP/7K w - - 0 1");
        let result = search_service.get_moves(&mut board, 2, true, &mut Stats::new(), &config, &Service::new(), &HashMap::default());
        //result.print_all_variants();
        assert_eq!(result.get_best_move_algebraic(), "d4e5");


        let mut board = fen_service.set_fen("7k/6pp/3p1p2/4r3/p2QP3/8/3R2PP/7K w - - 0 1");
        let result = search_service.get_moves(&mut board, 2, true, &mut Stats::new(), &config, &Service::new(), &HashMap::default());
        //result.print_all_variants();
        assert_eq!(result.get_best_move_algebraic(), "d4a4");

    }


    #[test]
    #[ignore]
    fn hit_move_unsolved() {
        let fen_service = Service::new().fen;
        let search_service = Service::new().search;
        let config = &Config::new();

        let mut board = fen_service.set_fen("4k3/5pp1/2r3np/2Ppp3/3BP3/7P/5PP1/3RR1K1 b - - 0 1");
        let result = search_service.get_moves(&mut board, 2, false, &mut Stats::new(), &config, &Service::new(), &HashMap::default());
        result.print_all_variants();
        //assert!(result.get_eval() < -100);
        //assert_eq!(result.get_best_move_algebraic(), "d5e4");
    }

    // 
    #[test]
    fn practical_moves_from_games() {
        let fen_service = Service::new().fen;
        let search_service = Service::new().search;
        let config = &Config::new();

        let mut board = fen_service.set_fen("r1q1k2r/p1pRbp2/5p2/1p5p/5B2/6P1/PPQ1PP1P/4KB1R b Kkq - 0 20");
        let result = search_service.get_moves(&mut board, 2, false, &mut Stats::new(), &config, &Service::new(), &HashMap::default());
        //result.print_all_variants();
        assert_eq!( "c8d7", result.get_best_move_algebraic());

        //  r2qk2r/pppbnppp/4pn2/bNQp4/5B2/2PP1N2/PP2PPPP/R3KB1R b KQkq - 6 9

        let mut board = fen_service.set_fen("7r/p1p2p1p/P3k1p1/2KR1nr1/2P5/8/8/8 w - - 2 35");
        let result = search_service.get_moves(&mut board, 2, true, &mut Stats::new(), &config, &Service::new(), &HashMap::default());
        assert_ne!("d5e5", result.get_best_move_algebraic());

        // hash 6026442690037892337
        let mut board = fen_service.set_fen("rnb1k1n1/pp4p1/2p3Nr/3p3p/q7/1RP3P1/3NPPBP/3QK2R w Kq - 3 19");
        let result = search_service.get_moves(&mut board, 4, true, &mut Stats::new(), &config, &Service::new(), &HashMap::default());
        result.print_all_variants();
    }

}
