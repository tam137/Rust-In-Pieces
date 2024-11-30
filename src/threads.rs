use std::io::{self, Write};
use std::collections::HashMap;
use std::thread;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::fs::OpenOptions;
use std::time::Duration;
use chrono::Local;

use crate::Instant;
use crate::DataMap;
use crate::{time_check, Config};
use crate::Service;
use crate::model::ThreadSafeDataMap;
use crate::DataMapKey;
use crate::global_map_handler;
use crate::model::RIP_COULDN_LOCK_ZOBRIST;
use crate::model::RIP_COULDN_SEND_TO_STD_IN_QUEUE;
use crate::model::RIP_COULDN_SEND_TO_GAME_CMD_QUEUE;
use crate::model::RIP_ERR_READING_STD_IN;


pub fn hash_writer(global_map: ThreadSafeDataMap, config: &Config, rx_hashes: Receiver<(u64, i16)>)  {
    let logger = global_map_handler::get_logger(&global_map);
    let mut hash_buffer: HashMap<u64, i16> = HashMap::default();
    loop {
        let received = match rx_hashes.recv() {
            Ok(msg) => msg,
            Err(_) => {
                logger("Err: Hash queue closed, can not write".to_string());
                break;
            }
        };

        let (hash, eval) = received;
        hash_buffer.insert(hash, eval);

        if hash_buffer.len() > config.write_hash_buffer_size {
            let zobrist_table = global_map_handler::get_zobrist_table(&global_map);
            let mut zobrist_table = zobrist_table.write().expect(RIP_COULDN_LOCK_ZOBRIST);

            for (hash, eval) in hash_buffer.drain() {
                zobrist_table.set_new_hash(&hash, eval);
            }
            hash_buffer.clear();
        }
    }
}


pub fn std_reader(global_map: ThreadSafeDataMap, _config: &Config) {    
    let sender = global_map_handler::get_std_in_sender(&global_map);
    loop {
        let mut uci_token = String::new();
        match io::stdin().read_line(&mut uci_token) {
            Ok(_) => {
                if uci_token.trim().starts_with("quit") {
                    break;
                }
                sender.send(uci_token).expect(RIP_COULDN_SEND_TO_STD_IN_QUEUE);
            }
            Err(_) => {
                panic!("{}", RIP_ERR_READING_STD_IN);
            }
        }
    }
}


pub fn uci_command_processor(global_map: ThreadSafeDataMap, config: &Config, rx_std_in: Receiver<String>) {

    let mut local_map = DataMap::new();
    local_map.insert(DataMapKey::CalcTime, Instant::now());

    let stdout = Service::new().stdout;
    let uci_parser = Service::new().uci_parser;
    let mut logger = global_map_handler::get_logger(&global_map);
    let debug_flag = global_map_handler::get_debug_flag(&global_map);
    let stop_flag = global_map_handler::get_stop_flag(&global_map);
    let tx_game_command = global_map_handler::get_game_command_sender(&global_map);
    let benchmark_value = time_check::calculate_benchmark(&global_map, &mut local_map);

    loop {
        
        match rx_std_in.recv() {
            Ok(uci_token) => {
                if config.print_commands {
                    logger(format!("RIP received '{}'", uci_token));
                }

                if uci_token.trim() == "uci" {
                    stdout.write(&format!("id name SupraH {}", config.version));
                    stdout.write("id author Jan Lange");
                    stdout.write("uciok");
                }

                else if uci_token.trim() == "uciready" {
                    stdout.write("readyok");
                }

                else if uci_token.trim() == "ucinewgame" {
                    tx_game_command.send("ucinewgame".to_string()).expect(RIP_COULDN_SEND_TO_GAME_CMD_QUEUE);
                }

                else if uci_token.trim() == "isready" {
                    stdout.write("readyok");
                }

                else if uci_token.trim().starts_with("position") {
                    let (fen, moves_str) = uci_parser.parse_position(&uci_token);
                    tx_game_command.send(format!("board {}", fen)).expect("RIP Could not send 'board' as internal cmd");
                    tx_game_command.send(format!("moves {}", moves_str)).expect("RIP Could not send 'move' as internal cmd");
                }

                else if uci_token.trim() == "go infinite" {
                    tx_game_command.send("ucinewgame".to_string()).expect("RIP Could not send 'ucinewgame' as internal cmd");
                    tx_game_command.send("infinite".to_string()).expect("RIP Could not send 'infinite' as internal cmd");
                }

                else if uci_token.trim().starts_with("go") {
                    tx_game_command.send(uci_token).expect("RIP Could not send 'go' as internal cmd");
                }

                else if uci_token.trim().starts_with("test") {
                    time_check::run_time_check(&global_map, &mut local_map);
                }

                else if uci_token.trim().starts_with("debug") {
                    logger = if uci_token.starts_with("debug on") {
                        let file = Arc::new(Mutex::new(
                            OpenOptions::new()
                                .write(true)
                                .append(true)
                                .create(true)
                                .open(format!("rust-in-piece-{}.log", config.version))
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
                    logger(format!("Engine startet: {}", config.version));
                    logger(format!("Benchmark Value: {}", benchmark_value));
                }

                else if uci_token.trim().starts_with("stop") {
                    let mut stop_flag_value = stop_flag.lock().expect("RIP Can not lock stop_flag");
                    *stop_flag_value = true;
                }

                else if uci_token.trim().starts_with("quit") {
                    let mut value = stop_flag.lock().expect("RIP Can not lock stop_flag");
                    *value = true;
                    tx_game_command.send("quit".to_string()).expect("RIP Could not send 'quit' as internal cmd");
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
                panic!("RIP Error reading from channel");
            }
        }
        if let Err(_e) = io::stdout().flush() {
            logger("RIP failed to flush stdout".to_string());
        };
    }
}
