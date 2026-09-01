use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::sync::atomic::Ordering;


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
    let mut book = Book::new();
    let logger = engine_state.log_sender.clone();
    let mut active_config = config.clone();

    while let Ok(command) = rx_game_command.recv() {
        if command.trim() == "ucinewgame" {
            game = UciGame::new(service.fen.set_init_board());
            engine_state.stop_flag.store(false, Ordering::SeqCst);
            engine_state.pv_nodes.lock().unwrap().clear();
            engine_state.pv_nodes_len.store(0, Ordering::SeqCst);
            service.pawn_table.clear();
            engine_state.zobrist_table.read().unwrap().clear();
            logger.send("Start new Game".to_string()).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
            continue;
        }

                else if command.starts_with("setoption") {
                    let parts: Vec<&str> = command.split_whitespace().collect();
                    if let Some(name_idx) = parts.iter().position(|&r| r.to_lowercase() == "name") {
                        if let Some(val_idx) = parts.iter().position(|&r| r.to_lowercase() == "value") {
                            let param_name = parts[name_idx+1..val_idx].join(" ");
                            let val_str = parts[val_idx+1..].join(" ");
                            let effect = active_config.apply_uci_option(&param_name, &val_str);
                            if effect == crate::config::UciOptionEffect::Unknown {
                                logger.send(format!("Unknown option ignored: {} = {}\n", param_name, val_str)).ok();
                            } else {
                                logger.send(format!("Received option: {} = {}\n", param_name, val_str)).ok();
                            }
                            // The three options that invalidate a loaded book. `Config` does not
                            // own the book, so it reports the effect and the reaction lives here.
                            match effect {
                                crate::config::UciOptionEffect::BookFileChanged => {
                                    book.clear_polyglot_cache();
                                    // Load eagerly: a book that was named and cannot be read has to
                                    // fail here, at the handshake, and not silently turn into a
                                    // searched move in the middle of a game.
                                    book.preload_or_exit(&active_config, Some(&logger));
                                }
                                crate::config::UciOptionEffect::BookEnabledChanged => {
                                    // The book may have been named before it was switched on.
                                    book.preload_or_exit(&active_config, Some(&logger));
                                }
                                crate::config::UciOptionEffect::BookCacheChanged => {
                                    if !active_config.cache_book_in_ram {
                                        book.clear_polyglot_cache();
                                    }
                                }
                                crate::config::UciOptionEffect::Stored
                                | crate::config::UciOptionEffect::Unknown => {}
                            }
                        }
                    }
                }

                else if command.starts_with("board") {
                    let fen = command[6..].to_string();
                    game = UciGame::new(service.fen.set_fen(&fen));
                }

                else if let Some(moves_str) = command.strip_prefix("moves") {
                    if command.len() <= 5 {
                        continue;
                    }
                    let moves_iter = moves_str.split_whitespace();
                    for mv in moves_iter {
                        game.do_move(mv);
                    }                   
                }

                else if command == "infinite" {
                    engine_state.stop_flag.store(false, Ordering::SeqCst);

                    let mut best_result: Option<SearchResult> = None;
                    // Score of the last completed iteration, used to seed the aspiration window.
                    let mut prev_score: Option<i16> = None;
                    for depth in 2..100 {
                        if engine_state.stop_flag.load(Ordering::SeqCst) {
                            break;
                        }

                        logger.send(format!("Start Level {}", depth)).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);

                        let is_white = game.board.white_to_move;
                        let mut stats = Stats::default();
                        let search_result = service.search.get_moves(&mut game.board, depth, is_white, &mut stats, &active_config, service, &engine_state, std::time::Instant::now(), None, prev_score);

                        if search_result.completed {
                            prev_score = Some(search_result.get_eval());
                            best_result = Some(search_result.clone());
                            service.stdout.write(&service.uci_parser.get_info_str(&search_result, &stats));

                            let mut stats_calc = stats.clone();
                            stats_calc.calculate();
                            let cp = if is_white { search_result.get_eval() } else { -search_result.get_eval() };
                            let score_str = service.uci_parser.format_score(cp);
                            let nps = if stats_calc.calc_time_ms > 0 {
                                (stats_calc.created_nodes as u64 * 1000) / (stats_calc.calc_time_ms as u64)
                            } else {
                                stats_calc.created_nodes as u64 * 1000
                            };
                            logger.send(format!(
                                "Depth {:2} completed | score {:>8} | time {:>4}ms | nodes {:>8} | nps {:>8} | pv {}",
                                search_result.calculated_depth,
                                score_str,
                                stats_calc.calc_time_ms,
                                stats_calc.created_nodes,
                                nps,
                                search_result.get_best_move_row()
                            )).ok();
                        }

                        if engine_state.stop_flag.load(Ordering::SeqCst) { break; }
                    }
                    if let Some(res) = best_result {
                        stdout.write(&format!("bestmove {}", res.get_best_move_algebraic()));
                        game.do_move(&res.get_best_move_algebraic());
                    }
                }

                else if command.starts_with("go") {
                    logger.send("Incoming go command".to_string()).ok();

                    engine_state.stop_flag.store(false, Ordering::SeqCst);
                    
                    let white = game.white_to_move();        
                    let game_fen = service.fen.get_fen(&game.board);
                    let book_ply = game.made_moves_str.split_whitespace().count();
                    let book_move = book.get_book_move(&game.board, &game_fen, book_ply, &active_config, Some(&logger));
                    let time_info = uci_parser.parse_go(command.as_str());

                    if book_move.is_empty() {

                        let mut stats = Stats::default();
                        let history_table = [[0u32; 64]; 64];
                        let current_zobrist_table_1 = engine_state.zobrist_table.read().unwrap().clone();
                        let context = crate::model::SearchContext {
                            zobrist_table: &current_zobrist_table_1,
                            stop_flag: &engine_state.stop_flag,
                            pv_nodes: &engine_state.pv_nodes,
                            killer_moves: [None; 2],
                            history_table: &history_table,
                            counter_move: None,
                            start_time: std::time::Instant::now(),
                            target_time: None,
                            root_moves_total: 0,
                            root_moves_searched: 0,
                            root_depth: 0,
                        };
                        let mut valid_moves = crate::model::MoveList::new();
                        service.move_gen.generate_valid_moves_list(&mut game.board, &mut stats, &active_config, &context, true, &mut valid_moves);

                        if valid_moves.len == 0 {
                            logger.send("No valid moves found at root! Game over.".to_string()).ok();
                            stdout.write("bestmove 0000");
                            continue;
                        }

                        if valid_moves.len == 1 {
                            let mv_str = valid_moves.moves[0].to_algebraic();
                            stdout.write(&format!("bestmove {}", mv_str));
                            game.do_move(&mv_str);
                            logger.send(format!("Only one legal move found. Playing bestmove: {}", mv_str)).ok();
                            continue;
                        }

                        let my_thinking_time = if time_info.time_mode == TimeMode::None || time_info.time_mode == TimeMode::Depth {
                            i32::MAX as u64
                        } else {
                            calculate_thinking_time(&time_info, white, game.board.move_count, &active_config)
                        };

                        logger.send(format!("My thinking time is: {}", my_thinking_time)).ok();

                        engine_state.pv_nodes.lock().unwrap().clear();
                        engine_state.pv_nodes_len.store(0, Ordering::SeqCst);

                        let go_start_time = std::time::Instant::now();
                        let mut best_result: Option<SearchResult> = None;
                        let max_depth = active_config.max_depth;
                        // Score of the last completed iteration, used to seed the aspiration window.
                        let mut prev_score: Option<i16> = None;


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
                                &active_config,
                                service,
                                &engine_state,
                                go_start_time,
                                Some(my_thinking_time as i32),
                                prev_score,
                            );

                            if search_result.completed {
                                prev_score = Some(search_result.get_eval());
                                best_result = Some(search_result.clone());
                                service.stdout.write(&service.uci_parser.get_info_str(&search_result, &stats));

                                let mut stats_calc = stats.clone();
                                stats_calc.calculate();
                                let cp = if is_white { search_result.get_eval() } else { -search_result.get_eval() };
                                let score_str = service.uci_parser.format_score(cp);
                                let nps = if stats_calc.calc_time_ms > 0 {
                                    (stats_calc.created_nodes as u64 * 1000) / (stats_calc.calc_time_ms as u64)
                                } else {
                                    stats_calc.created_nodes as u64 * 1000
                                };
                                logger.send(format!(
                                    "Depth {:2} completed | score {:>8} | time {:>4}ms | nodes {:>8} | nps {:>8} | pv {}",
                                    search_result.calculated_depth,
                                    score_str,
                                    stats_calc.calc_time_ms,
                                    stats_calc.created_nodes,
                                    nps,
                                    search_result.get_best_move_row()
                                )).ok();

                                let mut pv_guard = engine_state.pv_nodes.lock().unwrap();
                                pv_guard.clear();
                                let mut old_board = game.board.clone();
                                for turn in search_result.get_pv_move_row() {
                                    let hash = zobrist::gen_hash(&old_board);
                                    pv_guard.insert(hash, turn);
                                    old_board.do_move(&turn);
                                }
                                engine_state.pv_nodes_len.store(search_result.calculated_depth, Ordering::SeqCst);
                            }

                            if time_info.time_mode == TimeMode::Depth && depth >= time_info.depth {
                                break;
                            }

                            if let Some(ref res) = best_result {
                                if res.get_eval().abs() > 32000 {
                                    logger.send("found mate. stopping search".to_string()).ok();
                                    break;
                                }
                            }
                        }

                        if let Some(res) = best_result {
                            stdout.write(&format!("bestmove {}", res.get_best_move_algebraic()));
                            game.do_move(&res.get_best_move_algebraic());
                            logger.send(format!(
                                "final move: bestmove {} (total time: {}ms)",
                                res.get_best_move_algebraic(),
                                go_start_time.elapsed().as_millis()
                            )).ok();

                        } else {
                            let mut stats = Stats::default();
                            let history_table = [[0u32; 64]; 64];
                            let current_zobrist_table_2 = engine_state.zobrist_table.read().unwrap().clone();
                            let context = crate::model::SearchContext {
                                zobrist_table: &current_zobrist_table_2,
                                stop_flag: &engine_state.stop_flag,
                                pv_nodes: &engine_state.pv_nodes,
                                killer_moves: [None; 2],
                                history_table: &history_table,
                                counter_move: None,
                                start_time: std::time::Instant::now(),
                                target_time: None,
                                root_moves_total: 0,
                                root_moves_searched: 0,
                                root_depth: 0,
                            };
                            let mut valid_moves = crate::model::MoveList::new();
                            service.move_gen.generate_valid_moves_list(&mut game.board, &mut stats, &active_config, &context, true, &mut valid_moves);
                            if let Some(first_move) = valid_moves.as_slice().first() {
                                let mv_str = first_move.to_algebraic();
                                stdout.write(&format!("bestmove {}", mv_str));
                                game.do_move(&mv_str);
                            } else {
                                stdout.write("bestmove 0000");
                            }
                        }
                    } else {
                        logger.send(format!("found Book move: {} for position {}", book_move, game_fen))
                            .expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                        game.do_move(&book_move);
                        stdout.write(&format!("bestmove {}", book_move));
                    }
                }
    }
}


fn calculate_thinking_time(time_info: &TimeInfo, white: bool, move_count: i32, config: &Config) -> u64 {
    let mut my_time = if white { time_info.wtime } else { time_info.btime };
    my_time = my_time.saturating_sub(config.move_overhead as i32);

    let thinking_time = match time_info.time_mode {
        TimeMode::None => 2000,
        
        TimeMode::Movetime => {
            (my_time - 50).max(10)
        }
        
        TimeMode::MoveToGo => {
            let my_thinking_time = (my_time / (time_info.moves_to_go + 1)) + (if white { time_info.winc } else { time_info.binc });
            
            if my_thinking_time > my_time { // when increment is bigger then current time left
                (my_time - 1000).max(10)
            } else {
                my_thinking_time.max(10)
            }
        }
        
        TimeMode::HourGlas => {
            let my_thinking_time = if move_count < 40 {
                (my_time as f64 * (0.02 + (move_count as f64 / 1000.0))) as i32
            } else {
                my_time / 20
            } + if white { time_info.winc } else { time_info.binc };

            if my_thinking_time > my_time { // when increment is bigger then current time left
                (my_time - 1000).max(10)
            } else {
                my_thinking_time.max(10)
            }
            
        }
        
        TimeMode::Depth => {
            0
        }
    };

    let thinking_time = thinking_time.max(10);
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