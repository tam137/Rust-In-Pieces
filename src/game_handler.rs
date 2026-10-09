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
            // Killers, history and counter moves persist across the iterative deepening loop
            // and across the moves of a game (`task.md` 23.1); a new game is where they go.
            engine_state.search_tables.lock().unwrap().reset();
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
                    replay_moves(&mut game, moves_str);
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
                    let book_move = if book_move.is_empty()
                        || is_playable_book_move(service, &game.board, &book_move, &active_config, &engine_state) {
                        book_move
                    } else {
                        logger.send(format!("book move {} rejected: illegal here or a threefold", book_move)).ok();
                        String::new()
                    };
                    let time_info = uci_parser.parse_go(command.as_str());

                    if book_move.is_empty() {

                        let mut stats = Stats::default();
                        let history_table = [[[0i32; 64]; 64]; 2];
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
                            } else if let Some(previous_best) = best_result.as_ref()
                                .and_then(|previous| previous.variants.first())
                                .and_then(|variant| variant.best_move)
                            {
                                if proven_better_while_interrupted(&previous_best, &search_result) {
                                    logger.send(format!(
                                        "interrupted depth {} already proved {} over {}",
                                        depth,
                                        search_result.get_best_move_algebraic(),
                                        previous_best.to_algebraic()
                                    )).ok();
                                    best_result = Some(search_result.clone());
                                }
                            }

                            if time_info.time_mode == TimeMode::Depth && depth >= time_info.depth {
                                break;
                            }

                            if let Some(ref res) = best_result {
                                if mate_within_horizon(res.get_eval(), res.calculated_depth) {
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
                            let history_table = [[[0i32; 64]; 64]; 2];
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


/// Whether an iteration the clock interrupted has already proven a root move better than the one
/// the last completed iteration chose, so that it is played instead.
///
/// Only completed iterations used to count, and the last one, nearly always cut by the clock, was
/// thrown away whole: 37.8% of all search time at 1s + 150ms. `get_moves` stops its root loop only
/// between moves, so every variant of an interrupted pass is a move searched to the new depth, and
/// a variant is recorded only when it beats every move before it. The previous best is ordered
/// first; when it is among the variants it was searched in this pass, and another move on top beat
/// it. That proves something only inside the pass's window: below `alpha` for White, or above
/// `beta` for Black, every score is a fail-low bound, and two bounds compared say nothing.
fn proven_better_while_interrupted(previous_best: &crate::model::Turn, result: &SearchResult) -> bool {
    let top = match result.variants.first() {
        Some(top) => top,
        None => return false,
    };
    if top.best_move.as_ref() == Some(previous_best) {
        return false;
    }
    if !result.variants.iter().any(|variant| variant.best_move.as_ref() == Some(previous_best)) {
        return false;
    }
    if result.is_white_move {
        top.eval > result.window_alpha
    } else {
        top.eval < result.window_beta
    }
}

/// Whether a mate score has been proven inside the completed depth, which is when iterative
/// deepening may stop for it.
///
/// Any mate score used to stop the search. Once a mate had been seen, the table carried mate
/// scores for the whole mating net, so on the following moves a depth-2 or depth-3 iteration
/// already read a mate through a table cutoff and ended the search there - and the move a
/// 2-ply search prefers among moves that all read "mate" is rarely the one that makes progress.
/// The stored distance does not shrink, nothing forced progress, and won endings went round in
/// circles into a threefold. A mate in `n` plies is proven once `n` plies have been searched.
fn mate_within_horizon(eval: i16, completed_depth: i32) -> bool {
    let score = eval.saturating_abs();
    score > crate::model::MATE_SCORE_THRESHOLD
        && (crate::model::MATE_SCORE - score) as i32 <= completed_depth
}

/// Plays the moves of a `position ... moves` list onto the game.
///
/// A threefold inside the list leaves `Draw` on the board, after which the root generates no move
/// and the engine answered `bestmove 0000`. The game goes on until someone claims the draw, so the
/// root is searched like any other position.
fn replay_moves(game: &mut UciGame, moves_str: &str) {
    for mv in moves_str.split_whitespace() {
        game.do_move(mv);
    }
    game.board.game_status = crate::model::GameStatus::Normal;
}

/// Whether `book_move` may be played on `board`: it has to be one of the legal moves, and it may
/// not complete a threefold. The book knows neither - a key collision or a corrupt user book can
/// name any move, and the embedded book repeats the Najdorf Poisoned Pawn line into a draw.
fn is_playable_book_move(service: &Service, board: &crate::model::Board, book_move: &str, config: &Config, engine_state: &EngineState) -> bool {
    if !crate::notation_util::NotationUtil::is_long_algebraic(book_move) {
        return false;
    }
    let mut stats = Stats::default();
    let history_table = [[[0i32; 64]; 64]; 2];
    let zobrist_table = engine_state.zobrist_table.read().unwrap().clone();
    let context = crate::model::SearchContext {
        zobrist_table: &zobrist_table,
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
    let mut probe = board.clone();
    let mut legal = crate::model::MoveList::new();
    service.move_gen.generate_valid_moves_list(&mut probe, &mut stats, config, &context, false, &mut legal);
    match legal.as_slice().iter().find(|turn| turn.to_algebraic() == book_move) {
        Some(turn) => {
            probe.do_move(turn);
            probe.game_status != crate::model::GameStatus::Draw
        }
        None => false,
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

    /// 1. Nf3 Nf6 2. Ng1 Ng8 twice and 5. Nf3: the position after 1. Nf3 for the third time.
    const THREEFOLD_AFTER_NF3: &str = "g1f3 g8f6 f3g1 f6g8 g1f3 g8f6 f3g1 f6g8 g1f3";

    fn engine_state() -> std::sync::Arc<crate::model::EngineState> {
        let (tx_log, _rx_log) = std::sync::mpsc::channel();
        std::sync::Arc::new(crate::model::EngineState {
            stop_flag: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            debug_flag: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            zobrist_table: std::sync::RwLock::new(std::sync::Arc::new(crate::zobrist::ZobristTable::with_capacity(1024))),
            pv_nodes: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            pv_nodes_len: std::sync::Arc::new(std::sync::atomic::AtomicI32::new(0)),
            logger: std::sync::Arc::new(std::sync::RwLock::new(std::sync::Arc::new(|_| {}))),
            log_sender: tx_log,
            search_tables: std::sync::Mutex::new(crate::model::SearchTables::new()),
        })
    }

    fn pass(white: bool, window: (i16, i16), variants: &[((u8, u8), i16)]) -> crate::model::SearchResult {
        let mut result = crate::model::SearchResult::default();
        result.is_white_move = white;
        result.window_alpha = window.0;
        result.window_beta = window.1;
        for &((from, to), eval) in variants {
            result.add_variant(crate::model::Variant {
                eval,
                best_move: Some(crate::model::Turn::new(from, to, 0, 0, false, 0)),
                move_row: std::collections::VecDeque::new(),
            });
        }
        result
    }

    #[test]
    fn test_an_interrupted_iteration_keeps_a_move_it_has_proven_better() {
        use super::proven_better_while_interrupted as proven;
        let previous = crate::model::Turn::new(12, 28, 0, 0, false, 0);
        let (a, b, c) = ((12, 28), (6, 21), (11, 27));

        // White: the previous best was searched first, a later move beat it inside the window.
        assert!(proven(&previous, &pass(true, (10, 30), &[(b, 35), (a, 20)])));
        // Black, on the absolute scale: lower is better, and the edge that counts is `beta`.
        assert!(proven(&previous, &pass(false, (-30, -10), &[(b, -35), (a, -20)])));

        // The previous best is still on top.
        assert!(!proven(&previous, &pass(true, (10, 30), &[(a, 25), (b, 15)])));
        // The previous best was not searched in this pass, so nothing was beaten.
        assert!(!proven(&previous, &pass(true, (10, 30), &[(b, 35), (c, 20)])));
        // Every score failed low: two upper bounds compared with each other prove nothing.
        assert!(!proven(&previous, &pass(true, (50, 70), &[(b, 40), (a, 30)])));
        assert!(!proven(&previous, &pass(false, (-70, -50), &[(b, -40), (a, -30)])));
        // Nothing finished at all.
        assert!(!proven(&previous, &pass(true, (10, 30), &[])));
    }

    #[test]
    fn test_a_mate_stops_the_search_only_inside_the_completed_depth() {
        use crate::model::MATE_SCORE;
        // Mate in five plies, for either side: proven at depth 5, not at depth 4 or 2.
        for sign in [1i16, -1] {
            assert!(!super::mate_within_horizon(sign * (MATE_SCORE - 5), 2));
            assert!(!super::mate_within_horizon(sign * (MATE_SCORE - 5), 4));
            assert!(super::mate_within_horizon(sign * (MATE_SCORE - 5), 5));
            assert!(super::mate_within_horizon(sign * (MATE_SCORE - 1), 2));
        }
        assert!(!super::mate_within_horizon(950, 30), "a centipawn score is not a mate");
    }

    #[test]
    fn test_a_replayed_threefold_leaves_the_root_searchable() {
        let service = crate::service::Service::new();
        let mut game = crate::model::UciGame::new(service.fen.set_init_board());
        super::replay_moves(&mut game, THREEFOLD_AFTER_NF3);
        assert_eq!(game.board.game_status, crate::model::GameStatus::Normal,
            "the root must not keep the draw flag of the replayed list");
    }

    #[test]
    fn test_a_book_move_must_be_legal_and_must_not_repeat() {
        let service = crate::service::Service::new();
        let config = Config::new();
        let state = engine_state();
        let start = service.fen.set_init_board();
        assert!(super::is_playable_book_move(&service, &start, "e2e4", &config, &state));
        assert!(!super::is_playable_book_move(&service, &start, "e2e5", &config, &state), "illegal");
        assert!(!super::is_playable_book_move(&service, &start, "0000", &config, &state), "not a move");

        // One ply short of the threefold: the book move that completes it is refused, any other
        // legal move is not.
        let mut game = crate::model::UciGame::new(service.fen.set_init_board());
        super::replay_moves(&mut game, "g1f3 g8f6 f3g1 f6g8 g1f3 g8f6 f3g1 f6g8");
        assert!(!super::is_playable_book_move(&service, &game.board, "g1f3", &config, &state),
            "g1f3 repeats the position after 1. Nf3 a third time");
        assert!(super::is_playable_book_move(&service, &game.board, "e2e4", &config, &state));
    }

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