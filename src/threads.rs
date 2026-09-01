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


/// The UCI options this engine advertises, with every default read from the supplied
/// `Config` rather than written out as a literal.
///
/// Twelve advertised defaults had drifted from the value the engine actually uses, one SPSA
/// run at a time, and three separate releases each corrected one by hand. Deriving them here
/// makes that drift impossible, and exposing the list as data lets
/// `test_every_advertised_uci_option_is_accepted` check the other half of the same facade:
/// that every name the engine advertises actually reaches a field. See `task.md` 1.1.
pub fn uci_options(defaults: &Config) -> Vec<String> {
    vec![
        "option name Hash type spin default 128 min 1 max 1024".to_string(),
        "option name Threads type spin default 1 min 1 max 8".to_string(),
        format!("option name Move Overhead type spin default {} min 0 max 5000", defaults.move_overhead),
        "option name BookFile type string default <empty>".to_string(),
        format!("option name OwnBook type check default {}", defaults.use_book),
        format!("option name CacheBookInRam type check default {}", defaults.cache_book_in_ram),
        format!("option name BookMaxPly type spin default {} min 0 max 128", defaults.book_max_ply),
        "option name Aggressiveness type string default Normal".to_string(),
        format!("option name EnablePositionalCap type check default {}", defaults.enable_positional_cap),
        format!("option name PositionalCapDamping type spin default {} min 1 max 100", defaults.positional_cap_damping),
        format!("option name KingOpenFileMalus type spin default {} min 0 max 500", defaults.king_open_file_malus),
        format!("option name KingHalfOpenFileMalus type spin default {} min 0 max 500", defaults.king_half_open_file_malus),
        format!("option name KingRingDefenderValue type spin default {} min 0 max 10", defaults.king_ring_defender_value),
        format!("option name ThreatMinorAttacksRook type spin default {} min 0 max 200", defaults.threat_minor_attacks_rook),
        format!("option name ThreatMinorAttacksQueen type spin default {} min 0 max 200", defaults.threat_minor_attacks_queen),
        format!("option name ThreatRookAttacksQueen type spin default {} min 0 max 200", defaults.threat_rook_attacks_queen),
        "option name LogPath type string default <empty>".to_string(),
        format!("option name ConnectedPassedPawnMg type spin default {} min -500 max 500", defaults.connected_passed_pawn_mg),
        format!("option name ConnectedPassedPawnEg type spin default {} min -500 max 500", defaults.connected_passed_pawn_eg),
        format!("option name KnightOutpostTrueMg type spin default {} min -500 max 500", defaults.knight_outpost_true_mg),
        format!("option name KnightOutpostTrueEg type spin default {} min -500 max 500", defaults.knight_outpost_true_eg),
        format!("option name BishopOutpostTrueMg type spin default {} min -500 max 500", defaults.bishop_outpost_true_mg),
        format!("option name BishopOutpostTrueEg type spin default {} min -500 max 500", defaults.bishop_outpost_true_eg),
        format!("option name KingPawnShieldKingside type spin default {} min -500 max 500", defaults.king_pawn_shield_kingside),
        format!("option name KingPawnShieldQueenside type spin default {} min -500 max 500", defaults.king_pawn_shield_queenside),
        format!("option name KingPieceShieldKingside type spin default {} min -500 max 500", defaults.king_piece_shield_kingside),
        format!("option name KingPieceShieldQueenside type spin default {} min -500 max 500", defaults.king_piece_shield_queenside),
        format!("option name OppositeBishopsDrawScale type spin default {} min 0 max 100", defaults.opposite_bishops_draw_scale),
        format!("option name RookBehindEnemyPassedPawnMg type spin default {} min -500 max 500", defaults.rook_behind_enemy_passed_pawn_mg),
        format!("option name RookBehindEnemyPassedPawnEg type spin default {} min -500 max 500", defaults.rook_behind_enemy_passed_pawn_eg),
        format!("option name EnableLazyEval type check default {}", defaults.enable_lazy_eval),
        format!("option name LazyEvalMinGamePhase type spin default {} min 0 max 256", defaults.lazy_eval_min_game_phase),
        format!("option name LazyEvalMarginSearch type spin default {} min 10 max 1000", defaults.lazy_eval_margin_search),
        format!("option name LazyEvalMarginQs type spin default {} min 10 max 1000", defaults.lazy_eval_margin_qs),
        format!("option name EnableFutilityPruning type check default {}", defaults.enable_futility_pruning),
        format!("option name FutilityMaxDepth type spin default {} min 1 max 10", defaults.futility_max_depth),
        format!("option name FutilityMarginBase type spin default {} min 0 max 500", defaults.futility_margin_base),
        format!("option name FutilityMarginSlope type spin default {} min 0 max 300", defaults.futility_margin_slope),
        format!("option name EnableRazoring type check default {}", defaults.enable_razoring),
        format!("option name RazoringMargin type spin default {} min 50 max 800", defaults.razoring_margin),
        format!("option name EnableLmp type check default {}", defaults.enable_lmp),
        // // `lmp_max_depth` is inert above 4: the `lmp_base_moves + 2 * depth^2` threshold
        // // demands more quiet moves at a single node than any node produces. See
        // // `task.md` 10.6, pinned by `test_lmp_max_depth_is_inert_above_four`.
        format!("option name LmpMaxDepth type spin default {} min 1 max 4", defaults.lmp_max_depth),
        format!("option name LmpBaseMoves type spin default {} min 0 max 20", defaults.lmp_base_moves),
        format!("option name EnableBadCapturePruning type check default {}", defaults.enable_bad_capture_pruning),
        format!("option name BadCaptureSeeThreshold type spin default {} min -400 max 0", defaults.bad_capture_see_threshold),
        format!("option name AspirationWindowMaxDelta type spin default {} min 50 max 30000", defaults.aspiration_window_max_delta),
        format!("option name EnableCheckExtension type check default {}", defaults.enable_check_extension),
        format!("option name CheckExtensionMaxPly type spin default {} min 0 max 127", defaults.check_extension_max_ply),
        format!("option name CheckExtensionRequireSafe type check default {}", defaults.check_extension_require_safe),
        format!("option name CheckExtensionBudgetDivisor type spin default {} min 0 max 16", defaults.check_extension_budget_divisor),
        format!("option name CheckExtensionMinDepth type spin default {} min 0 max 32", defaults.check_extension_min_depth),
        format!("option name CheckExtensionMaxDepth type spin default {} min 0 max 32", defaults.check_extension_max_depth),
        format!("option name EnableOneReplyExtension type check default {}", defaults.enable_one_reply_extension),
        format!("option name EnableSingularExtensions type check default {}", defaults.enable_singular_extensions),
        format!("option name SingularMinDepth type spin default {} min 2 max 32", defaults.singular_min_depth),
        format!("option name SingularTtDepthMargin type spin default {} min 0 max 8", defaults.singular_tt_depth_margin),
        format!("option name SingularMargin type spin default {} min 0 max 64", defaults.singular_margin),
        format!("option name SingularDepthReduction type spin default {} min 0 max 8", defaults.singular_depth_reduction),
        format!("option name EnableSingularMulticut type check default {}", defaults.enable_singular_multicut),
        format!("option name UseNNUE type check default {}", defaults.use_nnue),
        "option name NnueModelPath type string default eval_models/quantised.bin".to_string(),
    ]
}

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

    let mut active_use_nnue = config.use_nnue;

    while let Ok(uci_token) = rx_std_in.recv() {
        let logger = engine_state.log_sender.clone();

                if uci_token.trim() == "uci" {
                    let name_str = if active_use_nnue {
                        format!("id name RIP V{}-NNUE", config.version)
                    } else {
                        format!("id name Rust-In-Pieces V{}", config.version)
                    };
                    // Every advertised default is read from `Config::default()` rather than written out as a
                    // literal. Twelve of them had drifted from the value the engine actually
                    // uses, one SPSA run at a time, and three earlier releases each corrected
                    // one by hand. See `task.md` 1.1.
                    let defaults = Config::new();
                    stdout.write(&name_str);
                    stdout.write("id author Jan Lange");
                    for option in uci_options(&defaults) {
                        stdout.write(&option);
                    }
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

                    logger.send(format!("Engine started: {}", config.version)).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                    logger.send(format!("Benchmark Value: {}", benchmark_value)).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                }

                else if uci_token.trim().starts_with("setoption") {
                    let token_lower = uci_token.to_lowercase();
                    if token_lower.contains("usennue") || token_lower.contains("use_nnue") {
                        let parts: Vec<&str> = uci_token.split_whitespace().collect();
                        if let Some(val_str) = parts.last() {
                            active_use_nnue = val_str.to_lowercase() == "true";
                        }
                        // ...and the token still has to reach the game thread, which owns the
                        // only `Config` the search ever reads. Intercepting it here to rename the
                        // engine and then dropping it left the option advertised, accepted and
                        // inert — the defect v0.37.2 set out to eliminate, re-introduced by its
                        // own repair. `task.md` 1.1.
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
                    } else {
                        if token_lower.contains("name logpath") || token_lower.contains("name log_path") {
                            let parts: Vec<&str> = uci_token.split_whitespace().collect();
                            if let Some(val_idx) = parts.iter().position(|&r| r.to_lowercase() == "value") {
                                let val_str = parts[val_idx+1..].join(" ");
                                let path = std::path::Path::new(&val_str);
                                let file_path = if path.is_dir() {
                                    path.join(format!("engine_{}.log", std::process::id()))
                                } else {
                                    path.to_path_buf()
                                };

                                if let Some(parent) = file_path.parent() {
                                    let _ = std::fs::create_dir_all(parent);
                                }

                                if let Ok(file) = OpenOptions::new()
                                    
                                    .append(true)
                                    .create(true)
                                    .open(&file_path)
                                {
                                    let file = Arc::new(Mutex::new(file));
                                    let logger_function: Arc<dyn Fn(String) + Send + Sync> = Arc::new(move |msg: String| {
                                        let mut file = file.lock().unwrap();
                                        let _ = file.write_all(msg.as_bytes());
                                    });
                                    *engine_state.logger.write().unwrap() = logger_function;
                                }
                            }
                        }
                        tx_game_command.send(uci_token.clone()).ok();
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
                        logger.send(format!("cmd unknown: {}", uci_token)).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                    }                        
                    thread::sleep(Duration::from_millis(5));
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

    while let Ok(log_msg) = rx_log_buffer.recv() {
        let timestamp = Local::now().format("%H:%M:%S%.3f");
        let log_entry = format!("{} {}\n", timestamp, log_msg);
        tx_log_msg.send(log_entry).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
    }
}


fn logger_thread(engine_state: Arc<EngineState>, _config: &Config, rx_log_msg: Receiver<String>) {
    while let Ok(log_msg) = rx_log_msg.recv() {
        let logger_function = engine_state.logger.read().unwrap().clone();
        logger_function(log_msg);
    }
}