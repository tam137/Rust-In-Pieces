mod fen_service;
mod notation_util;
mod model;
mod eval_service;
mod config;
mod search_service;
mod service;
mod move_gen_service;
mod book;
mod uci_parser_service;
mod zobrist;
mod stdout_wrapper;
mod global_map_handler;

use std::collections::HashMap;
use std::io;
use std::io::Write;
use std::thread;

use std::fs::OpenOptions;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc;
use std::sync::RwLock;
use std::sync::mpsc::{Sender, Receiver};
use std::time::Duration;
use std::time::Instant;

use chrono::Local;

use model::DataMapKey;
use model::QuiescenceSearchMode;
use model::UciGame;
use model::ThreadSafeDataMap;
use model::RIP_COULDN_LOCK_ZOBRIST;

use crate::book::Book;
use crate::config::Config;
use crate::model::Stats;
use crate::model::DataMap;
use crate::service::Service;



macro_rules! time_it {
    ($expr: expr) => {{
        let start = std::time::Instant::now();
        let result = { $expr };
        let end = std::time::Instant::now();
        let duration = end.duration_since(start);
        println!("Time taken [{}]: {:?}", stringify!($expr), duration);
        result
    }};
}

macro_rules! get_time_it {
    ($expr: expr) => {{
        let start = std::time::Instant::now();
        $expr;
        let end = std::time::Instant::now();
        let duration = end.duration_since(start);
        let duration_ms: i32 = duration.as_millis().min(i32::MAX as u128) as i32;
        duration_ms
    }};
}

fn get_initial_logging_info(logger: Arc<dyn Fn(String) + Send + Sync>, version: &str, benchmark_value: i32) {
    logger(format!("Engine startet: {}", version));
    logger(format!("Benchmark Value: {}", benchmark_value));
}

fn main() {

    let (tx_hashes, rx_hashes): (Sender<(u64, i16)>, Receiver<(u64, i16)>) = mpsc::channel();
    
    let global_map  = global_map_handler::create_new_global_map();
    let global_map_t2 = global_map.clone();
    let global_map_t3 = global_map.clone();

    global_map_handler::add_hash_sender(&global_map, tx_hashes);

    let mut local_map = DataMap::new();

    let book = Book::new();
    let service = &Service::new();
    let uci_parser = &service.uci_parser;
    let config = Config::new();
    let version = String::from(config.version.clone());
    let stdout = &service.stdout;

    if config.quiescence_search_mode == QuiescenceSearchMode::Alpha3 {
        local_map.insert(DataMapKey::WhiteThreshold, 0);
        local_map.insert(DataMapKey::BlackThreshold, 0);
    }

    let benchmark_value = calculate_benchmark(10000, &global_map, &mut local_map);

    let mut stats = Stats::new();
    let mut game = UciGame::new(service.fen.set_init_board());  
    let mut white = game.board.white_to_move; 

    let mut update_board_via_uci_token: bool = false;

    let mut logger = global_map_handler::get_logger(&global_map);
    let stop_flag = global_map_handler::get_stop_flag(&global_map);
    let debug_flag = global_map_handler::get_debug_flag(&global_map);
    let zobrist_table_mutex = global_map_handler::get_zobrist_table(&global_map);



    let _handle1 = thread::spawn(move || {
        let logger = global_map_handler::get_logger(&global_map_t3);
        let mut hash_buffer: HashMap<u64, i16> = HashMap::default();
        loop {
            let received = match rx_hashes.recv() {
                Ok(msg) => msg,
                Err(_) => {
                    logger("Hash queue closed".to_string());
                    break;
                }
            };

            let (hash, eval) = received;
            hash_buffer.insert(hash, eval);

            if hash_buffer.len() > config.write_hash_buffer_size {
                let zobrist_table = global_map_handler::get_zobrist_table(&global_map_t3);
                let mut zobrist_table = zobrist_table.write().expect(RIP_COULDN_LOCK_ZOBRIST);
                // Schreibe die gepufferten Werte in den ZobristTable
                for (hash, eval) in hash_buffer.drain() {
                    zobrist_table.set_new_hash(&hash, eval);
                }
                hash_buffer.clear();
            }
        }
    });

    let (tx_commands, rx_commands) = mpsc::channel();

    let _handle2 = thread::spawn(move || {

        let stdout = Service::new().stdout;
        let uci_parser = Service::new().uci_parser;

        loop {
            let mut uci_token = String::new();
            match io::stdin().read_line(&mut uci_token) {
                Ok(_) => {
                    //log(format!("RIP received '{}'", uci_token));
                    if uci_token.trim() == "uci" {
                        stdout.write(&format!("id name SupraH {}", version));
                        stdout.write("id author Jan Lange");
                        stdout.write("uciok");
                    }

                    else if uci_token.trim() == "uciready" {
                        stdout.write("readyok");
                    }

                    else if uci_token.trim() == "ucinewgame" {
                        tx_commands.send("ucinewgame".to_string()).expect("RIP Could not send 'ucinewgame' as internal cmd");
                    }

                    else if uci_token.trim() == "isready" {
                        stdout.write("readyok");
                    }

                    else if uci_token.trim().starts_with("position") {
                        let (fen, moves_str) = uci_parser.parse_position(&uci_token);
                        tx_commands.send(format!("board {}", fen)).expect("RIP Could not send 'board' as internal cmd");
                        tx_commands.send(format!("moves {}", moves_str)).expect("RIP Could not send 'move' as internal cmd");
                    }

                    else if uci_token.trim() == "go infinite" {
                        tx_commands.send("ucinewgame".to_string()).expect("RIP Could not send 'ucinewgame' as internal cmd");
                        tx_commands.send("infinite".to_string()).expect("RIP Could not send 'infinite' as internal cmd");
                    }

                    else if uci_token.trim().starts_with("go") {
                        tx_commands.send(uci_token).expect("RIP Could not send 'go' as internal cmd");
                    }

                    else if uci_token.trim().starts_with("test") {
                        tx_commands.send(format!("test")).expect("RIP Could not send 'test' as internal cmd");
                    }

                    else if uci_token.trim().starts_with("debug") {
                        logger = if uci_token.starts_with("debug on") {
                            let file = Arc::new(Mutex::new(
                                OpenOptions::new()
                                    .write(true)
                                    .append(true)
                                    .create(true)
                                    .open(format!("rust-in-piece-{}.log", version))
                                    .expect("Failed to open log file"),
                            ));
                            let mut debug_flag_value = debug_flag.lock().expect("RIP Can not lock debug_flag");
                            *debug_flag_value = true;
                            Arc::from(Box::new(move |msg: String| {
                                let timestamp = Local::now().format("%H:%M:%S%.3f");
                                let log_entry = format!("{} {}\n", timestamp, msg);
                                let mut file = file.lock().unwrap();
                                if let Err(e) = file.write_all(log_entry.as_bytes()) {
                                    eprintln!("RIP Error writing to file {}", e);
                                }
                            }) as Box<dyn Fn(String) + Send + Sync>)
                        } else if uci_token.starts_with("debug off") {
                            let mut debug_flag_value = debug_flag.lock().expect("RIP Can not lock debug_flag");
                            *debug_flag_value = false;
                            Arc::from(Box::new(|_msg: String| {
                                // No logging
                            }) as Box<dyn Fn(String) + Send + Sync>)
                        } else {
                            panic!("RIP Could not parse uci debug cmd");
                        };
                        let mut global_map_value = global_map.write().expect("RIP Could not lock global map");
                        global_map_value.insert(DataMapKey::Logger, logger.clone());                            
                        get_initial_logging_info(logger.clone(), &version, benchmark_value);
                    }

                    else if uci_token.trim().starts_with("stop") {
                        let mut stop_flag_value = stop_flag.lock().expect("RIP Can not lock stop_flag");
                        *stop_flag_value = true;
                    }

                    else if uci_token.trim().starts_with("quit") {
                        let mut value = stop_flag.lock().expect("RIP Can not lock stop_flag");
                        *value = true;
                        tx_commands.send("quit".to_string()).expect("RIP Could not send 'quit' as internal cmd");
                        break;
                    }

                    else {
                        if !uci_token.is_empty() {
                            logger("cmd unknown".to_string() + &uci_token);
                        }                        
                        thread::sleep(Duration::from_millis(5));
                    }
                },
                Err(_) => {
                    panic!("RIP Error reading std input");
                }
            }
            if let Err(_e) = io::stdout().flush() {
                logger("RIP failed to flush stdout".to_string());
            };
        }
    });



    loop {

        let global_map = global_map_t2.clone();

        let logger = global_map_handler::get_logger(&global_map);
        let stop_flag = global_map_handler::get_stop_flag(&global_map);

        let received = match rx_commands.recv() {
            Ok(msg) => msg,
            Err(_) => {
                logger("Internal Command Channel closed, exiting".to_string());
                break;
            }
        };

        if received == "infinite" {
            for depth in (2..100).step_by(2) {
                let _r = &service.search.get_moves(&mut game.board, depth, white, &mut stats, &config, &service, &global_map, &mut local_map);
            }
        }

        else if received.starts_with("board") {
            update_board_via_uci_token = false;
            let fen = received[6..].to_string();
            if fen != game.init_board_setup_fen {
                logger(format!("received new Board informations {} -> {}", game.init_board_setup_fen, fen));
                 game = UciGame::new(service.fen.set_fen(&fen));
                 update_board_via_uci_token = true;
            }
        }

        else if received.starts_with("go") {

            let calc_time = Instant::now();

            white = game.white_to_move();

            // info depth 2 score cp 214 time 1242 nodes 2124 nps 34928 pv e2e4 e7e5 g1f3
            // go wtime 31520 btime 1410

            let game_fen = service.fen.get_fen(&game.board);
            let book_move = book.get_random_book_move(&game_fen);

            let times: (i32, i32) = uci_parser.parse_go(received.as_str());
            let wtime = times.0;
            let btime = times.1;

            if book_move.is_empty() {

                let my_time_ms = if white { wtime } else { btime };
                let calculated_depth = 
                    calculate_depth(&config, game.board.calculate_complexity(), benchmark_value, my_time_ms, &global_map);
                
                let search_result =
                    &service.search.get_moves(&mut game.board, calculated_depth, white, &mut stats, &config, &service, &global_map, &mut local_map);
                
                game.do_move(&search_result.get_best_move_algebraic());

                if config.quiescence_search_mode == QuiescenceSearchMode::Alpha3 {
                    local_map.insert(DataMapKey::WhiteThreshold, search_result.get_eval() as i32);
                    local_map.insert(DataMapKey::BlackThreshold, search_result.get_eval() as i32);
                    logger(format!("quiescence_search_threshold: {:?}", local_map.get_data::<i32>(DataMapKey::WhiteThreshold)));
                }
                
                let calc_time_ms: u128 = calc_time.elapsed().as_millis();
                stats.calc_time_ms = calc_time_ms as usize;
                stats.calculate();

                {
                    let mut zobrist_table = zobrist_table_mutex.write().expect(RIP_COULDN_LOCK_ZOBRIST);
                    let cleaned = zobrist_table.clean_up_hash_if_needed(&config);                    
                    if cleaned > 0 { logger(format!("cleaned {} entries from cache", cleaned)); }
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
                    logger("Std Channel closed exiting".to_string());
                    break;
                }
                
                stdout.write(&format!("bestmove {}", search_result.get_best_move_algebraic()));

                if config.in_debug {
                    logger(format!("{:?}", stats));
                }                
            } else {
                if config.in_debug {    
                    logger(format!("found Book move: {} for position {}", book_move, game_fen));
                }
                game.do_move(book_move);
                stdout.write(&format!("bestmove {}", book_move));
            }

            stats.reset_stats();

        }
        
        else if received.starts_with("moves") {
            if update_board_via_uci_token {
                let moves_str = &received[5..];
                let moves_iter = moves_str.split_whitespace();
                for mv in moves_iter {
                    game.do_move(mv);
                }
            } else {
                let moves_str = &received[5..];
                let algebraic_notation = uci_parser.parse_last_move_from_moves_str(moves_str);
                logger(format!("uci: received move '{}' ", algebraic_notation));                
                game.do_move(&algebraic_notation);
            }
            
        }
        
        else if received == "ucinewgame" {
            stats = Stats::new();
            game = UciGame::new(service.fen.set_init_board());  
            white = game.board.white_to_move;
            let mut stop_flag_value = stop_flag.lock().expect("RIP Can not lock stop flag");
            *stop_flag_value = false;
            continue;
        }
        
        else if received == "test" {
            run_time_check(&global_map, &mut local_map);
        }
        
        else if received == "quit" {
            logger(format!("uci: received 'quit'"));
            break;
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

    if value > 200 {
        if config.in_debug {
            logger(format!("time threshold: {} -> depth: {}", value, 10));
        }        
        return 10;
    } else if value > 150 {
        if config.in_debug {
            logger(format!("time threshold: {} -> depth: {}", value, 8));
        }        
        return 8;
    } else if value > 90 {
        if config.in_debug {
            logger(format!("time threshold: {} -> depth: {}", value, 6));
        }        
        return 6;
    } else if value >= 6 {
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


fn calculate_benchmark (normalized_value: i32, global_map: &Arc<RwLock<DataMap>>, local_map: &mut DataMap) -> i32 {
    let mut board = Service::new().fen.set_fen("r1bqkbnr/1ppp1ppp/p1n5/1B2p3/4P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 0 4");
    let service = Service::new();
    let config = &Config::new().for_tests();

    normalized_value / get_time_it!(service.search.get_moves(&mut board, 4, true, &mut Stats::new(), &config, &service, global_map, local_map))
}

fn run_time_check(global_map: &ThreadSafeDataMap, local_map: &mut DataMap) {
    let service = &Service::new();
    let config = &Config::new().for_tests();
    let mut stats = Stats::new();

    println!("expected <5µs");
    let mut board = time_it!(service.fen.set_init_board());

    let turn = &notation_util::NotationUtil::get_turn_from_notation("e2e4");
    
    println!("\nexpected <5µs");
    let mi = time_it!(board.do_move(turn));

    println!("\nexpected <300ns");
    time_it!(board.undo_move(turn, mi));

    println!("\nexpected <5µs");
    time_it!(board.hash());

    println!("\nexpected ~45µs");
    time_it!(service.move_gen.generate_valid_moves_list(&mut board, &mut stats, service, config, global_map));

    println!("\nexpected ~1µs");
    time_it!(service.eval.calc_eval(&board, &config, &service.move_gen));
    
    println!("\nexpected ~1900ms");
    time_it!(service.search.get_moves(&mut service.fen.set_init_board(), 6, true, &mut Stats::new(), config, service, global_map, local_map)); 
    
    println!("\nexpected ~300ms");
    let mid_game_fen = "r1bqr1k1/ppp2ppp/2np1n2/2b1p3/2BPP3/2P1BN2/PPQ2PPP/RN3RK1 b - - 5 8";
    time_it!(service.search.get_moves(&mut service.fen.set_fen(mid_game_fen), 4, false, &mut Stats::new(), config, service, global_map, local_map));
    
    println!("\nexpected ~150ms");
    let mid_game_fen = "r1bqr1k1/2p2ppp/p1np1n2/1pb1p1N1/2BPP3/2P1B3/PPQ2PPP/RN3RK1 w - - 0 10";
    time_it!(service.search.get_moves(&mut service.fen.set_fen(mid_game_fen), 4, true, &mut Stats::new(), config, service, global_map, local_map));

    println!("\nexpected >60");
    println!("Benchmark Value: {}", calculate_benchmark(10000, global_map, local_map));
}
