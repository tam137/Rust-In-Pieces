use std::collections::VecDeque;
use std::sync::Arc;
use crate::config::Config;
use crate::model::{Board, GameStatus, SearchResult, Stats, Turn, Variant, SearchContext, EngineState, RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE};
use crate::service::Service;
use crate::move_gen_service::MoveGenService;


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
        start_time: std::time::Instant,
        target_time: Option<i32>,
    ) -> SearchResult {
        let mut search_config = config.clone();
        search_config.pre_sort_moves = false;
        let config = &search_config;
        let logger = engine_state.log_sender.clone();

        // Always ensure that WhiteGivesCheck and BlackGivesCheck are initialized in local_map
                
        let current_zobrist_table = engine_state.zobrist_table.read().unwrap().clone();
        let zobrist_table = &*current_zobrist_table;
        let stop_flag = &engine_state.stop_flag;
        let pv_nodes = &engine_state.pv_nodes;

        let mut killer_moves: [[Option<Turn>; 2]; 128] = [[None; 2]; 128];
        let mut history_table = [[0u32; 64]; 64];
        let mut counter_moves: [[Option<Turn>; 64]; 64] = [[None; 64]; 64];

        let mut context = SearchContext {
            zobrist_table,
            stop_flag,
            pv_nodes,
            killer_moves: [None; 2],
            history_table: &history_table,
            counter_move: None,
            start_time,
            target_time,
            root_moves_total: 0,
            root_moves_searched: 0,
        };

        let mut turns = crate::model::MoveList::new();
        service.move_gen.generate_valid_moves_list(board, stats, config, &context, true, false, &mut turns);

        // Sorting and SEE are deferred (Lazy Move Picking & Lazy SEE)

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
            search_result.is_pv_search_result = true;

            let mut current_alpha = alpha;
            let mut current_beta = beta;

            let mut best_eval = if white { i16::MIN } else { i16::MAX };
            let total_root_moves = turns.len as i32;
            context.root_moves_total = total_root_moves;
            context.root_moves_searched = 0;

            let mut turn_counter = 0;
            let mut child_pv = [None; 128];

            let mut i = 0;
            while i < turns.len {
                let mut best_idx = i;
                for j in (i + 1)..turns.len {
                    if turns.moves[j].rank > turns.moves[best_idx].rank {
                        best_idx = j;
                    }
                }
                turns.moves.swap(i, best_idx);

                if turns.moves[i].capture != 0 && turns.moves[i].rank >= 0 && turns.moves[i].rank < 100000 {
                    if !self.see_ge(board, &turns.moves[i], 0, config, &service.move_gen) {
                        turns.moves[i].rank -= 100000;
                        continue; // rank decreased, re-evaluate this index to find the next best move
                    }
                }

                let turn = &turns.moves[i];

                // Check time at the start of each root move
                let elapsed = context.start_time.elapsed().as_millis() as i32;
                if let Some(target) = context.target_time {
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
                    let calc_time_ms = context.start_time.elapsed().as_millis();
                    search_result.stats = stats.clone();
                    search_result.stats.best_turn_nr = turn_counter as i8;
                    search_result.stats.calc_time_ms = calc_time_ms as usize;
                    break;
                }

                turn_counter += 1;
                context.root_moves_searched = turn_counter - 1;
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
                    start_time: context.start_time,
                    target_time: context.target_time,
                    root_moves_total: context.root_moves_total,
                    root_moves_searched: context.root_moves_searched,
                };

                let min_max_result = self.minimax(board, turn, depth - 1, !white,
                    current_alpha, current_beta, stats, config, service, &child_context, true, false, false, &mut child_pv,
                    1, &mut killer_moves, &mut history_table, &mut counter_moves);

                if stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                    board.undo_move(turn, mi);
                    let calc_time_ms = context.start_time.elapsed().as_millis();
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
                        let calc_time_ms = context.start_time.elapsed().as_millis();
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
                        let calc_time_ms = context.start_time.elapsed().as_millis();
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
                i += 1;
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

        let calc_time_ms = context.start_time.elapsed().as_millis();
        search_result.stats = stats.clone();
        search_result.stats.calc_time_ms = calc_time_ms as usize;
        if stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
            search_result.completed = false;
        } else {
            search_result.completed = true;
        }
        search_result
    }
    

    fn get_piece_value(&self, piece: u8, config: &Config) -> i16 {
        match piece {
            10 | 20 => crate::pst::PIECE_EVAL_PAWN,
            11 | 21 => crate::pst::PIECE_EVAL_ROOK,
            12 | 22 => crate::pst::PIECE_EVAL_KNIGHT,
            13 | 23 => crate::pst::PIECE_EVAL_BISHOP,
            14 | 24 => crate::pst::PIECE_EVAL_QUEEN,
            15 | 25 => 20000, // King has "infinite" value proxy
            _ => 0,
        }
    }

    fn get_least_valuable_attacker(&self, board: &Board, target_idx: u8, white: bool, occupied: u64, movegen: &MoveGenService) -> Option<(u8, u8)> {
        let attackers_mask = movegen.get_attackers_mask(board, !white, target_idx, occupied);
        let active_attackers = attackers_mask & occupied;
        if active_attackers == 0 {
            return None;
        }

        let piece_order = if white {
            [
                (10, crate::model::WHITE_PAWN),
                (12, crate::model::WHITE_KNIGHT),
                (13, crate::model::WHITE_BISHOP),
                (11, crate::model::WHITE_ROOK),
                (14, crate::model::WHITE_QUEEN),
                (15, crate::model::WHITE_KING),
            ]
        } else {
            [
                (20, crate::model::BLACK_PAWN),
                (22, crate::model::BLACK_KNIGHT),
                (23, crate::model::BLACK_BISHOP),
                (21, crate::model::BLACK_ROOK),
                (24, crate::model::BLACK_QUEEN),
                (25, crate::model::BLACK_KING),
            ]
        };

        for &(piece_val, bb_idx) in &piece_order {
            let intersect = board.bitboards[bb_idx] & active_attackers;
            if intersect != 0 {
                let attacker_sq = intersect.trailing_zeros() as u8;
                return Some((attacker_sq, piece_val));
            }
        }

        None
    }

    pub fn see(&self, board: &Board, mv: &Turn, config: &Config, movegen: &MoveGenService) -> i16 {
        let from = mv.from as usize;
        let to = mv.to as usize;

        let mut gain = [0i16; 32];
        let mut depth = 0;

        let victim = board.get_piece_at(mv.to);
        gain[0] = self.get_piece_value(victim, config);

        let mut current_attacker = board.get_piece_at(mv.from);

        let mut occupied = board.occupied;
        occupied &= !(1u64 << from);

        let mut white_to_move = !board.white_to_move;

        loop {
            if let Some((attacker_sq, attacker_piece)) = self.get_least_valuable_attacker(board, to as u8, white_to_move, occupied, movegen) {
                if depth >= 31 {
                    break;
                }
                depth += 1;
                gain[depth] = self.get_piece_value(current_attacker, config);
                current_attacker = attacker_piece;
                occupied &= !(1u64 << attacker_sq);
                white_to_move = !white_to_move;
            } else {
                break;
            }
        }

        while depth > 0 {
            gain[depth - 1] = gain[depth - 1] - gain[depth].max(0);
            depth -= 1;
        }

        gain[0]
    }

    pub fn see_ge(&self, board: &Board, mv: &Turn, threshold: i16, config: &Config, movegen: &MoveGenService) -> bool {
        self.see(board, mv, config, movegen) >= threshold
    }

    fn minimax(&self, board: &mut Board, turn: &Turn, depth: i32, white: bool,
        mut alpha: i16, mut beta: i16, stats: &mut Stats, config: &Config, service: &Service,
        context: &SearchContext, is_pv: bool,
        skip_null_move: bool,
        force_skip_validation: bool,
        pv: &mut [Option<Turn>; 128],
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
                board.cached_hash = crate::zobrist::gen_hash(board);
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
            && !skip_null_move
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
            board.cached_hash = crate::zobrist::gen_hash(board);

            let dynamic_divisor = if config.nmp_dynamic_divisor > 0 { config.nmp_dynamic_divisor } else { 6 };
            let reduction = config.nmp_reduction + (depth / dynamic_divisor);
            let mut reduced_depth = depth - 1 - reduction;
            if reduced_depth < 0 {
                reduced_depth = 0;
            }
            let mut null_pv = [None; 128];

            let null_eval = if white {
                self.minimax(
                    board, turn, reduced_depth, false,
                    beta - 1, beta, stats, config, service, context,
                    is_pv, true, force_skip_validation, &mut null_pv, ply + 1, killer_moves, history_table, counter_moves
                ).1
            } else {
                self.minimax(
                    board, turn, reduced_depth, true,
                    alpha, alpha + 1, stats, config, service, context,
                    is_pv, true, force_skip_validation, &mut null_pv, ply + 1, killer_moves, history_table, counter_moves
                ).1
            };

            // Undo Null Move
            board.white_to_move = old_white_to_move;
            board.field_for_en_passante = old_field_for_en_passante;
            board.cached_hash = old_hash;

            let is_cutoff = if white { null_eval >= beta } else { null_eval <= alpha };

            if is_cutoff {
                // Verification Search for high depths
                if depth >= config.nmp_verification_threshold {
                    let mut verify_pv = [None; 128];
                    let verify_eval = self.minimax(
                        board, turn, reduced_depth, white,
                        alpha, beta, stats, config, service, context,
                        is_pv, true, force_skip_validation, &mut verify_pv, ply + 1, killer_moves, history_table, counter_moves
                    ).1;

                    let verify_cutoff = if white { verify_eval >= beta } else { verify_eval <= alpha };
                    if verify_cutoff {
                        return (None, if white { beta } else { alpha });
                    }
                } else {
                    return (None, if white { beta } else { alpha });
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
            let static_eval = service.eval.calc_eval(board, config, &service.move_gen);
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
            start_time: context.start_time,
            target_time: context.target_time,
            root_moves_total: context.root_moves_total,
            root_moves_searched: context.root_moves_searched,
        };

        // Quiescence Search (depth <= 0)
        if depth <= 0 {
            stats.add_eval_nodes(1);
            if board.white_to_move && turn.gives_check {
                            } else if !board.white_to_move && turn.gives_check {
                            } else {
                                            }

            let in_check = turn.gives_check;
            let mut stand_pat = 0;
            let mut eval = if white { i16::MIN } else { i16::MAX };

            if !in_check {
                stand_pat = service.eval.calc_eval(board, config, &service.move_gen);
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

            
            let mut turns = crate::model::MoveList::new();
            if in_check {
                service.move_gen.generate_valid_moves_list(board, stats, config, &current_context, true, force_skip_validation, &mut turns);
            } else {
                service.move_gen.generate_valid_moves_list_capture(board, stats, config, &current_context, true, force_skip_validation, &mut turns);
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
                        10 | 20 => crate::pst::PIECE_EVAL_PAWN,
                        11 | 21 => crate::pst::PIECE_EVAL_ROOK,
                        12 | 22 => crate::pst::PIECE_EVAL_KNIGHT,
                        13 | 23 => crate::pst::PIECE_EVAL_BISHOP,
                        14 | 24 => crate::pst::PIECE_EVAL_QUEEN,
                        15 | 25 => crate::pst::PIECE_EVAL_KING,
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

                // SEE Pruning: Skip capture moves that lose material (SEE < 0)
                if !in_check && capture_turn.promotion == 0 {
                    if !self.see_ge(board, capture_turn, 0, config, &service.move_gen) {
                        continue;
                    }
                }

                if stats.calculated_nodes & 1023 == 0 {
                    let elapsed = context.start_time.elapsed().as_millis() as i32;
                    if let Some(target) = context.target_time {
                        let mut dynamic_target = target;
                        let searched = context.root_moves_searched;
                    let total = context.root_moves_total;
                    if true {
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
                    alpha, beta, stats, config, service, &current_context, true, false, false, &mut child_pv,
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
        let force_skip_validation = config.skip_strong_validation;
        let mut turns = crate::model::MoveList::new();
        service.move_gen.generate_valid_moves_list(board, stats, config, &current_context, true, force_skip_validation, &mut turns);

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

        // Sorting and SEE are deferred (Lazy Move Picking & Lazy SEE)

        let mut turn_counter = 0;
        let mut child_pv = [None; 128];
        let mut searched_quiet_moves = [None; 64];
        let mut quiet_count = 0;

        let mut i = 0;
        while i < turns.len {
            let mut best_idx = i;
            for j in (i + 1)..turns.len {
                if turns.moves[j].rank > turns.moves[best_idx].rank {
                    best_idx = j;
                }
            }
            turns.moves.swap(i, best_idx);

            if turns.moves[i].capture != 0 && turns.moves[i].rank >= 0 && turns.moves[i].rank < 100000 {
                if !self.see_ge(board, &turns.moves[i], 0, config, &service.move_gen) {
                    turns.moves[i].rank -= 100000;
                    continue; // rank decreased, re-evaluate this index
                }
            }

            let current_turn = &turns.moves[i];

            if stats.calculated_nodes & 1023 == 0 {
                let elapsed = context.start_time.elapsed().as_millis() as i32;
                if let Some(target) = context.target_time {
                    let mut dynamic_target = target;
                    let searched = context.root_moves_searched;
                    let total = context.root_moves_total;
                    if true {
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
                let d_idx = (depth as usize).min(63);
                let m_idx = (turn_counter as usize).min(63);
                let mut reduction = config.lmr_table[d_idx][m_idx] as i32;

                // PV-Knoten vorsichtiger reduzieren (Dämpfung um 1)
                if is_pv {
                    reduction = reduction.saturating_sub(1);
                }

                // Killer-Moves weniger stark reduzieren (Dämpfung um 1)
                let is_killer = Some(*current_turn) == killer_moves[ply as usize][0]
                    || Some(*current_turn) == killer_moves[ply as usize][1];
                if is_killer {
                    reduction = reduction.saturating_sub(1);
                }

                // Counter-Moves weniger stark reduzieren (Dämpfung um 1)
                let is_counter = Some(*current_turn) == current_context.counter_move;
                if is_counter {
                    reduction = reduction.saturating_sub(1);
                }

                // History-Koppelung: Verringere Reduktion für gute Züge, erhöhe für historisch extrem schwache
                let hist_val = history_table[current_turn.from as usize][current_turn.to as usize];
                if hist_val > 4000 {
                    reduction = reduction.saturating_sub(1);
                } else if hist_val < 500 {
                    reduction = reduction.saturating_add(1);
                }

                if reduction > 0 {
                    let clamped_reduction = reduction.clamp(1, depth - 2);
                    let reduced_depth = depth - 1 - clamped_reduction;
                    
                    if white {
                        min_max_eval = self.minimax(
                            board, current_turn, reduced_depth, !white,
                            alpha, alpha + 1, stats, config, service, &current_context,
                            true, false, false, &mut child_pv, ply + 1, killer_moves, history_table, counter_moves
                        ).1;
                        if min_max_eval <= alpha {
                            searched = true;
                        }
                    } else {
                        min_max_eval = self.minimax(
                            board, current_turn, reduced_depth, !white,
                            beta - 1, beta, stats, config, service, &current_context,
                            true, false, false, &mut child_pv, ply + 1, killer_moves, history_table, counter_moves
                        ).1;
                        if min_max_eval >= beta {
                            searched = true;
                        }
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
                                true, false, false, &mut child_pv, ply + 1, killer_moves, history_table, counter_moves
                            ).1;
                            
                            if min_max_eval > alpha && min_max_eval < beta {
                                min_max_eval = self.minimax(
                                    board, current_turn, depth - 1, !white,
                                    alpha, beta, stats, config, service, &current_context,
                                    true, false, false, &mut child_pv, ply + 1, killer_moves, history_table, counter_moves
                                ).1;
                            }
                        } else {
                            min_max_eval = self.minimax(
                                board, current_turn, depth - 1, !white,
                                beta - 1, beta, stats, config, service, &current_context,
                                true, false, false, &mut child_pv, ply + 1, killer_moves, history_table, counter_moves
                            ).1;
                            
                            if min_max_eval < beta && min_max_eval > alpha {
                                min_max_eval = self.minimax(
                                    board, current_turn, depth - 1, !white,
                                    alpha, beta, stats, config, service, &current_context,
                                    true, false, false, &mut child_pv, ply + 1, killer_moves, history_table, counter_moves
                                ).1;
                            }
                        }
                    } else {
                        min_max_eval = self.minimax(
                            board, current_turn, depth - 1, !white,
                            alpha, beta, stats, config, service, &current_context,
                            true, false, false, &mut child_pv, ply + 1, killer_moves, history_table, counter_moves
                        ).1;
                    }
                } else {
                    min_max_eval = self.minimax(
                        board, current_turn, depth - 1, !white,
                        alpha, beta, stats, config, service, &current_context,
                        true, false, false, &mut child_pv, ply + 1, killer_moves, history_table, counter_moves
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
            i += 1;
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
#[cfg(test)]
mod tests {
    use crate::config::Config;
    use crate::service::Service;
    use crate::model::{EngineState, Stats, Turn};
    use crate::zobrist::ZobristTable;
    use std::sync::Arc;

    #[test]
    fn test_dynamic_nmp_verification_search() {
        let fen = "r1bqkbnr/pppp1ppp/2n5/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3";
        let mut board = Service::new().fen.set_fen(fen);
        let service = Service::new();
        
        let (tx_log, _rx_log) = std::sync::mpsc::channel();
        let engine_state = Arc::new(EngineState {
            stop_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            debug_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            zobrist_table: std::sync::RwLock::new(Arc::new(ZobristTable::with_capacity(100_000))),
            pv_nodes: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            pv_nodes_len: Arc::new(std::sync::atomic::AtomicI32::new(0)),
            logger: Arc::new(std::sync::RwLock::new(Arc::new(|_| {}))),
            log_sender: tx_log,
        });

        // Config with NMP Enabled
        let mut config_enabled = Config::for_tests();
        config_enabled.enable_nmp = true;
        config_enabled.nmp_depth_threshold = 2;
        config_enabled.nmp_verification_threshold = 3;
        config_enabled.nmp_reduction = 2;
        config_enabled.nmp_dynamic_divisor = 6;
        
        // Config with NMP Disabled
        let mut config_disabled = Config::for_tests();
        config_disabled.enable_nmp = false;

        let mut stats_enabled = Stats::new();
        let mut stats_disabled = Stats::new();

        service.search.get_moves(
            &mut board,
            4,
            true,
            &mut stats_enabled,
            &config_enabled,
            &service,
            &engine_state,
            std::time::Instant::now(),
            None,
        );

        service.search.get_moves(
            &mut board,
            4,
            true,
            &mut stats_disabled,
            &config_disabled,
            &service,
            &engine_state,
            std::time::Instant::now(),
            None,
        );

        assert!(stats_enabled.calculated_nodes < stats_disabled.calculated_nodes,
            "Enabled NMP nodes ({}) should be strictly less than disabled NMP nodes ({})!",
            stats_enabled.calculated_nodes, stats_disabled.calculated_nodes);
    }

    #[test]
    fn test_logarithmic_lmr_table() {
        let config = Config::new();
        
        // Assert 0 reduction at boundary depths or move indexes
        assert_eq!(config.lmr_table[1][12], 0);
        assert_eq!(config.lmr_table[8][1], 0);
        
        // Assert precalculated values for normal search depth/move counts
        // ln(8) * ln(12) / 1.95 = 2.07944 * 2.4849 / 1.95 = 2.649 -> 2
        assert_eq!(config.lmr_table[8][12], 2);
        
        // ln(16) * ln(16) / 1.95 = 2.77258 * 2.77258 / 1.95 = 7.687 / 1.95 = 3.94 -> 3
        assert_eq!(config.lmr_table[16][16], 3);
        
        let mut config_conservative = Config::new();
        config_conservative.lmr_table = {
            let mut table = [[0i16; 64]; 64];
            let divisor = 2.5;
            for depth in 1..64 {
                for move_idx in 1..64 {
                    let d = depth as f64;
                    let m = move_idx as f64;
                    let reduction = (d.ln() * m.ln() / divisor) as i16;
                    table[depth][move_idx] = reduction.max(0);
                }
            }
            table
        };
        
        // ln(16) * ln(16) / 2.5 = 7.687 / 2.5 = 3.07 -> 3
        assert_eq!(config_conservative.lmr_table[16][16], 3);
    }

    #[test]
    fn test_static_exchange_evaluation() {
        let service = Service::new();
        let config = Config::new();
        
        // 1. Equal trade case (e4 Pawn captures on d5, which is protected by Black's c6 Pawn)
        let fen = "rnbqkbnr/pp1ppppp/2p5/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2";
        let board = service.fen.set_fen(fen);
        assert_eq!(board.get_piece_at(35), 20); // Black Pawn
        assert_eq!(board.get_piece_at(42), 20); // Black Pawn
        let mv = Turn {
            from: 28, // e4
            to: 35,   // d5
            capture: 20, // Black Pawn
            promotion: 0,
            rank: 0,
            gives_check: false,
            eval: 0,
            hash: 0,
            has_hashed_eval: false,
        };
        let see_val = service.search.see(&board, &mv, &config, &service.move_gen);
        assert_eq!(see_val, 0);
        
        // 2. Favorable trade (White Bishop e2 captures undefended Black Pawn d3)
        let fen2 = "rnbqkbnr/ppp1pppp/8/8/8/3p4/PPPPBPPP/RNBQK1NR w KQkq - 0 1";
        let board2 = service.fen.set_fen(fen2);
        let mv2 = Turn {
            from: 12, // e2
            to: 19,   // d3
            capture: 20,
            promotion: 0,
            rank: 0,
            gives_check: false,
            eval: 0,
            hash: 0,
            has_hashed_eval: false,
        };
        let see_val2 = service.search.see(&board2, &mv2, &config, &service.move_gen);
        assert_eq!(see_val2, crate::pst::PIECE_EVAL_PAWN);
        
        // 3. Unfavorable blunder (White Queen d1 captures Black Pawn d5 protected by Knight c6)
        let fen3 = "r1bqkbnr/ppp1pppp/2n5/3p4/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let board3 = service.fen.set_fen(fen3);
        let mv3 = Turn {
            from: 3,  // d1
            to: 35,  // d5
            capture: 20,
            promotion: 0,
            rank: 0,
            gives_check: false,
            eval: 0,
            hash: 0,
            has_hashed_eval: false,
        };
        let see_val3 = service.search.see(&board3, &mv3, &config, &service.move_gen);
        assert_eq!(see_val3, crate::pst::PIECE_EVAL_PAWN - crate::pst::PIECE_EVAL_QUEEN);
    }
}
