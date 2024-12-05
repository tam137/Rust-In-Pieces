use std::sync::mpsc::Receiver;
use std::time::Instant;
use std::sync::Arc;

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

use crate::model::RIP_COULDN_LOCK_STOP_FLAG;
use crate::model::RIP_MISSED_DM_KEY;
use crate::model::RIP_COULDN_LOCK_ZOBRIST;
use crate::model::RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE;


pub fn game_loop(global_map: ThreadSafeDataMap, config: &Config, rx_game_command: Receiver<String>) {

    let service = &Service::new();
    let uci_parser = &service.uci_parser;
    let stdout = &service.stdout;
    let mut stats= Stats::new();
    let mut game = UciGame::new(service.fen.set_init_board());
    let stop_flag = global_map_handler::get_stop_flag(&global_map);
    let zobrist_table_mutex = global_map_handler::get_zobrist_table(&global_map);
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
                    stats = Stats::new();
                    game = UciGame::new(service.fen.set_init_board());
                    let mut stop_flag_value = stop_flag.lock().expect(RIP_COULDN_LOCK_STOP_FLAG);
                    *stop_flag_value = false;
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
                        let _r = &service.search.get_moves(&mut game.board, depth, is_white, &mut stats,
                            &config, &service, &global_map, &mut local_map);

                        if global_map_handler::is_stop_flag(&global_map) { break; }
                    }
                }

                else if command.starts_with("go") {
                    
                    let white = game.white_to_move();        
                    let game_fen = service.fen.get_fen(&game.board);
                    let book_move = book.get_random_book_move(&game_fen);
                    
        
                    let (wtime, btime): (i32, i32) = uci_parser.parse_go(command.as_str());
        
                    if book_move.is_empty() {
        
                        let _my_time_ms = if white { wtime } else { btime };
                        let calculated_depth = config.search_depth;
        
        
                        let mut local_map = local_map.clone();
                        local_map.insert(DataMapKey::CalcTime, Instant::now());
        
                        let search_result = service.search.get_moves(&mut game.board, calculated_depth, white, &mut stats,
                            config, service, &global_map, &mut local_map);
        
        
                        if global_map_handler::is_stop_flag(&global_map) { continue; }
                        if search_result.get_best_move_row().is_empty() { panic!("RIP Found no move"); }
                        
                        game.do_move(&search_result.get_best_move_algebraic());
        
                        if config.quiescence_search_mode == QuiescenceSearchMode::Alpha3 {
                            local_map.insert(DataMapKey::WhiteThreshold, search_result.get_eval() as i32);
                            local_map.insert(DataMapKey::BlackThreshold, search_result.get_eval() as i32);
                            logger.send(format!("quiescence_search_threshold: {:?}", local_map.get_data::<i32>(DataMapKey::WhiteThreshold)))
                                .expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                        }
                        
                        let calc_time_ms: u128 = local_map.get_data::<Instant>(DataMapKey::CalcTime)
                            .expect(RIP_MISSED_DM_KEY)
                            .elapsed()
                            .as_millis();
        
                        stats.calc_time_ms = calc_time_ms as usize;
                        stats.calculate();
        
                        {
                            let mut zobrist_table = zobrist_table_mutex.write().expect(RIP_COULDN_LOCK_ZOBRIST);
                            let cleaned = zobrist_table.clean_up_hash_if_needed(&config);                    
                            if cleaned > 0 { logger.send(format!("cleaned {} entries from cache", cleaned))
                                .expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE); }
                        }
        
                        let move_row = search_result.get_best_move_row();
        
                        let cp = if white { search_result.get_eval() } else { search_result.get_eval() *(-1) };
            
                        if let Err(_e) = stdout.write_get_result(&format!("info depth {} score cp {} time {} nodes {} nps {} pv {}",
                        search_result.get_depth(),
                        cp,
                        calc_time_ms,
                        stats.created_nodes,
                        stats.created_nodes / (calc_time_ms + 1) as usize,
                        move_row)
                        ) {
                            logger.send("Std Channel closed exiting".to_string()).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                            break;
                        }
                        
                        stdout.write(&format!("bestmove {}", search_result.get_best_move_algebraic()));
        
                        if config.in_debug {
                            logger.send(format!("{:?}", stats)).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                        }                
                    } else { // do book move
                        if config.in_debug {    
                            logger.send(format!("found Book move: {} for position {}", book_move, game_fen))
                                .expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                        }
                        game.do_move(book_move);
                        stdout.write(&format!("bestmove {}", book_move));
                    }
        
                    stats.reset_stats();
                }
            }

            Err(_) => {
                panic!("RIP Error reading std input");
            }
        }
    }

}


fn calculate_depth(config: &Config, complexity: i32, benchmark: i32, time: i32, global_map: &ThreadSafeDataMap) -> i32 {
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