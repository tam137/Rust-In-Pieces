use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};

use crate::global_map_handler;
use crate::DataMap;
use crate::DataMapKey;
use crate::Config;
use crate::UciGame;
use crate::Stats;
use crate::QuiescenceSearchMode;
use crate::model::ThreadSafeDataMap;
use crate::service::Service;
use crate::Book;
use crate::thread;

use crate::model::SearchResult;

use crate::model::RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE;


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
                        let _r = &service.search.get_moves(&mut game.board, depth, is_white, &mut stats,
                            &config, &service, &global_map, &mut local_map);

                        if global_map_handler::is_stop_flag(&global_map) { break; }
                    }
                }

                else if command.starts_with("go") {

                    global_map_handler::set_stop_flag(&global_map, false);
                    
                    let white = game.white_to_move();        
                    let game_fen = service.fen.get_fen(&game.board);
                    let book_move = book.get_random_book_move(&game_fen);                    
        
                    let (wtime, btime): (i32, i32) = uci_parser.parse_go(command.as_str());
                    let my_time_ms = if white { wtime } else { btime };

                    let global_map_handler_time_observer_thread = global_map.clone();

                    let _time_observer_thread = thread::spawn(move || {
                        let logger = global_map_handler::get_log_buffer_sender(&global_map_handler_time_observer_thread);
                        let my_thinking_time = my_time_ms / 20;
                        logger.send(format!("My thinking time is: {}", my_thinking_time)).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                        thread::sleep(Duration::from_millis((my_thinking_time) as u64));
                        global_map_handler::set_stop_flag(&global_map_handler_time_observer_thread, true);
                    });
        
                    if book_move.is_empty() || !config.use_book {
        
                        let mut local_map = local_map.clone();
                        local_map.insert(DataMapKey::CalcTime, Instant::now());

                        let results: Arc<Mutex<Vec<SearchResult>>> = Arc::new(Mutex::new(Vec::default()));
                        let max_depth = config.max_depth;
                        let depths = Arc::new(Mutex::new((2..=max_depth).rev().collect::<Vec<_>>()));
                        let active_threads = Arc::new(Mutex::new(0));
                    
                        let mut handles = vec![];
                    
                        loop {
                            let active_count = {
                                let active_threads_guard = active_threads.lock()
                                    .expect("RIP Could not lock active_threads mutex");
                                *active_threads_guard
                            };
                    
                            // start thread if config.max_threads is not reached
                            if active_count < config.search_threads {
                                let results = Arc::clone(&results);
                                let depths = Arc::clone(&depths);
                                let active_threads = Arc::clone(&active_threads);
                                let service = Service::new();
                                let mut game = game.clone();
                                let config = config.clone();
                                let global_map = global_map.clone();
                                let mut local_map = local_map.clone();
                    
                                let handle = thread::spawn(move || {
                                    {
                                        let mut active_threads_guard = active_threads.lock()
                                            .expect("RIP Could not lock active_threads mutex");
                                        *active_threads_guard += 1;
                                    }
                    
                                    loop {
                                        let current_depth = {
                                            let mut depths_guard = depths.lock()
                                                .expect("RIP Could not lock depths mutex");
                                            if let Some(depth) = depths_guard.pop() {
                                                depth
                                            } else {
                                                break; // reached config.max_depth
                                            }
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
                    
                                        let mut results_guard = results.lock()
                                            .expect("RIP Could not lock results mutex");
                                        results_guard.push(search_result);
                                    }
                    
                                    {
                                        let mut active_threads_guard = active_threads.lock()
                                            .expect("RIP Could not lock active_threads mutex");
                                        *active_threads_guard -= 1;
                                    }
                                });
                    
                                handles.push(handle);
                            }
                    
                            // TODO termination condition revise it
                            if {
                                let depths_guard = depths.lock().expect("RIP Could not lock depths mutex");
                                depths_guard.is_empty()
                            } && {
                                let active_threads_guard = active_threads.lock()
                                    .expect("RIP Could not lock active_threads mutex");
                                *active_threads_guard == 0
                            } {
                                break;
                            }
                            thread::sleep(Duration::from_millis(10));
                        }
                    
                        // wait until all threads received the stop cmd
                        for handle in handles {
                            handle.join().expect("RIP Thread panicked");
                        }
                        
                        let mut results = results.lock()
                            .expect("RIP Couldn lock search result")
                            .clone();

                        results.sort_by(|a, b| b.get_depth().cmp(&a.get_depth())); // 0 is highest depth


                        // use the before last calculated result because the last one is not finished
                        // TODO check if unfinished result is better, then use it
                        let search_result = results.get(1).or_else(|| results.get(0))
                            .expect("RIP Found no search result");

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


fn _calculate_depth(config: &Config, complexity: i32, benchmark: i32, time: i32, global_map: &ThreadSafeDataMap) -> i32 {
    let time_in_sec = (time / 1000) + 1;
    let value = time_in_sec * benchmark / (complexity + 1);
    let logger;
    {
        let global_map_value = global_map.read().expect("RIP Could not write data map");
        logger = global_map_value.get_data::<Arc<dyn Fn(String) + Send + Sync>>(DataMapKey::Logger)
            .expect("RIP Could not get logger from data map").clone();
    }

    if value > 500 {
        if config.in_debug {
            logger(format!("time threshold: {} -> depth: {}", value, 10));
        }        
        return 10;
    } else if value > 200 {
        if config.in_debug {
            logger(format!("time threshold: {} -> depth: {}", value, 8));
        }        
        return 8;
    } else if value > 120 {
        if config.in_debug {
            logger(format!("time threshold: {} -> depth: {}", value, 6));
        }        
        return 6;
    } else if value >= 30 {
        if config.in_debug {
            logger(format!("time threshold: {} -> depth: {}", value, 4));
        }
        return 4;
    } else {
        if config.in_debug {
            logger(format!("time threshold: {} -> depth: {}", value, 2));
        }
        return 2;
    }
}