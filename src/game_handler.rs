use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

use crate::global_map_handler;
use crate::DataMap;
use crate::DataMapKey;
use crate::Config;
use crate::UciGame;
use crate::Stats;
use crate::QuiescenceSearchMode;
use crate::model::{ThreadSafeDataMap, TimeInfo, TimeMode};
use crate::service::Service;
use crate::Book;
use crate::thread;

use crate::model::SearchResult;

use crate::model::RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE;
use crate::model::RIP_COULDN_LOCK_MUTEX;


pub fn game_loop(global_map: ThreadSafeDataMap, config: &Config, rx_game_command: Receiver<String>) {

    let service = &Service::new();
    let uci_parser = &service.uci_parser;
    let stdout = &service.stdout;
    let mut game = UciGame::new(service.fen.set_init_board());
    let book = Book::new();
    let logger = global_map_handler::get_log_buffer_sender(&global_map);

    let mut local_map = DataMap::new();
    local_map.insert(DataMapKey::CalcTime, Instant::now());

    if config.quiescence_search_mode == QuiescenceSearchMode::Alpha3 {
        local_map.insert(DataMapKey::WhiteThreshold, 0);
        local_map.insert(DataMapKey::BlackThreshold, 0);
    }


    loop {

        match rx_game_command.recv() {
            Ok(command) => {

                if command.trim() == "ucinewgame" {
                    game = UciGame::new(service.fen.set_init_board());
                    global_map_handler::set_stop_flag(&global_map, false);
                    logger.send("Start new Game".to_string()).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                    continue;
                }

                else if command.starts_with("board") {
                    let fen = command[6..].to_string();
                    game = UciGame::new(service.fen.set_fen(&fen));
                }

                else if command.starts_with("moves") {
                    if command.len() <= 5 {
                        continue;
                    }
                    let moves_str = &command[5..];
                    let moves_iter = moves_str.split_whitespace();
                    for mv in moves_iter {
                        game.do_move(mv);
                    }                   
                }

                else if command == "infinite" {
                    let mut local_map = local_map.clone();
                    local_map.insert(DataMapKey::CalcTime, Instant::now());
                    for depth in (2..100).step_by(1) {
                        global_map_handler::get_log_buffer_sender(&global_map)
                            .send(format!("Start Level {}", depth))
                            .expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);

                        let is_white = game.board.white_to_move;
                        let mut stats = Stats::default();
                        let search_result = &service.search.get_moves(&mut game.board, depth, is_white, &mut stats,
                            &config, &service, &global_map, &mut local_map);

                        service.stdout.write(&service.uci_parser.get_info_str(search_result, &stats));
                        
                        if config.use_pv_nodes {
                            global_map_handler::set_pv_nodes(&global_map, &search_result.get_pv_move_row(), &mut game.board);
                            global_map_handler::set_pv_nodes_len(&global_map, search_result.calculated_depth);
                        }
                        if global_map_handler::is_stop_flag(&global_map) { break; }
                    }
                }

                else if command.starts_with("go") {

                    logger.send("incomming go cmd".to_string()).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);

                    global_map_handler::set_stop_flag(&global_map, false);
                    
                    // try to find book move
                    let white = game.white_to_move();        
                    let game_fen = service.fen.get_fen(&game.board);
                    let book_move = book.get_random_book_move(&game_fen);
                    let time_info = uci_parser.parse_go(command.as_str());
                    let time_info_search_thread = time_info.clone(); 

                    let stop_new_search_threads = Arc::new(AtomicBool::new(false));
                    let stop_new_search_threads_2 = stop_new_search_threads.clone();

                    let global_map_handler_time_observer_thread = global_map.clone();
                    let config_time_thread = config.clone();
                    let depth_when_set_stop_new_search_threads = Arc::new(AtomicI32::new(0));
                    let depth_when_set_stop_new_search_threads_2 = depth_when_set_stop_new_search_threads.clone();
        
                    
                    if book_move.is_empty() || !config.use_book {

                        // the timer thread sets stop flags
                        let _time_observer_thread = thread::spawn(move || {
                            let logger = global_map_handler::get_log_buffer_sender(&global_map_handler_time_observer_thread);

                            if time_info.time_mode == TimeMode::None || time_info.time_mode == TimeMode::Depth {
                                logger.send(format!("TimeMode is {:?}", time_info.time_mode)).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                                return;
                            }

                            let my_thinking_time = calculate_thinking_time(&time_info, white, game.board.move_count, &config_time_thread);
                            logger.send(format!("My thinking time is: {}", my_thinking_time)).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);

                            if time_info.time_mode == TimeMode::Movetime {
                                thread::sleep(Duration::from_millis(my_thinking_time));
                                global_map_handler::set_stop_flag(&global_map_handler_time_observer_thread, true);
                                logger.send(format!("Time up. Set stop flag true")).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                            } 
                            else { // in tournament time mode we stop starting threads after thinking time is up
                                thread::sleep(Duration::from_millis(my_thinking_time / 2));
                                stop_new_search_threads.store(true, Ordering::SeqCst);
                                
                                depth_when_set_stop_new_search_threads
                                    .store(global_map_handler::get_max_completed_result_depth(&global_map_handler_time_observer_thread),
                                    Ordering::SeqCst);
                                
                                logger.send(format!("Half time up. Set stop new search depth flag true"))
                                .expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                                
                            }
                        });
        

                        let mut local_map = local_map.clone();
                        local_map.insert(DataMapKey::CalcTime, Instant::now());

                        let results: Arc<Mutex<Vec<SearchResult>>> = Arc::new(Mutex::new(Vec::default()));
                        let depths = Arc::new(Mutex::new((3..=config.max_depth).rev().collect::<Vec<_>>()));
                        let active_threads = Arc::new(AtomicI32::new(0));
                        let calculate_pv = Arc::new(AtomicBool::new(false));
                    
                        let mut handles = vec![];
                    
                        //running Lazy SMP Threads
                        loop {

                            let active_threads = active_threads.clone();
                            let active_threads_2 = active_threads.clone();
                    
                            // start thread if config.max_threads is not reached
                            if (active_threads.load(Ordering::SeqCst) < config.search_threads) &&
                                !global_map_handler::is_stop_flag(&global_map) &&
                                !stop_new_search_threads_2.load(Ordering::SeqCst)
                                {

                                let results = Arc::clone(&results);
                                let depths = Arc::clone(&depths);
                                
                                let global_map = Arc::clone(&global_map);
                                let calculate_pv = calculate_pv.clone();
                                let service = Service::new();
                                let mut game = game.clone();
                                let config = config.clone();                                
                                let mut local_map = local_map.clone();                                
                                let time_info_search_thread = time_info_search_thread.clone();
                    
                                let handle = thread::spawn(move || {

                                    let logger = global_map_handler::get_log_buffer_sender(&global_map);

                                    // handle active Thread counter
                                    let current_active_threads = active_threads.load(Ordering::SeqCst);
                                    active_threads.store(current_active_threads + 1, Ordering::SeqCst);

                                    // depth depends of if we perform a PV search, or a SMP search
                                    let current_depth = if config.use_pv_nodes && calculate_pv.load(Ordering::SeqCst) == false {
                                        local_map.insert(DataMapKey::PvFlag, true);
                                        local_map.insert(DataMapKey::MoveOrderingFlag, true);
                                        calculate_pv.store(true, Ordering::SeqCst);
                                        let mut depth = (global_map_handler::get_pv_nodes_calculated_depth(&global_map) + 1) as i32;
                                        depth = if depth >= 2 { depth } else { 2 };
                                        logger.send(format!("Start new PV search on level {}", depth))
                                            .expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                                        depth
                                    }
                                    else { // for SMP threads
                                        local_map.insert(DataMapKey::PvFlag, false);
                                        if config.use_pv_nodes {
                                            local_map.insert(DataMapKey::MoveOrderingFlag, false);
                                        } else {
                                            local_map.insert(DataMapKey::MoveOrderingFlag, true);
                                        }
                                        let mut depths_guard = depths.lock()
                                            .expect(RIP_COULDN_LOCK_MUTEX);
                                        let depth = depths_guard.pop().expect("RIP reached maximum depth");
                                        logger.send(format!("Start new SMP search on level {}", depth))
                                            .expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                                        depth
                                    };

                
                                    let mut stats = Stats::new();
                                    let white = game.board.white_to_move;
                
                                    // do calculation
                                    let search_result = service.search.get_moves(
                                        &mut game.board,
                                        current_depth,
                                        white,
                                        &mut stats,
                                        &config,
                                        &service,
                                        &global_map,
                                        &mut local_map,
                                    );

                                    logger.send(format!("Finished depth {} move {} completed: {}",
                                        search_result.get_depth(),
                                        search_result.get_best_move_algebraic(),
                                        search_result.completed))
                                        .expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);

                                    if search_result.completed {
                                        if let Err(_e) = service.stdout.write_get_result(&service.uci_parser.get_info_str(&search_result, &stats)) {
                                            logger.send("stdout channel closed during search".to_string())
                                                .expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                                        }

                                        if config.use_pv_nodes {
                                            global_map_handler::set_pv_nodes(&global_map, &search_result.get_pv_move_row(), &mut game.board);
                                            global_map_handler::set_pv_nodes_len(&global_map, search_result.calculated_depth);
                                            if search_result.is_pv_search_result {
                                                calculate_pv.store(false, Ordering::SeqCst);
                                            }
                                        }
                                    }
                
                                    let mut results_guard = results.lock()
                                        .expect(RIP_COULDN_LOCK_MUTEX);
                                    results_guard.push(search_result.clone());

                                    let current_active_threads = active_threads.load(Ordering::SeqCst);
                                    active_threads.store(current_active_threads - 1, Ordering::SeqCst);

                                    // Stop Search if game ends just by checking evaluation gt 32K
                                    if search_result.get_eval().abs() > 32000 {
                                        global_map_handler::set_stop_flag(&global_map, true);
                                        logger.send("found mat. set stop flag".to_string()).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                                    } 

                                    // stop search if in go depth command modus
                                    if search_result.completed &&
                                        time_info_search_thread.time_mode == TimeMode::Depth &&
                                        search_result.calculated_depth >= time_info_search_thread.depth {
                                        global_map_handler::set_stop_flag(&global_map, true);
                                        logger.send("reached max depth. set stop flag".to_string()).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                                    }
                                });
                    
                                handles.push(handle);
                            }
                            
                            if global_map_handler::is_stop_flag(&global_map) { break; }

                            if stop_new_search_threads_2.load(Ordering::SeqCst) {
                                let depth_when_stopped = depth_when_set_stop_new_search_threads_2.load(Ordering::SeqCst);
                                if (depth_when_stopped < global_map_handler::get_max_completed_result_depth(&global_map) as i32) ||
                                    active_threads_2.load(Ordering::SeqCst) == 0 {

                                    global_map_handler::set_stop_flag(&global_map, true);
                                }
                            }
                            thread::sleep(Duration::from_millis(config.game_loop));
                        }
                    
                        // wait until all threads received the stop cmd
                        for handle in handles {
                            handle.join().expect("RIP Thread panicked");

                        }
                        logger.send(format!("Stopped Search Threads")).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                        
                        let results = results.lock()
                            .expect("RIP Couldn lock search result")
                            .clone();

                        let mut results = results
                            .iter()
                            .filter(|r| !r.get_best_move_row().is_empty())
                            .collect::<Vec<_>>();

                        // idx 0 is highest calculated depth
                        results.sort_by(|a, b| b.calculated_depth.cmp(&a.calculated_depth)); 


                        // use the before last calculated result because the last one might not be finished
                        if global_map_handler::is_debug_flag(&global_map) {
                            let _ = results.iter().map(|r| {
                                logger.send(r.get_best_move_row())
                                    .expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                            });
                        }

                        let search_result = results
                            .iter()
                            .find(|r| r.completed)
                            .unwrap_or_else(|| {
                                logger.send(format!("Found no completed search result"))
                                    .expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                                panic!("RIP Found no completed search result");
                        });

                        let uci_string = service.uci_parser.get_info_str(&search_result, &search_result.stats);

                        // send best move uci informations and update internal board
                        if let Err(_e) = stdout.write_get_result(&uci_string) {
                                logger.send("stdout channel closed during search".to_string())
                                    .expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                        }
                        if search_result.get_best_move_row().is_empty() { panic!("RIP Found no move"); }
                        stdout.write(&format!("bestmove {}", search_result.get_best_move_algebraic()));
                        game.do_move(&search_result.get_best_move_algebraic());
                        logger.send(format!("final move: {}", uci_string))
                            .expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
        
                        if config.quiescence_search_mode == QuiescenceSearchMode::Alpha3 {
                            local_map.insert(DataMapKey::WhiteThreshold, search_result.get_eval() as i32);
                            local_map.insert(DataMapKey::BlackThreshold, search_result.get_eval() as i32);
                            logger.send(format!("quiescence_search_threshold: {:?}", local_map.get_data::<i32>(DataMapKey::WhiteThreshold)))
                                .expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                        }
        
                        logger.send(format!("{:?}", search_result.stats)).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                        
                        // Clear global map search results for new search
                        global_map_handler::clear_search_result(&global_map);
                        global_map_handler::clear_pv_nodes(&global_map);
                        

                    } else { // do book move
                        logger.send(format!("found Book move: {} for position {}", book_move, game_fen))
                            .expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                        game.do_move(book_move);
                        stdout.write(&format!("bestmove {}", book_move));
                    }
                }
            }

            Err(_) => {
                panic!("RIP Error reading std input");
            }
        }
    }

}


/// calculates the time {white} is thinking until the stop flag is set
fn calculate_thinking_time(time_info: &TimeInfo, white: bool, move_count: i32, config: &Config) -> u64 {
    let thinking_time = match time_info.time_mode {
        TimeMode::None => 2000,
        
        TimeMode::Movetime => if white { time_info.wtime - 500 } else { time_info.btime - 500 },
        
        TimeMode::MoveToGo => {
            let my_time = if white { time_info.wtime } else { time_info.btime };
            let my_thinking_time = (my_time / (time_info.moves_to_go + 1)) + (if white { time_info.winc } else { time_info.binc });
            
            if my_thinking_time > my_time { // when increment is bigger then current time left
                my_time - 1000
            } else {
                my_thinking_time
            }
        }
        
        TimeMode::HourGlas => {
            let my_time = if white { time_info.wtime } else { time_info.btime };
            
            let my_thinking_time = if move_count < 40 {
                (my_time as f64 * (0.02 + (move_count as f64 / 1000 as f64) as f64)) as i32
            } else {
                my_time / 20
            } + if white { time_info.winc } else { time_info.binc };

            if my_thinking_time > my_time { // when increment is bigger then current time left
                my_time - 1000
            } else {
                my_thinking_time
            }
            
        }
        
        TimeMode::Depth => {
            0
        }
    };

    if (thinking_time as u64) < config.min_thinking_time { config.min_thinking_time } else { thinking_time as u64}
}



#[cfg(test)]
mod tests {
    use crate::model::{TimeInfo, TimeMode};
    use super::calculate_thinking_time;
    use crate::Config;

    #[test]
    fn calculate_thinking_time_test() {

        let config = Config::new();

        let time_info = TimeInfo{
            wtime: 20000, btime: 10000, winc: 0, binc: 0, moves_to_go: 9, time_mode: TimeMode::MoveToGo, depth: 0
        };
        let thinking_time = calculate_thinking_time(&time_info, true, 0, &config);
        assert_eq!(2000, thinking_time);

        let time_info = TimeInfo{
            wtime: 20000, btime: 10000, winc: 0, binc: 0, moves_to_go: 9, time_mode: TimeMode::MoveToGo, depth: 0
        };
        let thinking_time = calculate_thinking_time(&time_info, false, 0, &config);
        assert_eq!(1000, thinking_time);

        let time_info = TimeInfo{
            wtime: 20000, btime: 10000, winc: 0, binc: 0, moves_to_go: 0, time_mode: TimeMode::HourGlas, depth: 0
        };
        let thinking_time = calculate_thinking_time(&time_info, true, 10, &config);
        assert_eq!(600, thinking_time);

        let time_info = TimeInfo{
            wtime: 20000, btime: 10000, winc: 0, binc: 0, moves_to_go: 0, time_mode: TimeMode::HourGlas, depth: 0
        };
        let thinking_time = calculate_thinking_time(&time_info, false, 20, &config);
        assert_eq!(400, thinking_time);

    }

}