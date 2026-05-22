use std::collections::VecDeque;
use std::time::Instant;
use std::sync::Arc;

use crate::config::Config;
use crate::model::{Board, DataMap, DataMapKey, GameStatus, QuiescenceSearchMode, SearchResult, Stats, Turn, Variant, SearchContext, EngineState, RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE};
use crate::service::Service;

use crate::model::RIP_MISSED_DM_KEY;


pub struct SearchService;

impl SearchService {

    pub fn new() -> Self {
        SearchService
    }

    pub fn get_moves(
        &self,
        board: &mut Board,
        depth: i32,
        white: bool,
        stats: &mut Stats,
        config: &Config,
        service: &Service,
        engine_state: &Arc<EngineState>,
        local_map: &mut DataMap,
    ) -> SearchResult {
        let logger = engine_state.log_sender.clone();

        let mut best_eval = if white { i16::MIN } else { i16::MAX };

        let zobrist_table = &engine_state.zobrist_table;
        let stop_flag = &engine_state.stop_flag;
        let pv_nodes = &engine_state.pv_nodes;

        let context = SearchContext {
            zobrist_table,
            stop_flag,
            pv_nodes,
        };

        let turns = service.move_gen.generate_valid_moves_list(board, stats, config, &context, local_map);
        let mut search_result: SearchResult = SearchResult::default();
        search_result.calculated_depth = depth;
        search_result.is_white_move = white;
        search_result.is_pv_search_result = *local_map.get_data::<bool>(DataMapKey::PvFlag).unwrap_or_else(|| &false);

        let mut alpha: i16 = i16::MIN;
        let mut beta: i16 = i16::MAX;

        let mut turn_counter = 0;
        let mut child_pv = [None; 128];

        for turn in &turns {
            turn_counter += 1;
            let mi = board.do_move(turn);
            let min_max_result = self.minimax(board, turn, depth - 1, !white,
                alpha, beta, stats, config, service, &context, local_map, &mut child_pv);

            if stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                board.undo_move(turn, mi);
                let calc_time_ms = self.get_calc_time(local_map);
                search_result.stats = stats.clone();
                search_result.stats.best_turn_nr = turn_counter;
                search_result.stats.calc_time_ms = calc_time_ms as usize;
                break;
            }

            let min_max_eval = min_max_result.1;

            // save min max eval in zobrist table for better move sorting, if depth = 2
            if depth == 2 && config.use_zobrist {
                zobrist_table.insert_entry(board.cached_hash, crate::zobrist::TranspositionEntry {
                    eval: min_max_eval,
                    depth: 1,
                    entry_type: crate::zobrist::TranspositionType::Exact,
                    best_move: min_max_result.0,
                });
            }

            board.undo_move(turn, mi);
            if white {
                if min_max_eval > best_eval {
                    best_eval = min_max_eval;
                    alpha = min_max_eval;
                    let mut best_move_row = VecDeque::new();
                    best_move_row.push_back(Some(*turn));
                    for mv in child_pv.iter().take_while(|x| x.is_some()) {
                        best_move_row.push_back(*mv);
                    }
                    search_result.add_variant(Variant { best_move: Some(*turn), move_row: best_move_row, eval: min_max_eval });
                    search_result.is_white_move = white;
                    search_result.variants.sort_by(|a, b| b.eval.cmp(&a.eval)); // Highest first for white
                    stats.best_turn_nr = turn_counter;
                    let calc_time_ms = self.get_calc_time(local_map);
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
                    let mut best_move_row = VecDeque::new();
                    best_move_row.push_back(Some(*turn));
                    for mv in child_pv.iter().take_while(|x| x.is_some()) {
                        best_move_row.push_back(*mv);
                    }
                    search_result.add_variant(Variant { best_move: Some(*turn), move_row: best_move_row, eval: min_max_eval });
                    search_result.is_white_move = white;
                    search_result.variants.sort_by(|a, b| a.eval.cmp(&b.eval)); // Lowest first for black
                    search_result.stats.best_turn_nr = turn_counter;
                    let calc_time_ms = self.get_calc_time(local_map);
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
        
        let calc_time_ms = self.get_calc_time(local_map);
        search_result.stats = stats.clone();
        search_result.stats.calc_time_ms = calc_time_ms as usize;
        if stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
            search_result.completed = false;
        } else {
            search_result.completed = true;
        }
        search_result
    }
    

    fn minimax(&self, board: &mut Board, turn: &Turn, depth: i32, white: bool,
        mut alpha: i16, mut beta: i16, stats: &mut Stats, config: &Config, service: &Service,
        context: &SearchContext, local_map: &mut DataMap, pv: &mut [Option<Turn>; 128])
        -> (Option<Turn>, i16) {

        for slot in pv.iter_mut() {
            *slot = None;
        }

        let mut turns: Vec<Turn> = Default::default();

        let orig_alpha = alpha;
        let orig_beta = beta;

        if depth > 0 && config.use_zobrist {
            if board.cached_hash == 0 {
                board.cached_hash = crate::zobrist::gen(board);
            }
            if let Some(entry) = context.zobrist_table.get_entry(&board.cached_hash) {
                if entry.depth >= depth {
                    match entry.entry_type {
                        crate::zobrist::TranspositionType::Exact => {
                            if let Some(m) = entry.best_move {
                                pv[0] = Some(m);
                            }
                            return (entry.best_move, entry.eval);
                        }
                        crate::zobrist::TranspositionType::LowerBound => {
                            alpha = alpha.max(entry.eval);
                            if alpha >= beta {
                                if let Some(m) = entry.best_move {
                                    pv[0] = Some(m);
                                }
                                return (entry.best_move, entry.eval);
                            }
                        }
                        crate::zobrist::TranspositionType::UpperBound => {
                            beta = beta.min(entry.eval);
                            if alpha >= beta {
                                if let Some(m) = entry.best_move {
                                    pv[0] = Some(m);
                                }
                                return (entry.best_move, entry.eval);
                            }
                        }
                    }
                }
            }
        }

        if depth <= 0 {
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
            
            let eval = service.eval.calc_eval(board, config, &service.move_gen, local_map);
            if config.use_zobrist {
                context.zobrist_table.insert_entry(board.cached_hash, crate::zobrist::TranspositionEntry {
                    eval,
                    depth: 0,
                    entry_type: crate::zobrist::TranspositionType::Exact,
                    best_move: None,
                });
            }

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
                    self.get_white_threshold_value(local_map) < eval as i32 || turn.capture == 0
                } else {
                    self.get_black_threshold_value(local_map) > eval as i32 || turn.capture == 0
                };
            }
            

            if stand_pat_cut && turns.is_empty(){
                // check for mate or draw or leave quitesearch
                if service.move_gen.generate_valid_moves_list(board, stats, config, context, local_map).is_empty() {
                    return match board.game_status {
                        GameStatus::WhiteWin => (None, i16::MAX - 1),
                        GameStatus::BlackWin => (None, i16::MIN + 1),
                        GameStatus::Draw => (None, 0),
                        _ => panic!("RIP no defined game end"),
                    };
                }
                return (None, eval);
            } else {
                turns = service.move_gen.generate_valid_moves_list_capture(board, stats, config, context, local_map);
                if turns.is_empty() {
                    // check for mate or draw
                    if service.move_gen.generate_valid_moves_list(board, stats, config, context, local_map).is_empty() {
                        return match board.game_status {
                            GameStatus::WhiteWin => (None, i16::MAX - 1),
                            GameStatus::BlackWin => (None, i16::MIN + 1),
                            GameStatus::Draw => (None, 0),
                            _ => panic!("RIP no defined game end"),
                        };
                    }
                    return (None, eval);
                }
            }
        } else {
            local_map.insert(DataMapKey::ForceSkipValidationFlag, config.skip_strong_validation);
            turns = service.move_gen.generate_valid_moves_list(board, stats, config, context, local_map);
        }

        let mut eval = if white { i16::MIN } else { i16::MAX };
        let mut best_move: Option<Turn> = None;

        if turns.len() == 0 || board.game_status != GameStatus::Normal {
            return match board.game_status {
                GameStatus::WhiteWin => (None, i16::MAX - 1),
                GameStatus::BlackWin => (None, i16::MIN + 1),
                GameStatus::Draw => (None, 0),
                _ => panic!("RIP no defined game end"),
            };
        }

        let mut turn_counter = 0;
        let mut child_pv = [None; 128];

        for turn in &turns {
            if context.stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            turn_counter += 1;
            stats.add_calculated_nodes(1);
            let mi = board.do_move(turn);
            let min_max_result = self.minimax(board, turn, depth - 1, !white,
                alpha, beta, stats, config, service, context, local_map, &mut child_pv);
            let min_max_eval = min_max_result.1;
            board.undo_move(turn, mi);

            if white {
                if eval < min_max_eval {
                    eval = min_max_eval;
                    alpha = min_max_eval;
                    best_move = Some(*turn);
                    pv[0] = Some(*turn);
                    pv[1..].copy_from_slice(&child_pv[..127]);
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
                    best_move = Some(*turn);
                    pv[0] = Some(*turn);
                    pv[1..].copy_from_slice(&child_pv[..127]);
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

        if config.use_zobrist && !context.stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
            let entry_type = if eval <= orig_alpha {
                crate::zobrist::TranspositionType::UpperBound
            } else if eval >= orig_beta {
                crate::zobrist::TranspositionType::LowerBound
            } else {
                crate::zobrist::TranspositionType::Exact
            };

            context.zobrist_table.insert_entry(
                board.cached_hash,
                crate::zobrist::TranspositionEntry {
                    eval,
                    depth,
                    entry_type,
                    best_move,
                },
            );
        }

        return (best_move, eval);
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
    use crate::config::Config;
    use crate::service::Service;
    use crate::model::{Board, SearchResult, EngineState, DataMap, DataMapKey, Stats, SearchContext, Turn};
    use crate::zobrist::ZobristTable;
    use std::sync::Arc;
    use std::time::Instant;

    pub fn search(board: &mut Board, depth: i32, white: bool) -> SearchResult {
        let service = Service::new();
        let config = Config::for_tests();
        let mut stats = Stats::new();
        let mut local_map = DataMap::new();
        local_map.insert(DataMapKey::CalcTime, std::time::Instant::now());
        local_map.insert(DataMapKey::WhiteGivesCheck, false);
        local_map.insert(DataMapKey::BlackGivesCheck, false);

        let (tx_log, _rx_log) = std::sync::mpsc::channel();
        let engine_state = Arc::new(EngineState {
            stop_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            debug_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            zobrist_table: Arc::new(ZobristTable::new()),
            pv_nodes: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            pv_nodes_len: Arc::new(std::sync::atomic::AtomicI32::new(0)),
            logger: Arc::new(std::sync::RwLock::new(Arc::new(|_| {}))),
            log_sender: tx_log,
        });

        service.search.get_moves(&mut *board, depth, white, &mut stats, &config, &service, &engine_state, &mut local_map)
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
        assert!(result.get_eval() > 0);
    }


    #[test]
    fn white_find_hit_move() {
        let fen_service = Service::new().fen;

        let mut board = fen_service.set_fen("3r2nk/6pp/3p4/4p3/3BP3/8/3R2PP/6NK w - - 0 1");
        let result = search(&mut board, 2, true);
        assert_eq!(result.get_best_move_algebraic(), "d4e5");
        
        let mut board = fen_service.set_fen("7k/6pp/3p4/4n3/3QP3/8/3R2PP/7K w - - 0 1");
        let result = search(&mut board, 2, true);
        assert_eq!(result.get_best_move_algebraic(), "d4d6");


        let mut board = fen_service.set_fen("7k/6pp/3p1p2/4r3/p2QP3/8/3R2PP/7K w - - 0 1");
        let result = search(&mut board, 2, true);
        assert_eq!(result.get_best_move_algebraic(), "d4a4");
    }


    #[test]
    #[ignore]
    fn hit_move_unsolved() {
        let fen_service = Service::new().fen;

        let mut board = fen_service.set_fen("4k3/5pp1/2r3np/2Ppp3/3BP3/7P/5PP1/3RR1K1 b - - 0 1");
        let result = search(&mut board, 2, false);
        result._print_all_variants();
    }

    
    #[test]
    fn practical_moves_from_games() {
        let fen_service = Service::new().fen;

        let mut board = fen_service.set_fen("rnbqkbnr/1p3ppp/p7/1Np5/1P1p4/5N2/P2PPPPP/R1BQKB1R w KQkq - 0 7");
        let result = search(&mut board, 3, true);
        result._print_all_variants();
    }

    #[test]
    fn stop_flag_termination_test() {
        use std::sync::atomic::Ordering;
        use std::thread;
        use std::time::Duration;

        let (tx_log, _rx_log) = std::sync::mpsc::channel();
        let engine_state = Arc::new(EngineState {
            stop_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            debug_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            zobrist_table: Arc::new(ZobristTable::new()),
            pv_nodes: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            pv_nodes_len: Arc::new(std::sync::atomic::AtomicI32::new(0)),
            logger: Arc::new(std::sync::RwLock::new(Arc::new(|_| {}))),
            log_sender: tx_log,
        });
        let stop_flag = engine_state.stop_flag.clone();

        let engine_state_clone = engine_state.clone();
        let handle = thread::spawn(move || {
            let service = Service::new();
            let mut board = service.fen.set_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
            let mut stats = Stats::new();
            let config = Config::for_tests();
            let mut local_map = DataMap::new();
            local_map.insert(DataMapKey::CalcTime, Instant::now());
            local_map.insert(DataMapKey::WhiteGivesCheck, false);
            local_map.insert(DataMapKey::BlackGivesCheck, false);

            service.search.get_moves(
                &mut board,
                8,
                true,
                &mut stats,
                &config,
                &service,
                &engine_state_clone,
                &mut local_map,
            )
        });

        thread::sleep(Duration::from_millis(15));
        stop_flag.store(true, Ordering::Relaxed);

        let start_wait = std::time::Instant::now();
        let search_result = handle.join().expect("Search thread panicked");
        let duration = start_wait.elapsed();

        assert!(duration < Duration::from_millis(150), "Search took too long to terminate: {:?}", duration);
        assert!(!search_result.completed, "Search should be marked as incomplete");
    }

    #[test]
    fn zobrist_table_concurrent_stress_test() {
        use std::thread;

        let zobrist_table = Arc::new(ZobristTable::new());

        let mut handles = vec![];
        let num_threads = 4;
        let elements_per_thread = 500;

        for t in 0..num_threads {
            let table = zobrist_table.clone();
            let handle = thread::spawn(move || {
                for i in 0..elements_per_thread {
                    let hash_key = (t as u64 * 1_000_000) + i as u64;
                    let eval_val = (i % 30000) as i16;
                    let entry = crate::zobrist::TranspositionEntry {
                        eval: eval_val,
                        depth: 4,
                        entry_type: crate::zobrist::TranspositionType::Exact,
                        best_move: None,
                    };
                    table.insert_entry(hash_key, entry);
                    let read_val = table.get_eval_for_hash(&hash_key);
                    assert_eq!(read_val, Some(eval_val));
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().expect("Stress thread panicked");
        }

        assert_eq!(zobrist_table.hash_map.len(), num_threads * elements_per_thread);
    }

    #[test]
    fn thread_counter_integrity_test() {
        use std::sync::atomic::{AtomicI32, Ordering};
        use std::thread;
        use std::sync::Arc;

        let counter = Arc::new(AtomicI32::new(0));
        let num_threads = 10;
        let mut spawn_handles = vec![];

        for _ in 0..num_threads {
            counter.fetch_add(1, Ordering::Relaxed);
            let counter_clone = counter.clone();

            let handle = thread::spawn(move || {
                thread::sleep(std::time::Duration::from_millis(5));
                counter_clone.fetch_sub(1, Ordering::Relaxed);
            });
            spawn_handles.push(handle);
        }

        for handle in spawn_handles {
            handle.join().expect("Spawned test thread panicked");
        }

        assert_eq!(counter.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn zobrist_transposition_table_cutoff_test() {
        let service = Service::new();
        let mut board = service.fen.set_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        
        let mut stats = Stats::new();
        let mut config = Config::for_tests();
        config.use_zobrist = true;

        let mut local_map = DataMap::new();
        local_map.insert(DataMapKey::CalcTime, Instant::now());
        local_map.insert(DataMapKey::WhiteGivesCheck, false);
        local_map.insert(DataMapKey::BlackGivesCheck, false);

        let table = Arc::new(ZobristTable::new());
        let (tx_log, _rx_log) = std::sync::mpsc::channel();
        let engine_state = Arc::new(EngineState {
            stop_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            debug_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            zobrist_table: table.clone(),
            pv_nodes: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            pv_nodes_len: Arc::new(std::sync::atomic::AtomicI32::new(0)),
            logger: Arc::new(std::sync::RwLock::new(Arc::new(|_| {}))),
            log_sender: tx_log,
        });

        let context = SearchContext {
            zobrist_table: &engine_state.zobrist_table,
            stop_flag: &engine_state.stop_flag,
            pv_nodes: &engine_state.pv_nodes,
        };

        // 1. Insert an Exact transposition entry
        board.cached_hash = crate::zobrist::gen(&board);
        let test_hash = board.cached_hash;
        table.insert_entry(test_hash, crate::zobrist::TranspositionEntry {
            eval: 500,
            depth: 3,
            entry_type: crate::zobrist::TranspositionType::Exact,
            best_move: None,
        });

        // Search depth 3. It should trigger an immediate exact cutoff and return 500.
        let result = service.search.minimax(
            &mut board,
            &Turn::new(0, 0, 0, 0, false, 0),
            3,
            true,
            i16::MIN,
            i16::MAX,
            &mut stats,
            &config,
            &service,
            &context,
            &mut local_map,
            &mut [None; 128],
        );

        assert_eq!(result.1, 500);
        assert_eq!(stats.calculated_nodes, 0); // No nodes calculated because of TT cutoff!

        // 2. LowerBound cutoff verification
        stats = Stats::new();
        table.insert_entry(test_hash, crate::zobrist::TranspositionEntry {
            eval: 600,
            depth: 4,
            entry_type: crate::zobrist::TranspositionType::LowerBound,
            best_move: None,
        });

        // Search depth 4, alpha = 200, beta = 500. Since eval (600) >= beta (500), it should cause a beta cutoff.
        let result_lower = service.search.minimax(
            &mut board,
            &Turn::new(0, 0, 0, 0, false, 0),
            4,
            true,
            200,
            500,
            &mut stats,
            &config,
            &service,
            &context,
            &mut local_map,
            &mut [None; 128],
        );
        assert_eq!(result_lower.1, 600);
        assert_eq!(stats.calculated_nodes, 0);

        // 3. UpperBound cutoff verification
        stats = Stats::new();
        table.insert_entry(test_hash, crate::zobrist::TranspositionEntry {
            eval: 100,
            depth: 2,
            entry_type: crate::zobrist::TranspositionType::UpperBound,
            best_move: None,
        });

        // Search depth 2, alpha = 300, beta = 700. Since eval (100) <= alpha (300), it should cause an alpha cutoff.
        let result_upper = service.search.minimax(
            &mut board,
            &Turn::new(0, 0, 0, 0, false, 0),
            2,
            true,
            300,
            700,
            &mut stats,
            &config,
            &service,
            &context,
            &mut local_map,
            &mut [None; 128],
        );
        assert_eq!(result_upper.1, 100);
        assert_eq!(stats.calculated_nodes, 0);
    }

}
