use std::io::{self, Write};
use std::collections::HashMap;
use std::thread;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::fs::OpenOptions;
use std::time::Duration;

use chrono::Local;
use crossbeam_queue::SegQueue;

use crate::{time_check, Config};
use crate::Service;
use crate::model::ThreadSafeDataMap;
use crate::DataMapKey;
use crate::global_map_handler;
use crate::model::LoggerFnType;

use crate::model::RIP_COULDN_SEND_TO_STD_IN_QUEUE;
use crate::model::RIP_COULDN_SEND_TO_GAME_CMD_QUEUE;
use crate::model::RIP_ERR_READING_STD_IN;
use crate::model::RIP_COULDN_LOCK_GLOBAL_MAP;
use crate::model::RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE;


pub fn hash_writer(global_map: ThreadSafeDataMap, config: &Config, hash_queue: Arc<SegQueue<(u64, i16)>>) {
    let mut hash_buffer: HashMap<u64, i16> = HashMap::default();

    loop {
        while let Some((hash, eval)) = hash_queue.pop() {
            hash_buffer.insert(hash, eval);
        }

        if hash_buffer.len() >= config.write_hash_buffer_size {
            let chash_map = &mut global_map_handler::get_zobrist_table(&global_map).hash_map.clone();

            for (hash, eval) in hash_buffer.drain() {
                chash_map.insert(hash, eval);
            }

            let clean_up_size = if config.max_zobrist_hash_entries <= chash_map.len() {
                let size = chash_map.len();
                chash_map.clear();
                size
            } else {
                0
            };

            if clean_up_size > 0 {
                global_map_handler::get_log_buffer_sender(&global_map)
                    .send(format!("Cleared hash table: {} entries", clean_up_size))
                    .expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
            }
        }
        thread::sleep(Duration::from_millis(1));
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
                sender.send(String::from(uci_token.trim())).expect(RIP_COULDN_SEND_TO_STD_IN_QUEUE);
            }
            Err(_) => {
                panic!("{}", RIP_ERR_READING_STD_IN);
            }
        }
    }
}


pub fn uci_command_processor(global_map: ThreadSafeDataMap, config: &Config, rx_std_in: Receiver<String>) {

    let mut local_map = global_map_handler::_get_default_local_map();

    let stdout = Service::new().stdout;
    let uci_parser = Service::new().uci_parser;
    //let debug_flag = global_map_handler::is_debug_flag(&global_map);
    //let stop_flag = global_map_handler::is_stop_flag(&global_map);
    let tx_game_command = global_map_handler::get_game_command_sender(&global_map);
    let benchmark_value = time_check::calculate_benchmark(&global_map, &mut local_map);

    loop {

        match rx_std_in.recv() {
            Ok(uci_token) => {

                let logger = global_map_handler::get_log_buffer_sender(&global_map);

                if uci_token.trim() == "uci" {
                    stdout.write(&format!("id name Rust-In-Pieces {}", config.version));
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

                    let logger_function: Arc<dyn Fn(String) + Send + Sync> = if uci_token.starts_with("debug on") {

                        global_map_handler::set_debug_flag(&global_map, true);

                        if config.log_to_console {
                            Arc::from(Box::new(move |msg: String| {
                                print!(">{}", msg);
                            }) as Box<dyn Fn(String) + Send + Sync>)
                        }
                        else {
                            let file = Arc::new(Mutex::new(
                                OpenOptions::new()
                                    .write(true)
                                    .append(true)
                                    .create(true)
                                    .open(format!("rust-in-piece-{}.log", config.version))
                                    .expect("RIP Failed to open log file"),
                            ));

                            Arc::from(Box::new(move |msg: String| {
                                let mut file = file.lock().unwrap();
                                if let Err(e) = file.write_all(msg.as_bytes()) {
                                    eprintln!("RIP Error writing to file {}", e);
                                }
                            }) as Box<dyn Fn(String) + Send + Sync>)
                        }
                        
                    } else if uci_token.starts_with("debug off") {
                        global_map_handler::set_debug_flag(&global_map, false);
                        Arc::from(Box::new(|_msg: String| {
                            // No logging
                        }) as Box<dyn Fn(String) + Send + Sync>)
                    } else {
                        panic!("RIP Could not parse uci debug cmd");
                    };
                    {
                        let mut global_map_value = global_map.write().expect("RIP Could not lock global map");
                        global_map_value.insert(DataMapKey::Logger, logger_function.clone());  
                    }
                    logger.send(format!("Engine startet: {}", config.version)).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                    logger.send(format!("Benchmark Value: {}", benchmark_value)).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                }

                else if uci_token.trim().starts_with("stop") {
                    global_map_handler::set_stop_flag(&global_map, true);
                }

                else if uci_token.trim().starts_with("quit") {
                    global_map_handler::set_stop_flag(&global_map, true);
                    tx_game_command.send("quit".to_string()).expect("RIP Could not send 'quit' as internal cmd");
                    break;
                }

                else {
                    if !uci_token.is_empty() {
                        logger.send("cmd unknown".to_string() + &uci_token).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                    }                        
                    thread::sleep(Duration::from_millis(5));
                }
            },
            Err(_) => {
                panic!("RIP Error reading from channel");
            }
        }
        if let Err(_e) = io::stdout().flush() {
            panic!("RIP failed to flush stdout");
        };
    }
}


pub fn logger_buffer_thread(global_map: ThreadSafeDataMap, _config: &Config, rx_log_buffer: Receiver<String>) {
    let (tx_log_msg, rx_log_msg) = mpsc::channel();

    let _log_writer = thread::spawn(move || {
        logger_thread(global_map, &Config::new(), rx_log_msg);
    });

    loop {
        match rx_log_buffer.recv() {
            Ok(log_msg) => {
                let timestamp = Local::now().format("%H:%M:%S%.3f");
                let log_entry = format!("{} {}\n", timestamp, log_msg);
                tx_log_msg.send(log_entry).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
            }
            Err(_) => {
                panic!("RIP Error reading from channel");
            }
        }
    }
}


fn logger_thread(global_map: ThreadSafeDataMap, _config: &Config, rx_log_msg: Receiver<String>) {
    loop {
        let logger_function = global_map.read().expect(RIP_COULDN_LOCK_GLOBAL_MAP)
        .get_data::<LoggerFnType>(DataMapKey::Logger)
        .expect("RIP Can not find logger")
        .clone();

        match rx_log_msg.recv() {
            Ok(log_msg) => {
                logger_function(log_msg);
            }
            Err(_) => {
                panic!("RIP Error reading from channel");
            }
        }
    }
}