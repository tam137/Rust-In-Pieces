use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread;

use crate::DataMap;
use crate::DataMapKey;
use crate::Config;
use crate::model::{EngineState, TimeInfo, TimeMode, SearchResult, UciGame, Stats};
use crate::service::Service;
use crate::book::Book;
use crate::zobrist;

use crate::model::RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE;


pub fn game_loop(engine_state: Arc<EngineState>, config: &Config, rx_game_command: Receiver<String>) {
    let service = &Service::new();
    let uci_parser = &service.uci_parser;
    let stdout = &service.stdout;
    let mut game = UciGame::new(service.fen.set_init_board());
    let book = Book::new();
    let logger = engine_state.log_sender.clone();

    let mut local_map = DataMap::new();
    local_map.insert(DataMapKey::CalcTime, Instant::now());



    loop {
        match rx_game_command.recv() {
            Ok(command) => {
                if command.trim() == "ucinewgame" {
                    game = UciGame::new(service.fen.set_init_board());
                    engine_state.stop_flag.store(false, Ordering::SeqCst);
                    engine_state.pv_nodes.lock().unwrap().clear();
                    engine_state.pv_nodes_len.store(0, Ordering::SeqCst);
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
                    engine_state.stop_flag.store(false, Ordering::SeqCst);
                    let mut local_map = local_map.clone();
                    local_map.insert(DataMapKey::CalcTime, Instant::now());
                    
                    let mut best_result: Option<SearchResult> = None;
                    for depth in 2..100 {
                        if engine_state.stop_flag.load(Ordering::SeqCst) {
                            break;
                        }

                        logger.send(format!("Start Level {}", depth)).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);

                        let is_white = game.board.white_to_move;
                        let mut stats = Stats::default();
                        let search_result = service.search.get_moves(
                            &mut game.board,
                            depth,
                            is_white,
                            &mut stats,
                            &config,
                            &service,
                            &engine_state,
                            &mut local_map,
                        );

                        if search_result.completed {
                            best_result = Some(search_result.clone());
                            service.stdout.write(&service.uci_parser.get_info_str(&search_result, &stats));
                        }

                        if engine_state.stop_flag.load(Ordering::SeqCst) { break; }
                    }
                    if let Some(res) = best_result {
                        stdout.write(&format!("bestmove {}", res.get_best_move_algebraic()));
                        game.do_move(&res.get_best_move_algebraic());
                    }
                }

                else if command.starts_with("go") {
                    logger.send("incomming go cmd".to_string()).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);

                    engine_state.stop_flag.store(false, Ordering::SeqCst);
                    
                    // try to find book move
                    let white = game.white_to_move();        
                    let game_fen = service.fen.get_fen(&game.board);
                    let book_move = book.get_random_book_move(&game_fen);
                    let time_info = uci_parser.parse_go(command.as_str());

                    if book_move.is_empty() || !config.use_book {
                        // Spawn background timer observer thread
                        let engine_state_timer = engine_state.clone();
                        let time_info_observer = time_info.clone();
                        let config_timer = config.clone();
                        let white_timer = white;
                        let move_count_timer = game.board.move_count;

                        let _time_observer_thread = thread::spawn(move || {
                            let timer_logger = engine_state_timer.log_sender.clone();
                            if time_info_observer.time_mode == TimeMode::None || time_info_observer.time_mode == TimeMode::Depth {
                                timer_logger.send(format!("TimeMode is {:?}", time_info_observer.time_mode)).ok();
                                return;
                            }

                            let my_thinking_time = calculate_thinking_time(&time_info_observer, white_timer, move_count_timer, &config_timer);
                            timer_logger.send(format!("My thinking time is: {}", my_thinking_time)).ok();

                            thread::sleep(Duration::from_millis(my_thinking_time));
                            engine_state_timer.stop_flag.store(true, Ordering::SeqCst);
                            timer_logger.send(format!("Time up. Set stop flag true")).ok();
                        });

                        let mut local_map = local_map.clone();
                        local_map.insert(DataMapKey::CalcTime, Instant::now());
                        local_map.insert(DataMapKey::WhiteGivesCheck, false);
                        local_map.insert(DataMapKey::BlackGivesCheck, false);

                        // Clear old PV data
                        engine_state.pv_nodes.lock().unwrap().clear();
                        engine_state.pv_nodes_len.store(0, Ordering::SeqCst);

                        let mut best_result: Option<SearchResult> = None;
                        let max_depth = config.max_depth;

                        for depth in 2..=max_depth {
                            if engine_state.stop_flag.load(Ordering::SeqCst) {
                                break;
                            }

                            logger.send(format!("Start search on level {}", depth)).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);

                            let mut stats = Stats::default();
                            let is_white = game.board.white_to_move;

                            let search_result = service.search.get_moves(
                                &mut game.board,
                                depth,
                                is_white,
                                &mut stats,
                                &config,
                                &service,
                                &engine_state,
                                &mut local_map,
                            );

                            if search_result.completed {
                                best_result = Some(search_result.clone());
                                service.stdout.write(&service.uci_parser.get_info_str(&search_result, &stats));

                                // Update PV nodes in EngineState
                                let mut pv_guard = engine_state.pv_nodes.lock().unwrap();
                                pv_guard.clear();
                                let mut old_board = game.board.clone();
                                for turn in search_result.get_pv_move_row() {
                                    let hash = zobrist::gen(&old_board);
                                    pv_guard.insert(hash, turn.clone());
                                    old_board.do_move(&turn);
                                }
                                engine_state.pv_nodes_len.store(search_result.calculated_depth, Ordering::SeqCst);
                            }

                            // If depth limit is hit, break
                            if time_info.time_mode == TimeMode::Depth && depth >= time_info.depth {
                                break;
                            }

                            // Break early if we found mate
                            if let Some(ref res) = best_result {
                                if res.get_eval().abs() > 32000 {
                                    logger.send("found mate. stopping search".to_string()).ok();
                                    break;
                                }
                            }
                        }

                        // Print the best move
                        if let Some(res) = best_result {
                            stdout.write(&format!("bestmove {}", res.get_best_move_algebraic()));
                            game.do_move(&res.get_best_move_algebraic());
                            logger.send(format!("final move: bestmove {}", res.get_best_move_algebraic())).ok();


                        } else {
                            // Fallback if search was immediately aborted
                            let mut stats = Stats::default();
                            let history_table = [[0u32; 64]; 64];
                            let context = crate::model::SearchContext {
                                zobrist_table: &engine_state.zobrist_table,
                                stop_flag: &engine_state.stop_flag,
                                pv_nodes: &engine_state.pv_nodes,
                                killer_moves: [None; 2],
                                history_table: &history_table,
                            };
                            let valid_moves = service.move_gen.generate_valid_moves_list(&mut game.board, &mut stats, &config, &context, &local_map);
                            if let Some(first_move) = valid_moves.first() {
                                let mv_str = first_move.to_algebraic();
                                stdout.write(&format!("bestmove {}", mv_str));
                                game.do_move(&mv_str);
                            } else {
                                stdout.write("bestmove 0000");
                            }
                        }
                    } else { // do book move
                        logger.send(format!("found Book move: {} for position {}", book_move, game_fen))
                            .expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                        game.do_move(book_move);
                        stdout.write(&format!("bestmove {}", book_move));
                    }
                }
            }

            Err(_) => {
                break;
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