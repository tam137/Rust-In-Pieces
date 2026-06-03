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
    let book = Book::new();
    let logger = engine_state.log_sender.clone();
    let mut active_config = config.clone();

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

                else if command.starts_with("setoption") {
                    let parts: Vec<&str> = command.split_whitespace().collect();
                    if let Some(name_idx) = parts.iter().position(|&r| r.to_lowercase() == "name") {
                        if let Some(val_idx) = parts.iter().position(|&r| r.to_lowercase() == "value") {
                            let param_name = parts[name_idx+1..val_idx].join("_").to_lowercase()
                                .replace("enablepositionalcap", "enable_positional_cap")
                                .replace("positionalcapdamping", "positional_cap_damping");
                            let val_str = parts[val_idx+1..].join(" ");

                            if param_name == "aggressiveness" {
                                if val_str.to_lowercase().contains("high") {
                                    active_config.aggressiveness = crate::config::Aggressiveness::HighAggressive;
                                } else if val_str.to_lowercase().contains("aggressive") {
                                    active_config.aggressiveness = crate::config::Aggressiveness::Aggressive;
                                } else {
                                    active_config.aggressiveness = crate::config::Aggressiveness::Normal;
                                }
                            } else if param_name == "enable_positional_cap" {
                                active_config.enable_positional_cap = val_str.to_lowercase() == "true";
                            } else if param_name == "move_overhead" {
                                if let Ok(overhead) = val_str.parse::<u64>() { active_config.move_overhead = overhead; }
                            } else {
                                match param_name.as_str() {
                                    "nmp_depth_threshold" => if let Ok(v) = val_str.parse::<i32>() { active_config.nmp_depth_threshold = v; },
                                    "nmp_reduction" => if let Ok(v) = val_str.parse::<i32>() { active_config.nmp_reduction = v; },
                                    "nmp_verification_threshold" => if let Ok(v) = val_str.parse::<i32>() { active_config.nmp_verification_threshold = v; },
                                    "nmp_dynamic_divisor" => if let Ok(v) = val_str.parse::<i32>() { active_config.nmp_dynamic_divisor = v; },
                                    "lmr_move_threshold" => if let Ok(v) = val_str.parse::<i32>() { active_config.lmr_move_threshold = v; },
                                    "killer_move_1_rank_bonus" => if let Ok(v) = val_str.parse::<i32>() { active_config.killer_move_1_rank_bonus = v; },
                                    "killer_move_2_rank_bonus" => if let Ok(v) = val_str.parse::<i32>() { active_config.killer_move_2_rank_bonus = v; },
                                    "counter_move_rank_bonus" => if let Ok(v) = val_str.parse::<i32>() { active_config.counter_move_rank_bonus = v; },
                                    "is_hashed_rank_bonus" => if let Ok(v) = val_str.parse::<i32>() { active_config.is_hashed_rank_bonus = v; },
                                    "give_check_rank_bonus" => if let Ok(v) = val_str.parse::<i32>() { active_config.give_check_rank_bonus = v; },
                                    "is_pv_node_rank_bonus" => if let Ok(v) = val_str.parse::<i32>() { active_config.is_pv_node_rank_bonus = v; },
                                    "give_promotion_rank_bonus_queen" => if let Ok(v) = val_str.parse::<i32>() { active_config.give_promotion_rank_bonus_queen = v; },
                                    "give_promotion_rank_bonus_knight" => if let Ok(v) = val_str.parse::<i32>() { active_config.give_promotion_rank_bonus_knight = v; },
                                    "history_max_threshold" => if let Ok(v) = val_str.parse::<u32>() { active_config.history_max_threshold = v; },
                                    "your_turn_bonus" => if let Ok(v) = val_str.parse::<i16>() { active_config.your_turn_bonus = v; },
                                    "aggressiveness" => match val_str.as_str() {
                                        "Normal" => active_config.aggressiveness = crate::config::Aggressiveness::Normal,
                                        "Aggressive" => active_config.aggressiveness = crate::config::Aggressiveness::Aggressive,
                                        "HighAggressive" => active_config.aggressiveness = crate::config::Aggressiveness::HighAggressive,
                                        _ => {}
                                    },
                                    "enablepositionalcap" | "enable_positional_cap" => {
                                        active_config.enable_positional_cap = val_str == "true";
                                    },
                                    "positionalcapdamping" | "positional_cap_damping" => {
                                        if let Ok(v) = val_str.parse::<i16>() { active_config.positional_cap_damping = v; }
                                    },
                                    "moveoverhead" | "move_overhead" => {
                                        if let Ok(v) = val_str.parse::<u64>() { active_config.move_overhead = v; }
                                    },
                                    "kingopenfilemalus" => if let Ok(v) = val_str.parse::<i16>() { active_config.king_open_file_malus = v; },
                                    "kinghalfopenfilemalus" => if let Ok(v) = val_str.parse::<i16>() { active_config.king_half_open_file_malus = v; },
                                    "kingringdefendervalue" => if let Ok(v) = val_str.parse::<i16>() { active_config.king_ring_defender_value = v; },
                                    "threatminorattacksrook" => if let Ok(v) = val_str.parse::<i16>() { active_config.threat_minor_attacks_rook = v; },
                                    "threatminorattacksqueen" => if let Ok(v) = val_str.parse::<i16>() { active_config.threat_minor_attacks_queen = v; },
                                    "threatrookattacksqueen" => if let Ok(v) = val_str.parse::<i16>() { active_config.threat_rook_attacks_queen = v; },
                                    "pawn_structure" => if let Ok(v) = val_str.parse::<i16>() { active_config.pawn_structure = v; },
                                    "pawn_supports_knight_outpost" => if let Ok(v) = val_str.parse::<i16>() { active_config.pawn_supports_knight_outpost = v; },
                                    "pawn_centered" => if let Ok(v) = val_str.parse::<i16>() { active_config.pawn_centered = v; },
                                    "pawn_undeveloped_malus" => if let Ok(v) = val_str.parse::<i16>() { active_config.pawn_undeveloped_malus = v; },
                                    "pawn_on_last_rank_bonus" => if let Ok(v) = val_str.parse::<i16>() { active_config.pawn_on_last_rank_bonus = v; },
                                    "pawn_on_before_last_rank_bonus" => if let Ok(v) = val_str.parse::<i16>() { active_config.pawn_on_before_last_rank_bonus = v; },
                                    "pawn_on_before_before_last_rank_bonus" => if let Ok(v) = val_str.parse::<i16>() { active_config.pawn_on_before_before_last_rank_bonus = v; },
                                    "pawn_defends_bishop" => if let Ok(v) = val_str.parse::<i16>() { active_config.pawn_defends_bishop = v; },
                                    "pawn_double_malus" => if let Ok(v) = val_str.parse::<i16>() { active_config.pawn_double_malus = v; },
                                    "pawn_isolated_malus" => if let Ok(v) = val_str.parse::<i16>() { active_config.pawn_isolated_malus = v; },
                                    "pawn_backward_malus" => if let Ok(v) = val_str.parse::<i16>() { active_config.pawn_backward_malus = v; },
                                    "protected_passed_pawn_middlegame" => if let Ok(v) = val_str.parse::<i16>() { active_config.protected_passed_pawn_middlegame = v; },
                                    "protected_passed_pawn_endgame" => if let Ok(v) = val_str.parse::<i16>() { active_config.protected_passed_pawn_endgame = v; },
                                    "undeveloped_knight_malus" => if let Ok(v) = val_str.parse::<i16>() { active_config.undeveloped_knight_malus = v; },
                                    "knight_on_rim_malus" => if let Ok(v) = val_str.parse::<i16>() { active_config.knight_on_rim_malus = v; },
                                    "knight_centered" => if let Ok(v) = val_str.parse::<i16>() { active_config.knight_centered = v; },
                                    "knight_blockes_pawn" => if let Ok(v) = val_str.parse::<i16>() { active_config.knight_blockes_pawn = v; },
                                    "knight_mobility_factor" => if let Ok(v) = val_str.parse::<i16>() { active_config.knight_mobility_factor = v; },
                                    "undeveloped_bishop_malus" => if let Ok(v) = val_str.parse::<i16>() { active_config.undeveloped_bishop_malus = v; },
                                    "bishop_pair_bonus" => if let Ok(v) = val_str.parse::<i16>() { active_config.bishop_pair_bonus = v; },
                                    "bishop_trapped_at_rim_malus" => if let Ok(v) = val_str.parse::<i16>() { active_config.bishop_trapped_at_rim_malus = v; },
                                    "bishop_mobility_factor" => if let Ok(v) = val_str.parse::<i16>() { active_config.bishop_mobility_factor = v; },
                                    "rook_open_file" => if let Ok(v) = val_str.parse::<i16>() { active_config.rook_open_file = v; },
                                    "rook_half_open_file" => if let Ok(v) = val_str.parse::<i16>() { active_config.rook_half_open_file = v; },
                                    "rook_doubled_bonus" => if let Ok(v) = val_str.parse::<i16>() { active_config.rook_doubled_bonus = v; },
                                    "rook_behind_passed_pawn_middlegame" => if let Ok(v) = val_str.parse::<i16>() { active_config.rook_behind_passed_pawn_middlegame = v; },
                                    "rook_behind_passed_pawn_endgame" => if let Ok(v) = val_str.parse::<i16>() { active_config.rook_behind_passed_pawn_endgame = v; },
                                    "rook_on_seventh" => if let Ok(v) = val_str.parse::<i16>() { active_config.rook_on_seventh = v; },
                                    "rook_mobility_factor" => if let Ok(v) = val_str.parse::<i16>() { active_config.rook_mobility_factor = v; },
                                    "undeveloped_king_malus" => if let Ok(v) = val_str.parse::<i16>() { active_config.undeveloped_king_malus = v; },
                                    "king_ring_attack_knight" => if let Ok(v) = val_str.parse::<i16>() { active_config.king_ring_attack_knight = v; },
                                    "king_ring_attack_bishop" => if let Ok(v) = val_str.parse::<i16>() { active_config.king_ring_attack_bishop = v; },
                                    "king_ring_attack_rook" => if let Ok(v) = val_str.parse::<i16>() { active_config.king_ring_attack_rook = v; },
                                    "king_ring_attack_queen" => if let Ok(v) = val_str.parse::<i16>() { active_config.king_ring_attack_queen = v; },
                                    "king_opposition_bonus" => if let Ok(v) = val_str.parse::<i16>() { active_config.king_opposition_bonus = v; },
                                    "king_pawn_shield" => if let Ok(v) = val_str.parse::<i16>() { active_config.king_pawn_shield = v; },
                                    "king_piece_shield" => if let Ok(v) = val_str.parse::<i16>() { active_config.king_piece_shield = v; },
                                    "king_trapp_at_baseline_malus" => if let Ok(v) = val_str.parse::<i16>() { active_config.king_trapp_at_baseline_malus = v; },
                                    "king_in_check_malus" => if let Ok(v) = val_str.parse::<i16>() { active_config.king_in_check_malus = v; },
                                    "king_in_double_check_malus" => if let Ok(v) = val_str.parse::<i16>() { active_config.king_in_double_check_malus = v; },
                                    "pawn_attacks_opponent_fig" => if let Ok(v) = val_str.parse::<i16>() { active_config.pawn_attacks_opponent_fig = v; },
                                    "pawn_attacks_opponent_fig_with_tempo" => if let Ok(v) = val_str.parse::<i16>() { active_config.pawn_attacks_opponent_fig_with_tempo = v; },
                                    "queen_in_attack" => if let Ok(v) = val_str.parse::<i16>() { active_config.queen_in_attack = v; },
                                    "queen_in_attack_with_tempo" => if let Ok(v) = val_str.parse::<i16>() { active_config.queen_in_attack_with_tempo = v; },
                                    "knight_attacks_bishop" => if let Ok(v) = val_str.parse::<i16>() { active_config.knight_attacks_bishop = v; },
                                    "knight_attacks_rook" => if let Ok(v) = val_str.parse::<i16>() { active_config.knight_attacks_rook = v; },
                                    "knight_attacks_bishop_tempo" => if let Ok(v) = val_str.parse::<i16>() { active_config.knight_attacks_bishop_tempo = v; },
                                    "knight_attacks_rook_tempo" => if let Ok(v) = val_str.parse::<i16>() { active_config.knight_attacks_rook_tempo = v; },
                                    "delta_pruning_margin" => if let Ok(v) = val_str.parse::<i16>() { active_config.delta_pruning_margin = v; },
                                    _ => {}
                                }
                            }
                            logger.send(format!("setoption: {} updated to {}", param_name, val_str)).ok();
                        }
                    }
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

                    let mut best_result: Option<SearchResult> = None;
                    for depth in 2..100 {
                        if engine_state.stop_flag.load(Ordering::SeqCst) {
                            break;
                        }

                        logger.send(format!("Start Level {}", depth)).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);

                        let is_white = game.board.white_to_move;
                        let mut stats = Stats::default();
                        let search_result = service.search.get_moves(&mut game.board, depth, is_white, &mut stats, &active_config, &service, &engine_state, std::time::Instant::now(), None);

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
                    
                    let white = game.white_to_move();        
                    let game_fen = service.fen.get_fen(&game.board);
                    let book_move = book.get_random_book_move(&game_fen);
                    let time_info = uci_parser.parse_go(command.as_str());

                    if book_move.is_empty() || !active_config.use_book {

                        let mut stats = Stats::default();
                        let history_table = [[0u32; 64]; 64];
                        let current_zobrist_table_1 = engine_state.zobrist_table.read().unwrap().clone();
                        let context = crate::model::SearchContext {
                            zobrist_table: &*current_zobrist_table_1,
                            stop_flag: &engine_state.stop_flag,
                            pv_nodes: &engine_state.pv_nodes,
                            killer_moves: [None; 2],
                            history_table: &history_table,
                            counter_move: None,
                            start_time: std::time::Instant::now(),
                            target_time: None,
                            root_moves_total: 0,
                            root_moves_searched: 0,
                        };
                        let mut valid_moves = crate::model::MoveList::new();
                        service.move_gen.generate_valid_moves_list(&mut game.board, &mut stats, &active_config, &context, true, false, &mut valid_moves);

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
                                &service,
                                &engine_state,
                                go_start_time,
                                Some(my_thinking_time as i32),
                            );

                            if search_result.completed {
                                best_result = Some(search_result.clone());
                                service.stdout.write(&service.uci_parser.get_info_str(&search_result, &stats));



                                let mut pv_guard = engine_state.pv_nodes.lock().unwrap();
                                pv_guard.clear();
                                let mut old_board = game.board.clone();
                                for turn in search_result.get_pv_move_row() {
                                    let hash = zobrist::gen_hash(&old_board);
                                    pv_guard.insert(hash, turn.clone());
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
                            logger.send(format!("final move: bestmove {}", res.get_best_move_algebraic())).ok();

                        } else {
                            let mut stats = Stats::default();
                            let history_table = [[0u32; 64]; 64];
                            let current_zobrist_table_2 = engine_state.zobrist_table.read().unwrap().clone();
                            let context = crate::model::SearchContext {
                                zobrist_table: &*current_zobrist_table_2,
                                stop_flag: &engine_state.stop_flag,
                                pv_nodes: &engine_state.pv_nodes,
                                killer_moves: [None; 2],
                                history_table: &history_table,
                                counter_move: None,
                                start_time: std::time::Instant::now(),
                                target_time: None,
                                root_moves_total: 0,
                                root_moves_searched: 0,
                            };
                            let mut valid_moves = crate::model::MoveList::new();
                            service.move_gen.generate_valid_moves_list(&mut game.board, &mut stats, &active_config, &context, true, false, &mut valid_moves);
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