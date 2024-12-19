use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

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


                    // the timer thread sets stop flags
                    let _time_observer_thread = thread::spawn(move || {
                        let logger = global_map_handler::get_log_buffer_sender(&global_map_handler_time_observer_thread);

                        if time_info.time_mode == TimeMode::None || time_info.time_mode == TimeMode::Depth {
                            logger.send(format!("TimeMode is {:?}", time_info.time_mode)).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                            return;
                        }

                        let my_thinking_time = calculate_thinking_time(&time_info, white, game.board.move_count);
                        logger.send(format!("My thinking time is: {}", my_thinking_time)).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);

                        if time_info.time_mode == TimeMode::Movetime {
                            thread::sleep(Duration::from_millis(my_thinking_time));
                        } else {
                            thread::sleep(Duration::from_millis(my_thinking_time / 2));
                            stop_new_search_threads.store(true, Ordering::SeqCst);
                            logger.send(format!("Set stop new search depth flag true")).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                            thread::sleep(Duration::from_millis(my_thinking_time / 2));
                        }
                        
                        logger.send(format!("Set stop flag true")).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                        global_map_handler::set_stop_flag(&global_map_handler_time_observer_thread, true);
                    });
        
                    
                    if book_move.is_empty() || !config.use_book {
        
                        let mut local_map = local_map.clone();
                        local_map.insert(DataMapKey::CalcTime, Instant::now());

                        let results: Arc<Mutex<Vec<SearchResult>>> = Arc::new(Mutex::new(Vec::default()));
                        let depths = Arc::new(Mutex::new((2..=config.max_depth).rev().collect::<Vec<_>>()));
                        let active_threads = Arc::new(Mutex::new(0));
                        let calculate_pv = Arc::new(AtomicBool::new(false));
                    
                        let mut handles = vec![];
                    
                        //running Lazy SMP Threads
                        loop {
                            let active_count = {
                                let active_threads_guard = active_threads.lock()
                                    .expect(RIP_COULDN_LOCK_MUTEX);
                                *active_threads_guard
                            };
                    
                            // start thread if config.max_threads is not reached
                            if (active_count < config.search_threads) && !global_map_handler::is_stop_flag(&global_map) {
                                let results = Arc::clone(&results);
                                let depths = Arc::clone(&depths);
                                let active_threads = Arc::clone(&active_threads);
                                let service = Service::new();
                                let mut game = game.clone();
                                let config = config.clone();
                                let global_map = global_map.clone();
                                let mut local_map = local_map.clone();
                                let calculate_pv = calculate_pv.clone();
                                let time_info_search_thread = time_info_search_thread.clone();
                    
                                let handle = thread::spawn(move || {

                                    let logger = global_map_handler::get_log_buffer_sender(&global_map);

                                    {
                                        let mut active_threads_guard = active_threads.lock()
                                            .expect(RIP_COULDN_LOCK_MUTEX);
                                        *active_threads_guard += 1;
                                    }

                                    // depth depends of if we perform a PV search, or a SMP search
                                    let current_depth = if config.use_pv_nodes && calculate_pv.load(Ordering::SeqCst) == false {
                                        local_map.insert(DataMapKey::PvFlag, true);
                                        calculate_pv.store(true, Ordering::SeqCst);
                                        let depth = (global_map_handler::get_pv_nodes_len(&global_map) + 1) as i32;
                                        logger.send(format!("Start new PV search on level {}", depth))
                                            .expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                                        if depth >= 2 { depth } else { 2 }
                                    } else {
                                        local_map.insert(DataMapKey::PvFlag, false);
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

                                    if search_result.completed {
                                        if let Err(_e) = service.stdout.write_get_result(&service.uci_parser.get_info_str(&search_result, &stats)) {
                                            logger.send("stdout channel closed during search".to_string())
                                                .expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                                        }

                                        if config.use_pv_nodes {
                                            global_map_handler::set_pv_nodes(&global_map, &search_result.get_pv_move_row(), &mut game.board);
                                            if search_result.is_pv_search_result {
                                                calculate_pv.store(false, Ordering::SeqCst);
                                            }
                                        }
                                    }
                
                                    let mut results_guard = results.lock()
                                        .expect(RIP_COULDN_LOCK_MUTEX);
                                    results_guard.push(search_result.clone());
                    
                                    {
                                        let mut active_threads_guard = active_threads.lock()
                                            .expect(RIP_COULDN_LOCK_MUTEX);
                                        *active_threads_guard -= 1;
                                    }

                                    // stop search if in go depth command modus
                                    if search_result.completed &&
                                        time_info_search_thread.time_mode == TimeMode::Depth &&
                                        search_result.calculated_depth >= time_info_search_thread.depth {
                                        global_map_handler::set_stop_flag(&global_map, true);
                                    }
                                });
                    
                                handles.push(handle);
                            }
                            
                            if global_map_handler::is_stop_flag(&global_map) { break; }
                            if stop_new_search_threads_2.load(Ordering::SeqCst) {
                                global_map_handler::set_stop_flag(&global_map, true);
                                break;
                            }
                            thread::sleep(Duration::from_millis(10));
                        }
                    
                        // wait until all threads received the stop cmd
                        for handle in handles {
                            handle.join().expect("RIP Thread panicked");
                        }
                        logger.send(format!("Stopped Search Threads")).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                        global_map_handler::clear_pv_nodes(&global_map);
                        
                        let results = results.lock()
                            .expect("RIP Couldn lock search result")
                            .clone();

                        let mut results = results
                            .iter()
                            .filter(|r| !r.get_best_move_row().is_empty())
                            .collect::<Vec<_>>();

                        results.sort_by(|a, b| b.calculated_depth.cmp(&a.calculated_depth)); // 0 is highest depth


                        // use the before last calculated result because the last one is not finished
                        // TODO check if unfinished result is better, then use it
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


                        // send best move uci informations and update internal board
                        if let Err(_e) = stdout.write_get_result(
                            &service.uci_parser.get_info_str(&search_result, &search_result.stats)) {
                                logger.send("stdout channel closed during search".to_string())
                                    .expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                        }
                        if search_result.get_best_move_row().is_empty() { panic!("RIP Found no move"); }
                        stdout.write(&format!("bestmove {}", search_result.get_best_move_algebraic()));
                        game.do_move(&search_result.get_best_move_algebraic());                        
        
                        if config.quiescence_search_mode == QuiescenceSearchMode::Alpha3 {
                            local_map.insert(DataMapKey::WhiteThreshold, search_result.get_eval() as i32);
                            local_map.insert(DataMapKey::BlackThreshold, search_result.get_eval() as i32);
                            logger.send(format!("quiescence_search_threshold: {:?}", local_map.get_data::<i32>(DataMapKey::WhiteThreshold)))
                                .expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                        }
        
                        if config.in_debug {
                            logger.send(format!("{:?}", search_result.stats)).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                        }

                    } else { // do book move
                        if config.in_debug {    
                            logger.send(format!("found Book move: {} for position {}", book_move, game_fen))
                                .expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                        }
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
fn calculate_thinking_time(time_info: &TimeInfo, white: bool, move_count: i32) -> u64 {
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
    if thinking_time < 100 { 100 } else { thinking_time as u64}
}



#[cfg(test)]
mod tests {
    use crate::model::{TimeInfo, TimeMode};
    use super::calculate_thinking_time;

    #[test]
    fn calculate_thinking_time_test() {
        let time_info = TimeInfo{
            wtime: 20000, btime: 10000, winc: 0, binc: 0, moves_to_go: 9, time_mode: TimeMode::MoveToGo, depth: 0
        };
        let thinking_time = calculate_thinking_time(&time_info, true, 0);
        assert_eq!(2000, thinking_time);

        let time_info = TimeInfo{
            wtime: 20000, btime: 10000, winc: 0, binc: 0, moves_to_go: 9, time_mode: TimeMode::MoveToGo, depth: 0
        };
        let thinking_time = calculate_thinking_time(&time_info, false, 0);
        assert_eq!(1000, thinking_time);

        let time_info = TimeInfo{
            wtime: 20000, btime: 10000, winc: 0, binc: 0, moves_to_go: 0, time_mode: TimeMode::HourGlas, depth: 0
        };
        let thinking_time = calculate_thinking_time(&time_info, true, 10);
        assert_eq!(600, thinking_time);

        let time_info = TimeInfo{
            wtime: 20000, btime: 10000, winc: 0, binc: 0, moves_to_go: 0, time_mode: TimeMode::HourGlas, depth: 0
        };
        let thinking_time = calculate_thinking_time(&time_info, false, 20);
        assert_eq!(400, thinking_time);

    }

}