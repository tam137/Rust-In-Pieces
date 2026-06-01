use std::io::{self, Write};
use std::thread;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::fs::OpenOptions;
use std::time::Duration;
use std::sync::atomic::Ordering;

use chrono::Local;

use crate::{time_check, Config};
use crate::service::Service;
use crate::model::EngineState;

use crate::model::RIP_COULDN_SEND_TO_GAME_CMD_QUEUE;
use crate::model::RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE;


pub fn std_reader(sender: mpsc::Sender<String>, _config: &Config) {
    loop {
        let mut uci_token = String::new();
        match io::stdin().read_line(&mut uci_token) {
            Ok(0) => {
                break;
            }
            Ok(_) => {
                if uci_token.trim().starts_with("quit") {
                    break;
                }
                let _ = sender.send(String::from(uci_token.trim()));
            }
            Err(_) => {
                break;
            }
        }
    }
}


pub fn uci_command_processor(
    engine_state: Arc<EngineState>,
    config: &Config,
    rx_std_in: Receiver<String>,
    tx_game_command: mpsc::Sender<String>,
) {
    let stdout = Service::new().stdout;
    let uci_parser = Service::new().uci_parser;
    let benchmark_value = time_check::calculate_benchmark(&engine_state);

    loop {
        match rx_std_in.recv() {
            Ok(uci_token) => {
                let logger = engine_state.log_sender.clone();

                if uci_token.trim() == "uci" {
                    stdout.write(&format!("id name Rust-In-Pieces {}", config.version));
                    stdout.write("id author Jan Lange");
                    stdout.write("option name Hash type spin default 128 min 1 max 1024");
                    stdout.write("option name Threads type spin default 1 min 1 max 8");
                    stdout.write("option name Move Overhead type spin default 0 min 0 max 5000");
                    stdout.write("option name SyzygyPath type string default <empty>");
                    stdout.write("option name Aggressiveness type string default Normal");
                    stdout.write("option name EnablePositionalCap type check default true");
                    stdout.write("option name PositionalCapDamping type spin default 5 min 1 max 100");
                    stdout.write("option name KingOpenFileMalus type spin default 40 min 0 max 500");
                    stdout.write("option name KingHalfOpenFileMalus type spin default 20 min 0 max 500");
                    stdout.write("option name KingRingDefenderValue type spin default 1 min 0 max 10");
                    stdout.write("option name ThreatMinorAttacksRook type spin default 15 min 0 max 200");
                    stdout.write("option name ThreatMinorAttacksQueen type spin default 30 min 0 max 200");
                    stdout.write("option name ThreatRookAttacksQueen type spin default 20 min 0 max 200");
                    stdout.write("option name EnableEasyMove type check default true");
                    stdout.write("option name EasyMoveDepthThreshold type spin default 6 min 1 max 100");
                    stdout.write("option name EasyMoveStableDepths type spin default 3 min 1 max 100");
                    stdout.write("option name EasyMoveMargin type spin default 150 min 0 max 10000");
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
                    tx_game_command.send("infinite".to_string()).expect("RIP Could not send 'infinite' as internal cmd");
                }

                else if uci_token.trim().starts_with("go") {
                    tx_game_command.send(uci_token).expect("RIP Could not send 'go' as internal cmd");
                }

                else if uci_token.trim().starts_with("test") {
                    time_check::run_time_check(&engine_state);
                }

                else if uci_token.trim().starts_with("debug") {
                    let logger_function: Arc<dyn Fn(String) + Send + Sync> = if uci_token.starts_with("debug on") {
                        engine_state.debug_flag.store(true, Ordering::SeqCst);

                        if config.log_to_console {
                            Arc::new(move |msg: String| {
                                print!(">{}", msg);
                            })
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

                            Arc::new(move |msg: String| {
                                let mut file = file.lock().unwrap();
                                if let Err(e) = file.write_all(msg.as_bytes()) {
                                    eprintln!("RIP Error writing to file {}", e);
                                }
                            })
                        }
                        
                    } else if uci_token.starts_with("debug off") {
                        engine_state.debug_flag.store(false, Ordering::SeqCst);
                        Arc::new(|_msg: String| {
                            // No logging
                        })
                    } else {
                        panic!("RIP Could not parse uci debug cmd");
                    };

                    *engine_state.logger.write().unwrap() = logger_function;

                    logger.send(format!("Engine startet: {}", config.version)).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                    logger.send(format!("Benchmark Value: {}", benchmark_value)).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                }

                else if uci_token.trim().starts_with("setoption") {
                    let token_lower = uci_token.to_lowercase();
                    if token_lower.contains("name aggressiveness")
                        || token_lower.contains("name positionalcapdamping")
                        || token_lower.contains("name positional_cap_damping")
                        || token_lower.contains("name enablepositionalcap")
                        || token_lower.contains("name enable_positional_cap")
                        || token_lower.contains("name move overhead")
                        || token_lower.contains("name kingopenfilemalus")
                        || token_lower.contains("name kinghalfopenfilemalus")
                        || token_lower.contains("name kingringdefendervalue")
                        || token_lower.contains("name threatminorattacksrook")
                        || token_lower.contains("name threatminorattacksqueen")
                        || token_lower.contains("name threatrookattacksqueen")
                        || token_lower.contains("easy_move")
                        || token_lower.contains("easymove")
                        || token_lower.contains("easy move")
                    {
                        tx_game_command.send(uci_token.clone()).ok();
                    } else if token_lower.contains("name threads") && token_lower.contains("value") {
                        let parts: Vec<&str> = uci_token.split_whitespace().collect();
                        if let Some(val_str) = parts.last() {
                            if let Ok(threads) = val_str.parse::<i32>() {
                                if threads > 0 {
                                    logger.send(format!("Single-threaded engine. Ignoring setoption threads to {}", threads))
                                        .expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                                }
                            }
                        }
                    } else if token_lower.contains("name hash") && token_lower.contains("value") {
                        let parts: Vec<&str> = uci_token.split_whitespace().collect();
                        if let Some(val_str) = parts.last() {
                            if let Ok(hash_size_mb) = val_str.parse::<i32>() {
                                let entries = (hash_size_mb as usize * 1024 * 1024) / 16;
                                *engine_state.zobrist_table.write().unwrap() = std::sync::Arc::new(crate::zobrist::ZobristTable::with_capacity(entries));
                                logger.send(format!("Hash size dynamically set to {} MB ({} entries)", hash_size_mb, entries))
                                    .expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                            }
                        }
                    }
                }

                else if uci_token.trim().starts_with("stop") {
                    engine_state.stop_flag.store(true, Ordering::SeqCst);
                }

                else if uci_token.trim().starts_with("quit") {
                    engine_state.stop_flag.store(true, Ordering::SeqCst);
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
                break;
            }
        }
        if let Err(_e) = io::stdout().flush() {
            panic!("RIP failed to flush stdout");
        };
    }
}


pub fn logger_buffer_thread(engine_state: Arc<EngineState>, _config: &Config, rx_log_buffer: Receiver<String>) {
    let (tx_log_msg, rx_log_msg) = mpsc::channel();

    let state_clone = engine_state.clone();
    let _log_writer = thread::spawn(move || {
        logger_thread(state_clone, &Config::new(), rx_log_msg);
    });

    loop {
        match rx_log_buffer.recv() {
            Ok(log_msg) => {
                let timestamp = Local::now().format("%H:%M:%S%.3f");
                let log_entry = format!("{} {}\n", timestamp, log_msg);
                tx_log_msg.send(log_entry).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
            }
            Err(_) => {
                break;
            }
        }
    }
}


fn logger_thread(engine_state: Arc<EngineState>, _config: &Config, rx_log_msg: Receiver<String>) {
    loop {
        match rx_log_msg.recv() {
            Ok(log_msg) => {
                let logger_function = engine_state.logger.read().unwrap().clone();
                logger_function(log_msg);
            }
            Err(_) => {
                break;
            }
        }
    }
}