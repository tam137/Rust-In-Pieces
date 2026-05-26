use std::collections::VecDeque;
use std::time::Instant;
use std::sync::Arc;

use crate::config::Config;
use crate::model::{Board, DataMap, DataMapKey, GameStatus, SearchResult, Stats, Turn, Variant, SearchContext, EngineState, RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE};
use crate::service::Service;

use crate::model::RIP_MISSED_DM_KEY;


pub struct SearchService;

impl SearchService {

    pub fn new() -> Self {
        SearchService
    }

    pub fn get_moves(
        &self,
        board: &mut Board,
        depth: i32,
        white: bool,
        stats: &mut Stats,
        config: &Config,
        service: &Service,
        engine_state: &Arc<EngineState>,
        local_map: &mut DataMap,
    ) -> SearchResult {
        let mut search_config = config.clone();
        search_config.pre_sort_moves = false;
        let config = &search_config;
        let logger = engine_state.log_sender.clone();

        // Always ensure that WhiteGivesCheck and BlackGivesCheck are initialized in local_map
        local_map.insert(DataMapKey::WhiteGivesCheck, false);
        local_map.insert(DataMapKey::BlackGivesCheck, false);

        let zobrist_table = &engine_state.zobrist_table;
        let stop_flag = &engine_state.stop_flag;
        let pv_nodes = &engine_state.pv_nodes;

        let mut killer_moves: [[Option<Turn>; 2]; 128] = [[None; 2]; 128];
        let mut history_table = [[0u32; 64]; 64];
        let mut counter_moves: [[Option<Turn>; 64]; 64] = [[None; 64]; 64];

        let context = SearchContext {
            zobrist_table,
            stop_flag,
            pv_nodes,
            killer_moves: [None; 2],
            history_table: &history_table,
            counter_move: None,
        };

        let mut turns = crate::model::MoveList::new();
        service.move_gen.generate_valid_moves_list(board, stats, config, &context, local_map, &mut turns);

        // Sort turns once by rank descending
        for i in 0..turns.len {
            let mut best_idx = i;
            for j in (i + 1)..turns.len {
                if turns.moves[j].rank > turns.moves[best_idx].rank {
                    best_idx = j;
                }
            }
            turns.moves.swap(i, best_idx);
        }

        let mut prev_eval = None;
        if depth > 2 && config.use_zobrist && config.enable_aspiration {
            if let Some(entry) = zobrist_table.get_entry(&board.cached_hash) {
                prev_eval = Some(entry.eval);
            }
        }

        let mut alpha: i16 = i16::MIN;
        let mut beta: i16 = i16::MAX;
        let mut delta = 15;

        if let Some(val) = prev_eval {
            // De-normalize mate score if present
            let mut val_de = val;
            if val > 30000 {
                val_de = 30000;
            } else if val < -30000 {
                val_de = -30000;
            }
            alpha = val_de.saturating_sub(delta);
            beta = val_de.saturating_add(delta);
        }

        let mut search_result;

        loop {
            search_result = SearchResult::default();
            search_result.calculated_depth = depth;
            search_result.is_white_move = white;
            search_result.is_pv_search_result = *local_map.get_data::<bool>(DataMapKey::PvFlag).unwrap_or_else(|| &false);

            let mut current_alpha = alpha;
            let mut current_beta = beta;

            let mut best_eval = if white { i16::MIN } else { i16::MAX };
            let total_root_moves = turns.len as i32;
            local_map.insert(DataMapKey::RootMovesTotal, total_root_moves);
            local_map.insert(DataMapKey::RootMovesSearched, 0);

            let mut turn_counter = 0;
            let mut child_pv = [None; 128];

            for i in 0..turns.len {
                let turn = &turns.moves[i];

                // Check time at the start of each root move
                let elapsed = self.get_calc_time(local_map) as i32;
                if let Some(&target) = local_map.get_data::<i32>(DataMapKey::TargetTime) {
                    let mut dynamic_target = target;
                    if target < i32::MAX - 1000000 {
                        if total_root_moves > 0 && (turn_counter * 100) / total_root_moves >= 85 {
                            dynamic_target = (target * 13) / 10;
                        }
                    }
                    if elapsed >= dynamic_target {
                        stop_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                }

                if stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                    let calc_time_ms = self.get_calc_time(local_map);
                    search_result.stats = stats.clone();
                    search_result.stats.best_turn_nr = turn_counter as i8;
                    search_result.stats.calc_time_ms = calc_time_ms as usize;
                    break;
                }

                turn_counter += 1;
                local_map.insert(DataMapKey::RootMovesSearched, turn_counter - 1);
                let mi = board.do_move(turn);

                let child_context = SearchContext {
                    zobrist_table: context.zobrist_table,
                    stop_flag: context.stop_flag,
                    pv_nodes: context.pv_nodes,
                    killer_moves: killer_moves[1],
                    history_table: &history_table,
                    counter_move: if config.enable_counter_moves {
                        counter_moves[turn.from as usize][turn.to as usize]
                    } else {
                        None
                    },
                };

                let min_max_result = self.minimax(board, turn, depth - 1, !white,
                    current_alpha, current_beta, stats, config, service, &child_context, local_map, &mut child_pv,
                    1, &mut killer_moves, &mut history_table, &mut counter_moves);

                if stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                    board.undo_move(turn, mi);
                    let calc_time_ms = self.get_calc_time(local_map);
                    search_result.stats = stats.clone();
                    search_result.stats.best_turn_nr = turn_counter as i8;
                    search_result.stats.calc_time_ms = calc_time_ms as usize;
                    break;
                }

                let min_max_eval = min_max_result.1;

                // save min max eval in zobrist table for better move sorting, if depth = 2
                if depth == 2 && config.use_zobrist {
                    let mut stored_eval = min_max_eval;
                    if min_max_eval > 30000 {
                        stored_eval = min_max_eval + 1;
                    } else if min_max_eval < -30000 {
                        stored_eval = min_max_eval - 1;
                    }
                    zobrist_table.insert_entry(board.cached_hash, crate::zobrist::TranspositionEntry {
                        key: board.cached_hash,
                        eval: stored_eval,
                        depth: 1,
                        entry_type: crate::zobrist::TranspositionType::Exact,
                        best_move: crate::zobrist::TranspositionEntry::compress_move(min_max_result.0),
                        padding: [0; 2],
                    });
                }

                board.undo_move(turn, mi);
                if white {
                    if min_max_eval > best_eval {
                        best_eval = min_max_eval;
                        current_alpha = current_alpha.max(min_max_eval);
                        let mut best_move_row = VecDeque::new();
                        best_move_row.push_back(Some(*turn));
                        for mv in child_pv.iter().take_while(|x| x.is_some()) {
                            best_move_row.push_back(*mv);
                        }
                        search_result.add_variant(Variant { best_move: Some(*turn), move_row: best_move_row, eval: min_max_eval });
                        search_result.is_white_move = white;
                        search_result.variants.sort_by(|a, b| b.eval.cmp(&a.eval)); // Highest first for white
                        stats.best_turn_nr = turn_counter as i8;
                        let calc_time_ms = self.get_calc_time(local_map);
                        stats.calc_time_ms = calc_time_ms as usize;
                        stats.calculate();
                        if config.print_info_string_during_search {
                            if let Err(_e) = service.stdout.write_get_result(&service.uci_parser.get_info_str(&search_result, stats)) {
                                logger.send("stdout channel closed during search".to_string())
                                    .expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                                break;
                            }
                        }
                    }
                } else {
                    if min_max_eval < best_eval {
                        best_eval = min_max_eval;
                        current_beta = current_beta.min(min_max_eval);
                        let mut best_move_row = VecDeque::new();
                        best_move_row.push_back(Some(*turn));
                        for mv in child_pv.iter().take_while(|x| x.is_some()) {
                            best_move_row.push_back(*mv);
                        }
                        search_result.add_variant(Variant { best_move: Some(*turn), move_row: best_move_row, eval: min_max_eval });
                        search_result.is_white_move = white;
                        search_result.variants.sort_by(|a, b| a.eval.cmp(&b.eval)); // Lowest first for black
                        search_result.stats.best_turn_nr = turn_counter as i8;
                        let calc_time_ms = self.get_calc_time(local_map);
                        stats.calc_time_ms = calc_time_ms as usize;
                        stats.calculate();
                        if config.print_info_string_during_search {
                            if let Err(_e) = service.stdout.write_get_result(&service.uci_parser.get_info_str(&search_result, stats)) {
                                logger.send("stdout channel closed during search".to_string())
                                    .expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
                                break;
                            }
                        } 
                    }
                }
            }

            if !config.enable_aspiration || prev_eval.is_none() || stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }

            let best_score = search_result.get_eval();

            // Fail-Low / Fail-High checks
            if best_score <= alpha || best_score >= beta {
                alpha = best_score.saturating_sub(delta).max(i16::MIN);
                beta = best_score.saturating_add(delta).min(i16::MAX);
                delta = delta.saturating_mul(4);
                continue;
            }

            break;
        }

        let calc_time_ms = self.get_calc_time(local_map);
        search_result.stats = stats.clone();
        search_result.stats.calc_time_ms = calc_time_ms as usize;
        if stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
            search_result.completed = false;
        } else {
            search_result.completed = true;
        }
        search_result
    }
    

    fn minimax(&self, board: &mut Board, turn: &Turn, depth: i32, white: bool,
        mut alpha: i16, mut beta: i16, stats: &mut Stats, config: &Config, service: &Service,
        context: &SearchContext, local_map: &mut DataMap, pv: &mut [Option<Turn>; 128],
        ply: i32, killer_moves: &mut [[Option<Turn>; 2]; 128],
        history_table: &mut [[u32; 64]; 64],
        counter_moves: &mut [[Option<Turn>; 64]; 64])
        -> (Option<Turn>, i16) {

        for slot in pv.iter_mut() {
            *slot = None;
        }

        // Mate Distance Pruning at node entry
        if ply > 0 {
            let mate_value = i16::MAX - 1 - ply as i16;
            if mate_value < beta {
                beta = beta.min(mate_value);
                if alpha >= beta {
                    return (None, beta);
                }
            }
            let mate_value = i16::MIN + 1 + ply as i16;
            if mate_value > alpha {
                alpha = alpha.max(mate_value);
                if alpha >= beta {
                    return (None, alpha);
                }
            }
        }

        let orig_alpha = alpha;
        let orig_beta = beta;

        // Transposition Table Lookup
        if depth > 0 && config.use_zobrist {
            if board.cached_hash == 0 {
                board.cached_hash = crate::zobrist::gen(board);
            }
            if let Some(entry) = context.zobrist_table.get_entry(&board.cached_hash) {
                if entry.depth as i32 >= depth {
                    let mut entry_eval = entry.eval;
                    // De-normalize mate score
                    if entry_eval > 30000 {
                        entry_eval = entry_eval - ply as i16;
                    } else if entry_eval < -30000 {
                        entry_eval = entry_eval + ply as i16;
                    }

                    let decompressed = entry.decompress_move(board);

                    match entry.entry_type {
                        crate::zobrist::TranspositionType::Exact => {
                            if let Some(m) = decompressed {
                                pv[0] = Some(m);
                            }
                            return (decompressed, entry_eval);
                        }
                        crate::zobrist::TranspositionType::LowerBound => {
                            alpha = alpha.max(entry_eval);
                            if alpha >= beta {
                                if let Some(m) = decompressed {
                                    pv[0] = Some(m);
                                }
                                return (decompressed, entry_eval);
                            }
                        }
                        crate::zobrist::TranspositionType::UpperBound => {
                            beta = beta.min(entry_eval);
                            if alpha >= beta {
                                if let Some(m) = decompressed {
                                    pv[0] = Some(m);
                                }
                                return (decompressed, entry_eval);
                            }
                        }
                    }
                }
            }
        }

        // 0. Null Move Pruning (NMP)
        if config.enable_nmp 
            && depth >= config.nmp_depth_threshold 
            && !turn.gives_check 
            && self.has_non_pawn_material(board, board.white_to_move) 
        {
            let old_white_to_move = board.white_to_move;
            let old_field_for_en_passante = board.field_for_en_passante;
            let old_hash = board.cached_hash;

            // Make Null Move
            board.white_to_move = !board.white_to_move;
            board.field_for_en_passante = -1;
            board.cached_hash = crate::zobrist::gen(board);

            let reduction = config.nmp_reduction;
            let reduced_depth = depth - 1 - reduction;
            let mut null_pv = [None; 128];

            let null_eval = if white {
                self.minimax(
                    board, turn, reduced_depth, false,
                    beta - 1, beta, stats, config, service, context,
                    local_map, &mut null_pv, ply + 1, killer_moves, history_table, counter_moves
                ).1
            } else {
                self.minimax(
                    board, turn, reduced_depth, true,
                    alpha, alpha + 1, stats, config, service, context,
                    local_map, &mut null_pv, ply + 1, killer_moves, history_table, counter_moves
                ).1
            };

            // Undo Null Move
            board.white_to_move = old_white_to_move;
            board.field_for_en_passante = old_field_for_en_passante;
            board.cached_hash = old_hash;

            if white {
                if null_eval >= beta {
                    return (None, beta); // Beta cutoff
                }
            } else {
                if null_eval <= alpha {
                    return (None, alpha); // Alpha cutoff
                }
            }
        }

        // 0.5. Reverse Futility Pruning (RFP) / Static Null Move Pruning
        if config.enable_rfp 
            && depth > 0 
            && depth <= 3 
            && !turn.gives_check 
            && self.has_non_pawn_material(board, board.white_to_move) 
        {
            local_map.insert(DataMapKey::WhiteGivesCheck, false);
            local_map.insert(DataMapKey::BlackGivesCheck, false);
            let static_eval = service.eval.calc_eval(board, config, &service.move_gen, local_map);
            let margin = 80 * depth as i16;
            
            if white {
                if static_eval - margin >= beta {
                    return (None, static_eval - margin); // Beta cutoff
                }
            } else {
                if static_eval + margin <= alpha {
                    return (None, static_eval + margin); // Alpha cutoff
                }
            }
        }

        let counter_move = if config.enable_counter_moves && ply > 0 {
            counter_moves[turn.from as usize][turn.to as usize]
        } else {
            None
        };

        // SearchContext for current ply with specific killer moves and counter move
        let current_context = SearchContext {
            zobrist_table: context.zobrist_table,
            stop_flag: context.stop_flag,
            pv_nodes: context.pv_nodes,
            killer_moves: if ply >= 0 && ply < 128 { killer_moves[ply as usize] } else { [None; 2] },
            history_table,
            counter_move,
        };

        // Quiescence Search (depth <= 0)
        if depth <= 0 {
            stats.add_eval_nodes(1);
            if board.white_to_move && turn.gives_check {
                local_map.insert(DataMapKey::BlackGivesCheck, true);
            } else if !board.white_to_move && turn.gives_check {
                local_map.insert(DataMapKey::WhiteGivesCheck, true);
            } else {
                local_map.insert(DataMapKey::WhiteGivesCheck, false);
                local_map.insert(DataMapKey::BlackGivesCheck, false);
            }

            let in_check = turn.gives_check;
            let mut stand_pat = 0;
            let mut eval = if white { i16::MIN } else { i16::MAX };

            if !in_check {
                stand_pat = service.eval.calc_eval(board, config, &service.move_gen, local_map);
                eval = stand_pat;
                if config.use_zobrist {
                    context.zobrist_table.insert_entry(board.cached_hash, crate::zobrist::TranspositionEntry {
                        key: board.cached_hash,
                        eval: stand_pat,
                        depth: 0,
                        entry_type: crate::zobrist::TranspositionType::Exact,
                        best_move: 0,
                        padding: [0; 2],
                    });
                }

                // Stand-pat cutoffs
                if white {
                    if stand_pat >= beta {
                        return (None, stand_pat);
                    }
                    alpha = alpha.max(stand_pat);
                } else {
                    if stand_pat <= alpha {
                        return (None, stand_pat);
                    }
                    beta = beta.min(stand_pat);
                }
            }

            local_map.insert(DataMapKey::ForceSkipValidationFlag, false);
            let mut turns = crate::model::MoveList::new();
            if in_check {
                service.move_gen.generate_valid_moves_list(board, stats, config, &current_context, local_map, &mut turns);
            } else {
                service.move_gen.generate_valid_moves_list_capture(board, stats, config, &current_context, local_map, &mut turns);
            }

            if turns.is_empty() {
                if in_check {
                    return match board.game_status {
                        GameStatus::WhiteWin => (None, i16::MAX - 1 - ply as i16),
                        GameStatus::BlackWin => (None, i16::MIN + 1 + ply as i16),
                        GameStatus::Draw => (None, 0),
                        _ => panic!("RIP no defined game end"),
                    };
                }
                return (None, stand_pat);
            }

            let mut child_pv = [None; 128];

            for i in 0..turns.len {
                let mut best_idx = i;
                for j in (i + 1)..turns.len {
                    if turns.moves[j].rank > turns.moves[best_idx].rank {
                        best_idx = j;
                    }
                }
                turns.moves.swap(i, best_idx);
                let capture_turn = &turns.moves[i];

                if config.enable_delta_pruning && !in_check && capture_turn.promotion == 0 {
                    let gain = match capture_turn.capture {
                        10 | 20 => config.piece_eval_pawn,
                        11 | 21 => config.piece_eval_rook,
                        12 | 22 => config.piece_eval_knight,
                        13 | 23 => config.piece_eval_bishop,
                        14 | 24 => config.piece_eval_queen,
                        15 | 25 => config.piece_eval_king,
                        _ => 0,
                    };
                    let delta_margin = config.delta_pruning_margin;
                    if white {
                        if stand_pat + gain + delta_margin < alpha {
                            continue;
                        }
                    } else {
                        if stand_pat - gain - delta_margin > beta {
                            continue;
                        }
                    }
                }

                if stats.calculated_nodes & 1023 == 0 {
                    let elapsed = self.get_calc_time(local_map) as i32;
                    if let Some(&target) = local_map.get_data::<i32>(DataMapKey::TargetTime) {
                        let mut dynamic_target = target;
                        if let (Some(&searched), Some(&total)) = (
                            local_map.get_data::<i32>(DataMapKey::RootMovesSearched),
                            local_map.get_data::<i32>(DataMapKey::RootMovesTotal),
                        ) {
                            if target < i32::MAX - 1000000 && total > 0 && (searched * 100) / total >= 85 {
                                dynamic_target = (target * 13) / 10;
                            }
                        }
                        if elapsed >= dynamic_target {
                            context.stop_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                }

                if context.stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }
                stats.add_calculated_nodes(1);
                let mi = board.do_move(capture_turn);
                let min_max_result = self.minimax(board, capture_turn, depth - 1, !white,
                    alpha, beta, stats, config, service, &current_context, local_map, &mut child_pv,
                    ply + 1, killer_moves, history_table, counter_moves);
                let min_max_eval = min_max_result.1;
                board.undo_move(capture_turn, mi);

                if white {
                    if eval < min_max_eval {
                        eval = min_max_eval;
                        alpha = alpha.max(min_max_eval);
                        pv[0] = Some(*capture_turn);
                        pv[1..].copy_from_slice(&child_pv[..127]);
                    }
                } else {
                    if eval > min_max_eval {
                        eval = min_max_eval;
                        beta = beta.min(min_max_eval);
                        pv[0] = Some(*capture_turn);
                        pv[1..].copy_from_slice(&child_pv[..127]);
                    }
                }

                if beta <= alpha {
                    break;
                }
            }

            return (None, eval);
        }

        // Standard Search (depth > 0)
        local_map.insert(DataMapKey::ForceSkipValidationFlag, config.skip_strong_validation);
        let mut turns = crate::model::MoveList::new();
        service.move_gen.generate_valid_moves_list(board, stats, config, &current_context, local_map, &mut turns);

        let mut eval = if white { i16::MIN } else { i16::MAX };
        let mut best_move: Option<Turn> = None;

        if turns.is_empty() || board.game_status != GameStatus::Normal {
            return match board.game_status {
                GameStatus::WhiteWin => (None, i16::MAX - 1 - ply as i16),
                GameStatus::BlackWin => (None, i16::MIN + 1 + ply as i16),
                GameStatus::Draw => (None, 0),
                _ => panic!("RIP no defined game end"),
            };
        }

        let mut turn_counter = 0;
        let mut child_pv = [None; 128];
        let mut searched_quiet_moves = [None; 64];
        let mut quiet_count = 0;

        for i in 0..turns.len {
            let mut best_idx = i;
            for j in (i + 1)..turns.len {
                if turns.moves[j].rank > turns.moves[best_idx].rank {
                    best_idx = j;
                }
            }
            turns.moves.swap(i, best_idx);
            let current_turn = &turns.moves[i];

            if stats.calculated_nodes & 1023 == 0 {
                let elapsed = self.get_calc_time(local_map) as i32;
                if let Some(&target) = local_map.get_data::<i32>(DataMapKey::TargetTime) {
                    let mut dynamic_target = target;
                    if let (Some(&searched), Some(&total)) = (
                        local_map.get_data::<i32>(DataMapKey::RootMovesSearched),
                        local_map.get_data::<i32>(DataMapKey::RootMovesTotal),
                    ) {
                        if target < i32::MAX - 1000000 && total > 0 && (searched * 100) / total >= 85 {
                            dynamic_target = (target * 13) / 10;
                        }
                    }
                    if elapsed >= dynamic_target {
                        context.stop_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            }

            if context.stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            turn_counter += 1;
            if current_turn.capture == 0 && quiet_count < 64 {
                searched_quiet_moves[quiet_count] = Some(*current_turn);
                quiet_count += 1;
            }
            stats.add_calculated_nodes(1);
            let mi = board.do_move(current_turn);

            let mut min_max_eval = if white { i16::MIN } else { i16::MAX };
            let mut searched = false;

            // 1. Late Move Reductions (LMR)
            if config.enable_lmr 
                && depth >= config.lmr_depth_threshold 
                && turn_counter > config.lmr_move_threshold 
                && current_turn.capture == 0 
                && current_turn.promotion == 0 
                && !current_turn.gives_check 
            {
                let reduction = config.lmr_reduction;
                let reduced_depth = (depth - 1 - reduction).max(1);
                
                if white {
                    min_max_eval = self.minimax(
                        board, current_turn, reduced_depth, !white,
                        alpha, alpha + 1, stats, config, service, &current_context,
                        local_map, &mut child_pv, ply + 1, killer_moves, history_table, counter_moves
                    ).1;
                    if min_max_eval <= alpha {
                        searched = true;
                    }
                } else {
                    min_max_eval = self.minimax(
                        board, current_turn, reduced_depth, !white,
                        beta - 1, beta, stats, config, service, &current_context,
                        local_map, &mut child_pv, ply + 1, killer_moves, history_table, counter_moves
                    ).1;
                    if min_max_eval >= beta {
                        searched = true;
                    }
                }
            }

            // 2. Principal Variation Search (PVS)
            if !searched {
                if config.enable_pvs {
                    if turn_counter > 1 {
                        if white {
                            min_max_eval = self.minimax(
                                board, current_turn, depth - 1, !white,
                                alpha, alpha + 1, stats, config, service, &current_context,
                                local_map, &mut child_pv, ply + 1, killer_moves, history_table, counter_moves
                            ).1;
                            
                            if min_max_eval > alpha && min_max_eval < beta {
                                min_max_eval = self.minimax(
                                    board, current_turn, depth - 1, !white,
                                    alpha, beta, stats, config, service, &current_context,
                                    local_map, &mut child_pv, ply + 1, killer_moves, history_table, counter_moves
                                ).1;
                            }
                        } else {
                            min_max_eval = self.minimax(
                                board, current_turn, depth - 1, !white,
                                beta - 1, beta, stats, config, service, &current_context,
                                local_map, &mut child_pv, ply + 1, killer_moves, history_table, counter_moves
                            ).1;
                            
                            if min_max_eval < beta && min_max_eval > alpha {
                                min_max_eval = self.minimax(
                                    board, current_turn, depth - 1, !white,
                                    alpha, beta, stats, config, service, &current_context,
                                    local_map, &mut child_pv, ply + 1, killer_moves, history_table, counter_moves
                                ).1;
                            }
                        }
                    } else {
                        min_max_eval = self.minimax(
                            board, current_turn, depth - 1, !white,
                            alpha, beta, stats, config, service, &current_context,
                            local_map, &mut child_pv, ply + 1, killer_moves, history_table, counter_moves
                        ).1;
                    }
                } else {
                    min_max_eval = self.minimax(
                        board, current_turn, depth - 1, !white,
                        alpha, beta, stats, config, service, &current_context,
                        local_map, &mut child_pv, ply + 1, killer_moves, history_table, counter_moves
                    ).1;
                }
            }

            board.undo_move(current_turn, mi);

            if white {
                if eval < min_max_eval {
                    eval = min_max_eval;
                    alpha = alpha.max(min_max_eval);
                    best_move = Some(*current_turn);
                    pv[0] = Some(*current_turn);
                    pv[1..].copy_from_slice(&child_pv[..127]);
                    if config.in_debug && turn_counter > 30 {
                        stats.add_turn_nr_gt_threshold(1);
                        stats.add_log(format!("{}, move {} was the {} lvl:{}",
                        service.fen.get_fen(board), &current_turn.to_algebraic(), turn_counter, config.search_depth - depth));
                    };
                }
            } else {
                if eval > min_max_eval {
                    eval = min_max_eval;
                    beta = beta.min(min_max_eval);
                    best_move = Some(*current_turn);
                    pv[0] = Some(*current_turn);
                    pv[1..].copy_from_slice(&child_pv[..127]);
                    if config.in_debug && turn_counter > 30 {
                        stats.add_turn_nr_gt_threshold(1);
                        stats.add_log(format!("{}, move {} was the {} lvl:{}",
                        service.fen.get_fen(board), &current_turn.to_algebraic(), turn_counter, config.search_depth - depth));
                    };
                }
            }
            if beta <= alpha {
                if depth > 0 && current_turn.capture == 0 {
                    // Killer Move storage
                    if (ply as usize) < 128 {
                        let p = ply as usize;
                        if Some(*current_turn) != killer_moves[p][0] {
                            killer_moves[p][1] = killer_moves[p][0];
                            killer_moves[p][0] = Some(*current_turn);
                        }
                    }

                    // History Heuristic Accumulation
                    let from = current_turn.from as usize;
                    let to = current_turn.to as usize;
                    history_table[from][to] += (depth * depth) as u32;

                    // History Malus for previously searched quiet moves
                    if config.enable_history_malus {
                        for j in 0..quiet_count {
                            if let Some(bad_move) = searched_quiet_moves[j] {
                                if bad_move != *current_turn {
                                    let b_from = bad_move.from as usize;
                                    let b_to = bad_move.to as usize;
                                    let penalty = (depth * depth) as u32;
                                    history_table[b_from][b_to] = history_table[b_from][b_to].saturating_sub(penalty);
                                }
                            }
                        }
                    }

                    // Counter-Moves Heuristic storage
                    if config.enable_counter_moves && ply > 0 {
                        counter_moves[turn.from as usize][turn.to as usize] = Some(*current_turn);
                    }

                    // Overflow Protection & Ageing
                    if history_table[from][to] > config.history_max_threshold {
                        for r in history_table.iter_mut() {
                            for c in r.iter_mut() {
                                *c /= 2;
                            }
                        }
                    }
                }
                break;
            }
        }

        // Transposition Table Write
        if config.use_zobrist && !context.stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
            let entry_type = if eval <= orig_alpha {
                crate::zobrist::TranspositionType::UpperBound
            } else if eval >= orig_beta {
                crate::zobrist::TranspositionType::LowerBound
            } else {
                crate::zobrist::TranspositionType::Exact
            };

            let mut stored_eval = eval;
            // Normalize mate score
            if eval > 30000 {
                stored_eval = eval + ply as i16;
            } else if eval < -30000 {
                stored_eval = eval - ply as i16;
            }

            context.zobrist_table.insert_entry(
                board.cached_hash,
                crate::zobrist::TranspositionEntry {
                    key: board.cached_hash,
                    eval: stored_eval,
                    depth: depth as i8,
                    entry_type,
                    best_move: crate::zobrist::TranspositionEntry::compress_move(best_move),
                    padding: [0; 2],
                },
            );
        }

        return (best_move, eval);
    }

    fn get_calc_time(&self, local_map: &DataMap) -> u128 {
        local_map.get_data::<Instant>(DataMapKey::CalcTime)
            .expect(RIP_MISSED_DM_KEY)
            .elapsed()
            .as_millis()
    }

    fn has_non_pawn_material(&self, board: &Board, white: bool) -> bool {
        if white {
            (board.bitboards[crate::model::WHITE_ROOK] |
             board.bitboards[crate::model::WHITE_KNIGHT] |
             board.bitboards[crate::model::WHITE_BISHOP] |
             board.bitboards[crate::model::WHITE_QUEEN]) != 0
        } else {
            (board.bitboards[crate::model::BLACK_ROOK] |
             board.bitboards[crate::model::BLACK_KNIGHT] |
             board.bitboards[crate::model::BLACK_BISHOP] |
             board.bitboards[crate::model::BLACK_QUEEN]) != 0
        }
    }

}


#[cfg(test)]
mod tests {
    use crate::config::Config;
    use crate::service::Service;
    use crate::model::{Board, SearchResult, EngineState, DataMap, DataMapKey, Stats, SearchContext, Turn};
    use crate::zobrist::ZobristTable;
    use std::sync::Arc;
    use std::time::Instant;

    pub fn search(board: &mut Board, depth: i32, white: bool) -> SearchResult {
        let service = Service::new();
        let config = Config::for_tests();
        let mut stats = Stats::new();
        let mut local_map = DataMap::new();
        local_map.insert(DataMapKey::CalcTime, std::time::Instant::now());
        local_map.insert(DataMapKey::WhiteGivesCheck, false);
        local_map.insert(DataMapKey::BlackGivesCheck, false);

        let (tx_log, _rx_log) = std::sync::mpsc::channel();
        let engine_state = Arc::new(EngineState {
            stop_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            debug_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            zobrist_table: Arc::new(ZobristTable::new()),
            pv_nodes: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            pv_nodes_len: Arc::new(std::sync::atomic::AtomicI32::new(0)),
            logger: Arc::new(std::sync::RwLock::new(Arc::new(|_| {}))),
            log_sender: tx_log,
        });

        service.search.get_moves(&mut *board, depth, white, &mut stats, &config, &service, &engine_state, &mut local_map)
    }
    

    #[test]
    #[ignore]
    fn white_matt_tests() {
        let fen_service = Service::new().fen;
        
        let mut board = fen_service.set_fen("8/3K4/8/8/5RR1/8/k7/8 w - - 0 1");
        let result = search(&mut board, 6, true);
        assert_eq!(result.get_eval(), 32761);
        let best_move = result.get_best_move_algebraic();
        assert!(best_move == "f4f3" || best_move == "f4b4", "Expected f4f3 or f4b4, got {}", best_move);

        let mut board = fen_service.set_fen("r1q1r1k1/ppppppp1/n1b4p/7N/2B1P2N/2B2Q1P/PPPP1PP1/R3R1K1 w Qq - 0 1");
        let result = search(&mut board, 4, true);
        assert_eq!(result.get_eval(), 32763);
        assert_eq!(result.get_best_move_algebraic(), "f3f7");
        

        let mut board = fen_service.set_fen("6rk/R2R4/7P/8/p1B2P2/2P4P/P5K1/8 w - - 5 39");
        let result = search(&mut board, 6, true);
        assert_eq!(result.get_eval(), 32761);
        assert_eq!(result.get_best_move_algebraic(), "c4g8");
    }


    #[test]
    #[ignore]
    fn black_matt_tests() {
        let fen_service = Service::new().fen;
        
        let mut board = fen_service.set_fen("8/1p6/p1P5/2p5/K1p2P2/P2kPn1P/1r6/8 b - - 3 43");
        let result = search(&mut board, 6, false);
        assert_eq!(result.get_eval(), -32762);
        assert_eq!(result.get_best_move_algebraic(), "b7b6");

        
        let mut board = fen_service.set_fen("8/8/8/2k5/8/5p1r/1K6/8 b - - 0 1");
        let result = search(&mut board, 8, false);
        assert_eq!(result.get_eval(), -32760);
        assert_eq!(result.get_best_move_algebraic(), "f3f2");
        

        let mut board = fen_service.set_fen("8/5pkp/p5p1/4p3/1P3P2/P3P1KP/2q3P1/3r4 b - - 0 37");
        let result = search(&mut board, 6, false);
        assert_eq!(result.get_eval(), -32762);
        assert_eq!(result.get_best_move_algebraic(), "d1g1");
    }


    #[test]
    fn black_find_hit_move() {
        let fen_service = Service::new().fen;
        
        let mut board = fen_service.set_fen("2r2rk1/1b2bppp/pqn1pn2/8/1PBB4/P3PN2/5PPP/RN1Q1RK1 b - - 2 14");
        let result = search(&mut board, 2, false);
        result._print_all_variants();
        assert!(result.get_eval() < -100);
        assert_eq!(result.get_best_move_algebraic(), "c6d4");

        let mut board = fen_service.set_fen("6k1/5pp1/5rnp/2Npb3/3PP3/r1P1R2P/5PP1/4BR1K b - - 0 1");
        let result = search(&mut board, 2, false);
        assert!(result.get_eval() > 0);
    }


    #[test]
    fn white_find_hit_move() {
        let fen_service = Service::new().fen;

        let mut board = fen_service.set_fen("3r2nk/6pp/3p4/4p3/3BP3/8/3R2PP/6NK w - - 0 1");
        let result = search(&mut board, 2, true);
        assert_eq!(result.get_best_move_algebraic(), "d4e5");
        
        let mut board = fen_service.set_fen("7k/6pp/3p4/4n3/3QP3/8/3R2PP/7K w - - 0 1");
        let result = search(&mut board, 2, true);
        assert_eq!(result.get_best_move_algebraic(), "d4d6");


        let mut board = fen_service.set_fen("7k/6pp/3p1p2/4r3/p2QP3/8/3R2PP/7K w - - 0 1");
        let result = search(&mut board, 2, true);
        assert_eq!(result.get_best_move_algebraic(), "d4a4");
    }


    #[test]
    #[ignore]
    fn hit_move_unsolved() {
        let fen_service = Service::new().fen;

        let mut board = fen_service.set_fen("4k3/5pp1/2r3np/2Ppp3/3BP3/7P/5PP1/3RR1K1 b - - 0 1");
        let result = search(&mut board, 2, false);
        result._print_all_variants();
    }

    
    #[test]
    fn practical_moves_from_games() {
        let fen_service = Service::new().fen;

        let mut board = fen_service.set_fen("rnbqkbnr/1p3ppp/p7/1Np5/1P1p4/5N2/P2PPPPP/R1BQKB1R w KQkq - 0 7");
        let result = search(&mut board, 3, true);
        result._print_all_variants();
    }

    #[test]
    fn stop_flag_termination_test() {
        use std::sync::atomic::Ordering;
        use std::thread;
        use std::time::Duration;

        let (tx_log, _rx_log) = std::sync::mpsc::channel();
        let engine_state = Arc::new(EngineState {
            stop_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            debug_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            zobrist_table: Arc::new(ZobristTable::new()),
            pv_nodes: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            pv_nodes_len: Arc::new(std::sync::atomic::AtomicI32::new(0)),
            logger: Arc::new(std::sync::RwLock::new(Arc::new(|_| {}))),
            log_sender: tx_log,
        });
        let stop_flag = engine_state.stop_flag.clone();

        let engine_state_clone = engine_state.clone();
        let handle = thread::spawn(move || {
            let service = Service::new();
            let mut board = service.fen.set_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
            let mut stats = Stats::new();
            let config = Config::for_tests();
            let mut local_map = DataMap::new();
            local_map.insert(DataMapKey::CalcTime, Instant::now());
            local_map.insert(DataMapKey::WhiteGivesCheck, false);
            local_map.insert(DataMapKey::BlackGivesCheck, false);

            service.search.get_moves(
                &mut board,
                8,
                true,
                &mut stats,
                &config,
                &service,
                &engine_state_clone,
                &mut local_map,
            )
        });

        thread::sleep(Duration::from_millis(15));
        stop_flag.store(true, Ordering::Relaxed);

        let start_wait = std::time::Instant::now();
        let search_result = handle.join().expect("Search thread panicked");
        let duration = start_wait.elapsed();

        assert!(duration < Duration::from_millis(400), "Search took too long to terminate: {:?}", duration);
        assert!(!search_result.completed, "Search should be marked as incomplete");
    }

    #[test]
    fn zobrist_table_concurrent_stress_test() {
        use std::thread;

        let zobrist_table = Arc::new(ZobristTable::new());

        let mut handles = vec![];
        let num_threads = 4;
        let elements_per_thread = 500;

        for t in 0..num_threads {
            let table = zobrist_table.clone();
            let handle = thread::spawn(move || {
                for i in 0..elements_per_thread {
                    let hash_key = (t as u64 * 1_000_000) + i as u64;
                    let eval_val = (i % 30000) as i16;
                    let entry = crate::zobrist::TranspositionEntry {
                        key: hash_key,
                        eval: eval_val,
                        depth: 4,
                        entry_type: crate::zobrist::TranspositionType::Exact,
                        best_move: 0,
                        padding: [0; 2],
                    };
                    table.insert_entry(hash_key, entry);
                    let read_val = table.get_eval_for_hash(&hash_key);
                    assert_eq!(read_val, Some(eval_val));
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().expect("Stress thread panicked");
        }

        assert_eq!(zobrist_table._size(), num_threads * elements_per_thread);
    }

    #[test]
    fn thread_counter_integrity_test() {
        use std::sync::atomic::{AtomicI32, Ordering};
        use std::thread;
        use std::sync::Arc;

        let counter = Arc::new(AtomicI32::new(0));
        let num_threads = 10;
        let mut spawn_handles = vec![];

        for _ in 0..num_threads {
            counter.fetch_add(1, Ordering::Relaxed);
            let counter_clone = counter.clone();

            let handle = thread::spawn(move || {
                thread::sleep(std::time::Duration::from_millis(5));
                counter_clone.fetch_sub(1, Ordering::Relaxed);
            });
            spawn_handles.push(handle);
        }

        for handle in spawn_handles {
            handle.join().expect("Spawned test thread panicked");
        }

        assert_eq!(counter.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn zobrist_transposition_table_cutoff_test() {
        let service = Service::new();
        let mut board = service.fen.set_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        
        let mut stats = Stats::new();
        let mut config = Config::for_tests();
        config.use_zobrist = true;

        let mut local_map = DataMap::new();
        local_map.insert(DataMapKey::CalcTime, Instant::now());
        local_map.insert(DataMapKey::WhiteGivesCheck, false);
        local_map.insert(DataMapKey::BlackGivesCheck, false);

        let table = Arc::new(ZobristTable::new());
        let (tx_log, _rx_log) = std::sync::mpsc::channel();
        let engine_state = Arc::new(EngineState {
            stop_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            debug_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            zobrist_table: table.clone(),
            pv_nodes: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            pv_nodes_len: Arc::new(std::sync::atomic::AtomicI32::new(0)),
            logger: Arc::new(std::sync::RwLock::new(Arc::new(|_| {}))),
            log_sender: tx_log,
        });

        let mut test_history_table = [[0u32; 64]; 64];
        let context = SearchContext {
            zobrist_table: &engine_state.zobrist_table,
            stop_flag: &engine_state.stop_flag,
            pv_nodes: &engine_state.pv_nodes,
            killer_moves: [None; 2],
            history_table: &test_history_table,
            counter_move: None,
        };

        let mut test_killer_moves = [[None; 2]; 128];
        let mut test_counter_moves = [[None; 64]; 64];

        // 1. Insert an Exact transposition entry
        board.cached_hash = crate::zobrist::gen(&board);
        let test_hash = board.cached_hash;
        table.insert_entry(test_hash, crate::zobrist::TranspositionEntry {
            key: test_hash,
            eval: 500,
            depth: 3,
            entry_type: crate::zobrist::TranspositionType::Exact,
            best_move: 0,
            padding: [0; 2],
        });

        // Search depth 3. It should trigger an immediate exact cutoff and return 500.
        let result = service.search.minimax(
            &mut board,
            &Turn::new(0, 0, 0, 0, false, 0),
            3,
            true,
            i16::MIN,
            i16::MAX,
            &mut stats,
            &config,
            &service,
            &context,
            &mut local_map,
            &mut [None; 128],
            1,
            &mut test_killer_moves,
            &mut test_history_table,
            &mut test_counter_moves,
        );

        assert_eq!(result.1, 500);
        assert_eq!(stats.calculated_nodes, 0); // No nodes calculated because of TT cutoff!

        // 2. LowerBound cutoff verification
        stats = Stats::new();
        table.insert_entry(test_hash, crate::zobrist::TranspositionEntry {
            key: test_hash,
            eval: 600,
            depth: 4,
            entry_type: crate::zobrist::TranspositionType::LowerBound,
            best_move: 0,
            padding: [0; 2],
        });

        // Search depth 4, alpha = 200, beta = 500. Since eval (600) >= beta (500), it should cause a beta cutoff.
        let result_lower = service.search.minimax(
            &mut board,
            &Turn::new(0, 0, 0, 0, false, 0),
            4,
            true,
            200,
            500,
            &mut stats,
            &config,
            &service,
            &context,
            &mut local_map,
            &mut [None; 128],
            1,
            &mut test_killer_moves,
            &mut test_history_table,
            &mut test_counter_moves,
        );
        assert_eq!(result_lower.1, 600);
        assert_eq!(stats.calculated_nodes, 0);

        // 3. UpperBound cutoff verification
        stats = Stats::new();
        table.insert_entry(test_hash, crate::zobrist::TranspositionEntry {
            key: test_hash,
            eval: 100,
            depth: 2,
            entry_type: crate::zobrist::TranspositionType::UpperBound,
            best_move: 0,
            padding: [0; 2],
        });

        // Search depth 2, alpha = 300, beta = 700. Since eval (100) <= alpha (300), it should cause an alpha cutoff.
        let result_upper = service.search.minimax(
            &mut board,
            &Turn::new(0, 0, 0, 0, false, 0),
            2,
            true,
            300,
            700,
            &mut stats,
            &config,
            &service,
            &context,
            &mut local_map,
            &mut [None; 128],
            1,
            &mut test_killer_moves,
            &mut test_history_table,
            &mut test_counter_moves,
        );
        assert_eq!(result_upper.1, 100);
        assert_eq!(stats.calculated_nodes, 0);
    }

    #[test]
    #[ignore]
    fn print_search_times_test() {
        let service = Service::new();
        let mut board = service.fen.set_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        
        let config = Config::new();
        let table = Arc::new(ZobristTable::new());
        let (tx_log, _rx_log) = std::sync::mpsc::channel();
        let engine_state = Arc::new(EngineState {
            stop_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            debug_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            zobrist_table: table.clone(),
            pv_nodes: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            pv_nodes_len: Arc::new(std::sync::atomic::AtomicI32::new(0)),
            logger: Arc::new(std::sync::RwLock::new(Arc::new(|_| {}))),
            log_sender: tx_log,
        });

        println!("\nSuchtiefen-Benchmark für startpos:");
        for d in 1..=9 {
            let mut stats = Stats::new();
            let mut local_map = DataMap::new();
            local_map.insert(DataMapKey::CalcTime, Instant::now());
            local_map.insert(DataMapKey::WhiteGivesCheck, false);
            local_map.insert(DataMapKey::BlackGivesCheck, false);

            let start = Instant::now();
            service.search.get_moves(&mut board, d, true, &mut stats, &config, &service, &engine_state, &mut local_map);
            let elapsed = start.elapsed();
            let elapsed_ms = elapsed.as_millis().max(1);
            println!("Tiefe {}: {} ms (Knoten: {}, nps: {} k)", d, elapsed.as_millis(), stats.calculated_nodes, stats.calculated_nodes / elapsed_ms as usize);
        }
    }

    #[test]
    fn search_feature_toggles_test() {
        let service = Service::new();
        let mut board = service.fen.set_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        
        let (tx_log, _rx_log) = std::sync::mpsc::channel();
        let engine_state = Arc::new(EngineState {
            stop_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            debug_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            zobrist_table: Arc::new(ZobristTable::new()),
            pv_nodes: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            pv_nodes_len: Arc::new(std::sync::atomic::AtomicI32::new(0)),
            logger: Arc::new(std::sync::RwLock::new(Arc::new(|_| {}))),
            log_sender: tx_log,
        });

        // 1. Search with both PVS and LMR DISABLED
        let mut config_disabled = Config::for_tests();
        config_disabled.enable_pvs = false;
        config_disabled.enable_lmr = false;
        
        let mut stats_disabled = Stats::new();
        let mut local_map_disabled = DataMap::new();
        local_map_disabled.insert(DataMapKey::CalcTime, Instant::now());
        local_map_disabled.insert(DataMapKey::WhiteGivesCheck, false);
        local_map_disabled.insert(DataMapKey::BlackGivesCheck, false);
        
        let result_disabled = service.search.get_moves(
            &mut board,
            4,
            true,
            &mut stats_disabled,
            &config_disabled,
            &service,
            &engine_state,
            &mut local_map_disabled,
        );

        // 2. Search with both PVS and LMR ENABLED
        let mut config_enabled = Config::for_tests();
        config_enabled.enable_pvs = true;
        config_enabled.enable_lmr = true;
        
        let mut stats_enabled = Stats::new();
        let mut local_map_enabled = DataMap::new();
        local_map_enabled.insert(DataMapKey::CalcTime, Instant::now());
        local_map_enabled.insert(DataMapKey::WhiteGivesCheck, false);
        local_map_enabled.insert(DataMapKey::BlackGivesCheck, false);
        
        let result_enabled = service.search.get_moves(
            &mut board,
            4,
            true,
            &mut stats_enabled,
            &config_enabled,
            &service,
            &engine_state,
            &mut local_map_enabled,
        );

        assert!(result_disabled.completed);
        assert!(result_enabled.completed);
        
        // PVS & LMR should result in a massive node reduction
        assert!(stats_enabled.calculated_nodes < stats_disabled.calculated_nodes, 
            "Enabled nodes ({}) should be strictly less than disabled nodes ({}) due to PVS and LMR pruning!",
            stats_enabled.calculated_nodes, stats_disabled.calculated_nodes);
    }

    #[test]
    fn search_nmp_pruning_test() {
        let service = Service::new();
        let mut board = service.fen.set_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        
        let (tx_log, _rx_log) = std::sync::mpsc::channel();
        let engine_state = Arc::new(EngineState {
            stop_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            debug_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            zobrist_table: Arc::new(ZobristTable::new()),
            pv_nodes: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            pv_nodes_len: Arc::new(std::sync::atomic::AtomicI32::new(0)),
            logger: Arc::new(std::sync::RwLock::new(Arc::new(|_| {}))),
            log_sender: tx_log,
        });

        // 1. Search with NMP DISABLED
        let mut config_disabled = Config::for_tests();
        config_disabled.enable_nmp = false;
        
        let mut stats_disabled = Stats::new();
        let mut local_map_disabled = DataMap::new();
        local_map_disabled.insert(DataMapKey::CalcTime, Instant::now());
        local_map_disabled.insert(DataMapKey::WhiteGivesCheck, false);
        local_map_disabled.insert(DataMapKey::BlackGivesCheck, false);
        
        let result_disabled = service.search.get_moves(
            &mut board,
            5,
            true,
            &mut stats_disabled,
            &config_disabled,
            &service,
            &engine_state,
            &mut local_map_disabled,
        );

        // 2. Search with NMP ENABLED
        let mut config_enabled = Config::for_tests();
        config_enabled.enable_nmp = true;
        
        let mut stats_enabled = Stats::new();
        let mut local_map_enabled = DataMap::new();
        local_map_enabled.insert(DataMapKey::CalcTime, Instant::now());
        local_map_enabled.insert(DataMapKey::WhiteGivesCheck, false);
        local_map_enabled.insert(DataMapKey::BlackGivesCheck, false);
        
        let result_enabled = service.search.get_moves(
            &mut board,
            5,
            true,
            &mut stats_enabled,
            &config_enabled,
            &service,
            &engine_state,
            &mut local_map_enabled,
        );

        assert!(result_disabled.completed);
        assert!(result_enabled.completed);
        
        // NMP should result in a massive node reduction at depth 5
        assert!(stats_enabled.calculated_nodes < stats_disabled.calculated_nodes, 
            "Enabled NMP nodes ({}) should be strictly less than disabled NMP nodes ({}) due to Null Move Pruning!",
            stats_enabled.calculated_nodes, stats_disabled.calculated_nodes);
    }

    #[test]
    fn search_aspiration_window_test() {
        let service = Service::new();
        let mut board = service.fen.set_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        
        let (tx_log, _rx_log) = std::sync::mpsc::channel();
        let engine_state = Arc::new(EngineState {
            stop_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            debug_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            zobrist_table: Arc::new(ZobristTable::with_capacity(10000)),
            pv_nodes: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            pv_nodes_len: Arc::new(std::sync::atomic::AtomicI32::new(0)),
            logger: Arc::new(std::sync::RwLock::new(Arc::new(|_| {}))),
            log_sender: tx_log.clone(),
        });

        // Search with Aspiration Windows DISABLED
        let mut config_disabled = Config::for_tests();
        config_disabled.use_zobrist = true;
        config_disabled.enable_aspiration = false;
        
        let mut stats_disabled = Stats::new();
        let mut local_map_disabled = DataMap::new();
        local_map_disabled.insert(DataMapKey::CalcTime, Instant::now());
        local_map_disabled.insert(DataMapKey::WhiteGivesCheck, false);
        local_map_disabled.insert(DataMapKey::BlackGivesCheck, false);
        
        let result_disabled = service.search.get_moves(
            &mut board,
            3,
            true,
            &mut stats_disabled,
            &config_disabled,
            &service,
            &engine_state,
            &mut local_map_disabled,
        );

        // Search with Aspiration Windows ENABLED
        let mut config_enabled = Config::for_tests();
        config_enabled.use_zobrist = true;
        config_enabled.enable_aspiration = true;
        
        let table = Arc::new(ZobristTable::with_capacity(10000));
        table.insert_entry(board.cached_hash, crate::zobrist::TranspositionEntry {
            key: board.cached_hash,
            eval: result_disabled.get_eval(),
            depth: 2,
            entry_type: crate::zobrist::TranspositionType::Exact,
            best_move: crate::zobrist::TranspositionEntry::compress_move(result_disabled.variants[0].best_move),
            padding: [0; 2],
        });
        
        let engine_state_enabled = Arc::new(EngineState {
            stop_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            debug_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            zobrist_table: table,
            pv_nodes: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            pv_nodes_len: Arc::new(std::sync::atomic::AtomicI32::new(0)),
            logger: Arc::new(std::sync::RwLock::new(Arc::new(|_| {}))),
            log_sender: tx_log.clone(),
        });

        let mut stats_enabled = Stats::new();
        let mut local_map_enabled = DataMap::new();
        local_map_enabled.insert(DataMapKey::CalcTime, Instant::now());
        local_map_enabled.insert(DataMapKey::WhiteGivesCheck, false);
        local_map_enabled.insert(DataMapKey::BlackGivesCheck, false);
        
        let result_enabled = service.search.get_moves(
            &mut board,
            3,
            true,
            &mut stats_enabled,
            &config_enabled,
            &service,
            &engine_state_enabled,
            &mut local_map_enabled,
        );

        assert!(result_disabled.completed);
        assert!(result_enabled.completed);
        assert_eq!(result_disabled.get_eval(), result_enabled.get_eval());
    }

    #[test]
    fn search_rfp_pruning_test() {
        let service = Service::new();
        let mut board = service.fen.set_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        
        let (tx_log, _rx_log) = std::sync::mpsc::channel();
        let engine_state = Arc::new(EngineState {
            stop_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            debug_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            zobrist_table: Arc::new(ZobristTable::with_capacity(10000)),
            pv_nodes: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            pv_nodes_len: Arc::new(std::sync::atomic::AtomicI32::new(0)),
            logger: Arc::new(std::sync::RwLock::new(Arc::new(|_| {}))),
            log_sender: tx_log,
        });

        // 1. Search with RFP DISABLED
        let mut config_disabled = Config::for_tests();
        config_disabled.use_zobrist = true;
        config_disabled.enable_rfp = false;
        
        let mut stats_disabled = Stats::new();
        let mut local_map_disabled = DataMap::new();
        local_map_disabled.insert(DataMapKey::CalcTime, Instant::now());
        local_map_disabled.insert(DataMapKey::WhiteGivesCheck, false);
        local_map_disabled.insert(DataMapKey::BlackGivesCheck, false);
        
        let result_disabled = service.search.get_moves(
            &mut board,
            3,
            true,
            &mut stats_disabled,
            &config_disabled,
            &service,
            &engine_state,
            &mut local_map_disabled,
        );

        // 2. Search with RFP ENABLED
        let mut config_enabled = Config::for_tests();
        config_enabled.use_zobrist = true;
        config_enabled.enable_rfp = true;
        
        let mut stats_enabled = Stats::new();
        let mut local_map_enabled = DataMap::new();
        local_map_enabled.insert(DataMapKey::CalcTime, Instant::now());
        local_map_enabled.insert(DataMapKey::WhiteGivesCheck, false);
        local_map_enabled.insert(DataMapKey::BlackGivesCheck, false);
        
        let result_enabled = service.search.get_moves(
            &mut board,
            3,
            true,
            &mut stats_enabled,
            &config_enabled,
            &service,
            &engine_state,
            &mut local_map_enabled,
        );

        assert!(result_disabled.completed);
        assert!(result_enabled.completed);
        
        // RFP should prune branches and reduce nodes
        assert!(stats_enabled.calculated_nodes < stats_disabled.calculated_nodes, 
            "Enabled RFP nodes ({}) should be strictly less than disabled RFP nodes ({}) due to Reverse Futility Pruning!",
            stats_enabled.calculated_nodes, stats_disabled.calculated_nodes);
    }

}
