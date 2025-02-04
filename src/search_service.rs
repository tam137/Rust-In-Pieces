use std::collections::VecDeque;
use std::time::Instant;

use crate::config::Config;
use crate::model::{Board, DataMap, DataMapKey, GameStatus, QuiescenceSearchMode, SearchResult, Stats, ThreadSafeDataMap, Turn, Variant, RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE};
use crate::service::Service;
use crate::global_map_handler;

use crate::model::RIP_MISSED_DM_KEY;


pub struct SearchService;

impl SearchService {

    pub fn new() -> Self {
        SearchService
    }

    pub fn get_moves(&self, board: &mut Board, depth: i32, white: bool, stats: &mut Stats, config: &Config,
        service: &Service, global_map: &ThreadSafeDataMap, local_map: &mut DataMap) -> SearchResult {

        let logger = global_map_handler::get_log_buffer_sender(global_map);

        let mut best_eval = if white { i16::MIN } else { i16::MAX };
        let turns = service.move_gen.generate_valid_moves_list(board, stats, config, global_map, local_map);
        let mut search_result: SearchResult = SearchResult::default();
        search_result.calculated_depth = depth;
        search_result.is_pv_search_result = *local_map.get_data::<bool>(DataMapKey::PvFlag).unwrap_or_else(|| &false);

        let mut alpha: i16 = i16::MIN;
        let mut beta: i16 = i16::MAX;

        let mut turn_counter = 0;

        for turn in &turns {
            turn_counter += 1;
            let mi = board.do_move(&turn);
            let min_max_result = self.minimax(board, &turn, depth - 1, !white,
                alpha, beta, stats, config, service, global_map, local_map);

            if global_map_handler::is_stop_flag(global_map) {
                let calc_time_ms = self.get_calc_time(&local_map);
                search_result.stats = stats.clone();
                search_result.stats.best_turn_nr = turn_counter;
                search_result.stats.calc_time_ms = calc_time_ms as usize;
                break;
            }

            let min_max_eval = min_max_result.1;

            // save min max eval in zobrist table for better move sorting, if depth = 2
            // TODO missing test or remove
            if depth == 2 && config.use_zobrist {
                let hash_sender = global_map_handler::get_hash_sender(global_map);
                hash_sender.push((board.cached_hash, min_max_eval));
            }


            board.undo_move(&turn, mi);
            if white {
                if min_max_eval > best_eval {
                    best_eval = min_max_eval;
                    alpha = min_max_eval;
                    let mut best_move_row = min_max_result.2;
                    best_move_row.insert(0, Some(turn.clone()));
                    search_result.add_variant(Variant { best_move: Some(turn.clone()), move_row: best_move_row, eval: min_max_eval });
                    search_result.is_white_move = white;
                    search_result.variants.sort_by(|a, b| b.eval.cmp(&a.eval)); // Highest first for white
                    stats.best_turn_nr = turn_counter;
                    let calc_time_ms = self.get_calc_time(&local_map);
                    stats.calc_time_ms = calc_time_ms as usize;
                    stats.calculate();
                    if config.print_info_string_during_search {
                        if let Err(_e) = service.stdout.write_get_result(&service.uci_parser.get_info_str(&search_result, stats)) {
                            logger.send("stdout channel closed during search".to_string())
                                .expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                            break;
                        }
                    }
                }
            } else {
                if min_max_eval < best_eval {
                    best_eval = min_max_eval;
                    beta = min_max_eval;
                    let mut best_move_row = min_max_result.2;
                    best_move_row.insert(0, Some(turn.clone()));
                    search_result.add_variant(Variant { best_move: Some(turn.clone()), move_row: best_move_row, eval: min_max_eval });
                    search_result.is_white_move = white;
                    search_result.variants.sort_by(|a, b| a.eval.cmp(&b.eval)); // Lowest first for black
                    search_result.stats.best_turn_nr = turn_counter;
                    let calc_time_ms = self.get_calc_time(&local_map);
                    stats.calc_time_ms = calc_time_ms as usize;
                    stats.calculate();
                    if config.print_info_string_during_search {
                        if let Err(_e) = service.stdout.write_get_result(&service.uci_parser.get_info_str(&search_result, stats)) {
                            logger.send("stdout channel closed during search".to_string())
                                .expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                            break;
                        }
                    } 
                }
            }
        }        
        
        let calc_time_ms = self.get_calc_time(&local_map);
        search_result.stats = stats.clone();
        search_result.stats.calc_time_ms = calc_time_ms as usize;
        if global_map_handler::is_stop_flag(global_map) {
            search_result.completed = false;
        } else {
            search_result.completed = true;
        }
        global_map_handler::push_search_result(global_map, search_result.clone());
        logger.send(format!("pushed search result calculated depth {}", search_result.calculated_depth))
            .expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
        search_result
    }
    

    fn minimax(&self, board: &mut Board, turn: &Turn, depth: i32, white: bool,
        mut alpha: i16, mut beta: i16, stats: &mut Stats, config: &Config, service: &Service,
        global_map: &ThreadSafeDataMap, local_map: &mut DataMap)
        -> (Option<Turn>, i16, VecDeque<Option<Turn>>) {

        let mut turns: Vec<Turn> = Default::default();
        let mut best_move_row: VecDeque<Option<Turn>> = VecDeque::new();

/*
        // for debug
        if depth <= 0 && turn.from == 61 && turn.to == 72 && turn.capture == 11 && board.cached_hash == 6026442690037892337 {
            println!("stop");
        }
 */
        if depth <= 0 {
            let eval = if turn.has_hashed_eval {
                turn.eval
            } else {
                stats.add_eval_nodes(1);
                if board.white_to_move && turn.gives_check { // the turn is already applied to the board, so invert is_white
                    local_map.insert(DataMapKey::BlackGivesCheck, true);
                }
                else if !board.white_to_move && turn.gives_check {
                    local_map.insert(DataMapKey::WhiteGivesCheck, true);
                } else {
                    local_map.insert(DataMapKey::WhiteGivesCheck, false);
                    local_map.insert(DataMapKey::BlackGivesCheck, false);
                }
                
                let eval = service.eval.calc_eval(board, config, &service.move_gen, &local_map);
                if config.use_zobrist {
                    let hash_sender = global_map_handler::get_hash_sender(global_map); // todo extract from map
                    hash_sender.push((board.cached_hash, eval)); // TODO critical Test board.cached_hash is correct here
                }
                eval
            };

            let mut stand_pat_cut = true;
            local_map.insert(DataMapKey::ForceSkipValidationFlag, false);

            if config.quiescence_search_mode == QuiescenceSearchMode::Alpha2 {
                stand_pat_cut = if white {
                    beta < eval || (turn.capture == 0)
                } else {
                    alpha > eval || (turn.capture == 0)
                };
            }
                

            if config.quiescence_search_mode == QuiescenceSearchMode::Alpha3 {
                stand_pat_cut = if white {
                    self.get_white_threshold_value(&local_map) < eval as i32 || turn.capture == 0
                } else {
                    self.get_black_threshold_value(&local_map) > eval as i32 || turn.capture == 0
                };
            }
            

            /*
            if stand_pat_cut && turn.gives_check {
                turns = service.move_gen.generate_valid_moves_list(board, stats, service);
            }
            */          

            if stand_pat_cut && turns.is_empty(){
                // check for mate or draw or leave quitesearch
                if service.move_gen.generate_valid_moves_list(board, stats, config, global_map, local_map).is_empty() {
                    return match board.game_status {
                        GameStatus::WhiteWin => (None, i16::MAX - 1, best_move_row),
                        GameStatus::BlackWin => (None, i16::MIN + 1, best_move_row),
                        GameStatus::Draw => (None, 0, Default::default()),
                        _ => panic!("RIP no defined game end"),
                    };
                }
                return (None, eval, Default::default());
            } else {
                turns = service.move_gen.generate_valid_moves_list_capture(board, stats, config, global_map, local_map);
                if turns.is_empty() {
                    // check for mate or draw
                    if service.move_gen.generate_valid_moves_list(board, stats, config, global_map, local_map).is_empty() {
                        return match board.game_status {
                            GameStatus::WhiteWin => (None, i16::MAX - 1, best_move_row),
                            GameStatus::BlackWin => (None, i16::MIN + 1, best_move_row),
                            GameStatus::Draw => (None, 0, Default::default()),
                            _ => panic!("RIP no defined game end"),
                        };
                    }
                    return (None, eval, Default::default());
                }
            }
        } else {
            local_map.insert(DataMapKey::ForceSkipValidationFlag, config.skip_strong_validation);
            turns = service.move_gen.generate_valid_moves_list(board, stats, config, global_map, local_map);
        }

        let mut eval = if white { i16::MIN } else { i16::MAX };
        let mut best_move: Option<Turn> = None;

        if turns.len() == 0 || board.game_status != GameStatus::Normal {
            return match board.game_status {
                GameStatus::WhiteWin => (None, i16::MAX - 1, best_move_row),
                GameStatus::BlackWin => (None, i16::MIN + 1, best_move_row),
                GameStatus::Draw => (None, 0, Default::default()),
                _ => panic!("RIP no defined game end"),
            };
        }

        let mut turn_counter = 0;

        for turn in &turns {
            if global_map_handler::is_stop_flag(global_map) {
                break;
            }
            turn_counter += 1;
            stats.add_calculated_nodes(1);
            let mi = board.do_move(&turn);
            let min_max_result = self.minimax(board, &turn, depth - 1, !white,
                alpha, beta, stats, config, service, global_map, local_map);
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
                        stats.add_turn_nr_gt_threshold(1);
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
                        stats.add_turn_nr_gt_threshold(1);
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

    fn get_white_threshold_value(&self, local_map: &DataMap) -> i32 {
        if let Some(white_threshold) = local_map.get_data::<i32>(DataMapKey::WhiteThreshold) {
            *white_threshold
        }
        else {
            panic!("RIP Cant read white threshold");
        }
    }

    fn get_black_threshold_value(&self, local_map: &DataMap) -> i32 {
        if let Some(black_threshold) = local_map.get_data::<i32>(DataMapKey::BlackThreshold) {
            *black_threshold
        }
        else {
            panic!("RIP Cant read white threshold");
        }
    }

    fn get_calc_time(&self, local_map: &DataMap) -> u128 {
        local_map.get_data::<Instant>(DataMapKey::CalcTime)
            .expect(RIP_MISSED_DM_KEY)
            .elapsed()
            .as_millis()
    }
    

}


#[cfg(test)]
mod tests {
    use crate::{config::Config, Stats};
    use crate::service::Service;
    use crate::model::{Board, SearchResult};
    use crate::global_map_handler;

    pub fn search(board: &mut Board, depth: i32, white: bool) -> SearchResult {
        let service = Service::new();
        let config = Config::for_tests();
        let mut stats = Stats::new();
        let mut local_map = global_map_handler::_get_default_local_map();

        let global_map = global_map_handler::create_new_global_map();

        service.search.get_moves(&mut *board, depth, white, &mut stats, &config, &service, &global_map, &mut local_map)
    }
    

    #[test]
    #[ignore]
    fn white_matt_tests() {
        let fen_service = Service::new().fen;
        
        let mut board = fen_service.set_fen("8/3K4/8/8/5RR1/8/k7/8 w - - 0 1");
        let result = search(&mut board, 6, true);
        assert_eq!(result.get_eval(), 32766);
        assert_eq!(result.get_best_move_algebraic(), "f4f3");

        let mut board = fen_service.set_fen("r1q1r1k1/ppppppp1/n1b4p/7N/2B1P2N/2B2Q1P/PPPP1PP1/R3R1K1 w Qq - 0 1");
        let result = search(&mut board, 4, true);
        assert_eq!(result.get_eval(), 32766);
        assert_eq!(result.get_best_move_algebraic(), "f3f7");
        

        let mut board = fen_service.set_fen("6rk/R2R4/7P/8/p1B2P2/2P4P/P5K1/8 w - - 5 39");
        let result = search(&mut board, 6, true);
        assert_eq!(result.get_eval(), 32766);
        assert_eq!(result.get_best_move_algebraic(), "c4g8");
    }


    #[test]
    #[ignore]
    fn black_matt_tests() {
        let fen_service = Service::new().fen;
        
        let mut board = fen_service.set_fen("8/1p6/p1P5/2p5/K1p2P2/P2kPn1P/1r6/8 b - - 3 43");
        let result = search(&mut board, 6, false);
        assert_eq!(result.get_eval(), -32767);
        assert_eq!(result.get_best_move_algebraic(), "b7b6");

        
        let mut board = fen_service.set_fen("8/8/8/2k5/8/5p1r/1K6/8 b - - 0 1");
        let result = search(&mut board, 8, false);
        assert_eq!(result.get_eval(), -32767);
        assert_eq!(result.get_best_move_algebraic(), "f3f2");
        

        let mut board = fen_service.set_fen("8/5pkp/p5p1/4p3/1P3P2/P3P1KP/2q3P1/3r4 b - - 0 37");
        let result = search(&mut board, 6, false);
        assert_eq!(result.get_eval(), -32767);
        assert_eq!(result.get_best_move_algebraic(), "d1g1");
    }


    #[test]
    fn black_find_hit_move() {
        let fen_service = Service::new().fen;
        
        let mut board = fen_service.set_fen("2r2rk1/1b2bppp/pqn1pn2/8/1PBB4/P3PN2/5PPP/RN1Q1RK1 b - - 2 14");
        let result = search(&mut board, 2, false);
        result._print_all_variants();
        assert!(result.get_eval() < -100);
        assert_eq!(result.get_best_move_algebraic(), "c6d4");

        let mut board = fen_service.set_fen("6k1/5pp1/5rnp/2Npb3/3PP3/r1P1R2P/5PP1/4BR1K b - - 0 1");
        let result = search(&mut board, 2, false);
        //result.print_all_variants();
        assert!(result.get_eval() > 0);
        // assert_eq!(result.get_best_move_algebraic(), "e5d4"); // TODO activate

    }


    #[test]
    fn white_find_hit_move() {
        let fen_service = Service::new().fen;

        let mut board = fen_service.set_fen("3r2nk/6pp/3p4/4p3/3BP3/8/3R2PP/6NK w - - 0 1");
        let result = search(&mut board, 2, true);
        //result.print_all_variants();
        assert_eq!(result.get_best_move_algebraic(), "d4e5");
        
        let mut board = fen_service.set_fen("7k/6pp/3p4/4n3/3QP3/8/3R2PP/7K w - - 0 1");
        let result = search(&mut board, 2, true);
        //result.print_all_variants();
        assert_eq!(result.get_best_move_algebraic(), "d4d6"); // d4e5 also matt in 4


        let mut board = fen_service.set_fen("7k/6pp/3p1p2/4r3/p2QP3/8/3R2PP/7K w - - 0 1");
        let result = search(&mut board, 2, true);
        //result.print_all_variants();
        assert_eq!(result.get_best_move_algebraic(), "d4a4");

    }


    #[test]
    #[ignore]
    fn hit_move_unsolved() {
        let fen_service = Service::new().fen;

        let mut board = fen_service.set_fen("4k3/5pp1/2r3np/2Ppp3/3BP3/7P/5PP1/3RR1K1 b - - 0 1");
        let result = search(&mut board, 2, false);
        result._print_all_variants();
        //assert!(result.get_eval() < -100);
        //assert_eq!(result.get_best_move_algebraic(), "d5e4");
    }

    // 
    #[test]
    fn practical_moves_from_games() {
        let fen_service = Service::new().fen;

        let mut board = fen_service.set_fen("rnbqkbnr/1p3ppp/p7/1Np5/1P1p4/5N2/P2PPPPP/R1BQKB1R w KQkq - 0 7");
        let result = search(&mut board, 3, true);
        //assert_eq!( "g4e2", result.get_best_move_algebraic());
        result._print_all_variants();

        /*

        let mut board = fen_service.set_fen("r2q1rk1/1pp2pbp/3p1np1/P1nPp1N1/4P1b1/2N5/P1PBBPPP/R2Q1RK1 b - - 4 11");
        let result = search(&mut board, 2, false);
        assert_eq!( "g4e2", result.get_best_move_algebraic()); // or g4c8 or g4d7
        //result.print_all_variants();

        let mut board = fen_service.set_fen("r1q1k2r/p1pRbp2/5p2/1p5p/5B2/6P1/PPQ1PP1P/4KB1R b Kkq - 0 20");
        let result = search(&mut board, 2, false);
        //result.print_all_variants();
        assert_eq!( "c8d7", result.get_best_move_algebraic());

        let mut board = fen_service.set_fen("7r/p1p2p1p/P3k1p1/2KR1nr1/2P5/8/8/8 w - - 2 35");
        let result = search(&mut board, 2, true);
        assert_ne!("d5e5", result.get_best_move_algebraic());

        let mut board = fen_service.set_fen("r3k2r/pp1n2p1/2p3p1/5pB1/3Pn3/2P3P1/PP2B1PP/R4RK1 w kq - 3 18");
        let result = search(&mut board, 3, true);
        assert_ne!("g5h4", result.get_best_move_algebraic());
         */

    }

}
