use rand::Rng;
use once_cell::sync::Lazy;

use crate::zobrist;
use crate::config::Config;
use crate::model::{
    Board, DataMapKey, GameStatus, Stats, Turn, SearchContext,
    WHITE_PAWN, WHITE_ROOK, WHITE_KNIGHT, WHITE_BISHOP, WHITE_QUEEN, WHITE_KING,
    BLACK_PAWN, BLACK_ROOK, BLACK_KNIGHT, BLACK_BISHOP, BLACK_QUEEN, BLACK_KING,
};
use crate::zobrist::ZobristTable;
use crate::DataMap;

static RAYS: Lazy<[[u64; 8]; 64]> = Lazy::new(|| {
    let mut rays = [[0u64; 8]; 64];
    let dxs = [0, 0, 1, -1, 1, -1, -1, 1]; // NORTH, SOUTH, EAST, WEST, NORTH_EAST, SOUTH_WEST, NORTH_WEST, SOUTH_EAST
    let dys = [1, -1, 0, 0, 1, -1, 1, -1];
    for sq in 0..64 {
        let file = (sq % 8) as i32;
        let rank = (sq / 8) as i32;
        for dir in 0..8 {
            let dx = dxs[dir];
            let dy = dys[dir];
            let mut f = file + dx;
            let mut r = rank + dy;
            let mut mask = 0u64;
            while f >= 0 && f < 8 && r >= 0 && r < 8 {
                let s = r * 8 + f;
                mask |= 1u64 << s;
                f += dx;
                r += dy;
            }
            rays[sq][dir] = mask;
        }
    }
    rays
});

static KNIGHT_ATTACKS: Lazy<[u64; 64]> = Lazy::new(|| {
    let mut attacks = [0u64; 64];
    let offsets = [
        (-2, -1), (-2, 1), (-1, -2), (-1, 2),
        (1, -2), (1, 2), (2, -1), (2, 1)
    ];
    for sq in 0..64 {
        let file = (sq % 8) as i32;
        let rank = (sq / 8) as i32;
        let mut mask = 0u64;
        for &(df, dr) in &offsets {
            let f = file + df;
            let r = rank + dr;
            if f >= 0 && f < 8 && r >= 0 && r < 8 {
                mask |= 1u64 << (r * 8 + f);
            }
        }
        attacks[sq] = mask;
    }
    attacks
});

static KING_ATTACKS: Lazy<[u64; 64]> = Lazy::new(|| {
    let mut attacks = [0u64; 64];
    for sq in 0..64 {
        let file = (sq % 8) as i32;
        let rank = (sq / 8) as i32;
        let mut mask = 0u64;
        for df in -1..=1 {
            for dr in -1..=1 {
                if df == 0 && dr == 0 { continue; }
                let f = file + df;
                let r = rank + dr;
                if f >= 0 && f < 8 && r >= 0 && r < 8 {
                    mask |= 1u64 << (r * 8 + f);
                }
            }
        }
        attacks[sq] = mask;
    }
    attacks
});

pub struct MoveGenService {}

impl MoveGenService {
    pub fn new() -> Self {
        MoveGenService {}
    }

    pub fn get_knight_attacks(&self, sq: usize) -> u64 {
        KNIGHT_ATTACKS[sq]
    }

    fn get_bishop_attacks(&self, square: usize, occupied: u64) -> u64 {
        let mut attacks = 0u64;
        // Positive: NORTH_EAST (4), NORTH_WEST (6)
        for &dir in &[4, 6] {
            let ray = RAYS[square][dir];
            let blockers = ray & occupied;
            if blockers == 0 {
                attacks |= ray;
            } else {
                let first_blocker = blockers.trailing_zeros() as usize;
                attacks |= ray & !RAYS[first_blocker][dir];
            }
        }
        // Negative: SOUTH_WEST (5), SOUTH_EAST (7)
        for &dir in &[5, 7] {
            let ray = RAYS[square][dir];
            let blockers = ray & occupied;
            if blockers == 0 {
                attacks |= ray;
            } else {
                let first_blocker = 63 - blockers.leading_zeros() as usize;
                attacks |= ray & !RAYS[first_blocker][dir];
            }
        }
        attacks
    }

    fn get_rook_attacks(&self, square: usize, occupied: u64) -> u64 {
        let mut attacks = 0u64;
        // Positive: NORTH (0), EAST (2)
        for &dir in &[0, 2] {
            let ray = RAYS[square][dir];
            let blockers = ray & occupied;
            if blockers == 0 {
                attacks |= ray;
            } else {
                let first_blocker = blockers.trailing_zeros() as usize;
                attacks |= ray & !RAYS[first_blocker][dir];
            }
        }
        // Negative: SOUTH (1), WEST (3)
        for &dir in &[1, 3] {
            let ray = RAYS[square][dir];
            let blockers = ray & occupied;
            if blockers == 0 {
                attacks |= ray;
            } else {
                let first_blocker = 63 - blockers.leading_zeros() as usize;
                attacks |= ray & !RAYS[first_blocker][dir];
            }
        }
        attacks
    }

    /// Generates a list of valid capture moves for a given board state.
    pub fn generate_valid_moves_list_capture(
        &self,
        board: &mut Board,
        stats: &mut Stats,
        config: &Config,
        context: &SearchContext,
        local_map: &DataMap,
    ) -> Vec<Turn> {
        if board.game_status != GameStatus::Normal {
            return vec![];
        }
        let move_list = self.generate_moves_list_for_piece(board, 0);
        let capture_moves: Vec<Turn> = self.get_valid_moves_from_move_list(
            &move_list,
            board,
            stats,
            config,
            true,
            context,
            local_map,
        );

        stats.add_created_capture_nodes(capture_moves.len());
        capture_moves
    }

    /// Generates a list of valid moves for a given board state.
    pub fn generate_valid_moves_list(
        &self,
        board: &mut Board,
        stats: &mut Stats,
        config: &Config,
        context: &SearchContext,
        local_map: &DataMap,
    ) -> Vec<Turn> {
        if board.game_status != GameStatus::Normal {
            return vec![];
        }
        let move_list = self.generate_moves_list_for_piece(board, 0);
        self.get_valid_moves_from_move_list(
            &move_list,
            board,
            stats,
            config,
            false,
            context,
            local_map,
        )
    }

    fn get_valid_moves_from_move_list(
        &self,
        move_list: &[i32],
        board: &mut Board,
        stats: &mut Stats,
        config: &Config,
        only_captures: bool,
        context: &SearchContext,
        local_map: &DataMap,
    ) -> Vec<Turn> {
        let mut valid_moves = Vec::with_capacity(64);
        let white_turn = board.white_to_move;
        let king_value = if white_turn { 15 } else { 25 };

        // get pv node
        let mut pv_node = None;
        if !only_captures && config.use_pv_nodes {
            if board.cached_hash == 0 {
                board.cached_hash = zobrist::gen(board);
            }
            let pv_nodes_guard = context.pv_nodes.lock().expect(crate::model::RIP_COULDN_LOCK_MUTEX);
            if let Some(pv_node_result) = pv_nodes_guard.get(&board.cached_hash) {
                pv_node = Some(*pv_node_result);
            }
        }

        let mut tt_best_move = None;
        if !only_captures && config.use_zobrist {
            if board.cached_hash == 0 {
                board.cached_hash = zobrist::gen(board);
            }
            if let Some(entry) = context.zobrist_table.get_entry(&board.cached_hash) {
                tt_best_move = entry.best_move;
            }
        }

        let zobrist_table_read = context.zobrist_table;

        for i in (0..move_list.len()).step_by(2) {
            let idx0 = move_list[i] as u8;
            let idx1 = move_list[i + 1] as u8;

            let capture = board.get_piece_at(idx1);
            if capture == 0 && only_captures {
                continue;
            }

            let mut move_turn = Turn::new(idx0, idx1, capture, 0, false, 0);

            if let Some(pv) = &pv_node {
                if *pv == move_turn {
                    move_turn.rank = config.is_pv_node_rank_bonus * 10000;
                }
            } else if let Some(tt_move) = &tt_best_move {
                if *tt_move == move_turn {
                    move_turn.rank = config.is_pv_node_rank_bonus * 10000;
                }
            }

            if move_turn.capture == 0 {
                if Some(move_turn) == context.killer_moves[0] {
                    move_turn.rank = move_turn.rank.max(20000);
                } else if Some(move_turn) == context.killer_moves[1] {
                    move_turn.rank = move_turn.rank.max(10000);
                }

                let from = move_turn.from as usize;
                let to = move_turn.to as usize;
                let history_bonus = unsafe { (*context.history_table)[from][to] } as i32;
                move_turn.rank += history_bonus;
            }

            // Check for castling
            let moved_piece = board.get_piece_at(idx0);
            if !only_captures && (moved_piece == king_value && (idx1 as i32 - idx0 as i32).abs() == 2) {
                if !self.is_valid_castling(board, white_turn, idx1 as i32) {
                    continue;
                }
            }

            move_turn.rank += match move_turn.capture {
                10 | 20 => 20000,
                11 | 21 => 50000,
                12 | 22 => 30000,
                13 | 23 => 30000,
                14 | 24 => 90000,
                _ => 0,
            };

            if move_turn.capture != 0 {
                move_turn.rank += match board.get_piece_at(move_turn.from) {
                    11 | 21 => -10000,
                    14 | 24 => -30000,
                    _ => 0,
                };
            }

            if move_turn.rank < 0 {
                move_turn.rank = 0;
            }

            // Check for promotion
            if let Some(promotion_move) = self.get_promotion_move(board, white_turn, idx0 as i32, idx1 as i32) {
                move_turn.promotion = promotion_move.promotion;
                self.validate_and_add_promotion_moves(
                    board,
                    stats,
                    &mut move_turn,
                    config,
                    &mut valid_moves,
                    white_turn,
                    zobrist_table_read,
                    local_map,
                );
            } else {
                self.validate_and_add_move(
                    board,
                    stats,
                    &mut move_turn,
                    config,
                    &mut valid_moves,
                    zobrist_table_read,
                    local_map,
                );
            }
        }

        // Add en passant moves
        if !only_captures {
            let en_passante_turns = self.get_en_passante_turns(board, white_turn);
            for mut turn in en_passante_turns {
                self.validate_and_add_move(
                    board,
                    stats,
                    &mut turn,
                    config,
                    &mut valid_moves,
                    zobrist_table_read,
                    local_map,
                );
            }
        }

        // Move sorting
        if *local_map.get_data::<bool>(DataMapKey::MoveOrderingFlag).unwrap_or(&true) {
            valid_moves.sort_unstable_by(|a, b| b.rank.cmp(&a.rank));
        } else {
            let mut rng = rand::thread_rng();
            let mut noisy_moves: Vec<(Turn, i32)> = valid_moves
                .into_iter()
                .map(|mv| {
                    let noise = rng.gen_range(-config.smp_thread_eval_noise..=config.smp_thread_eval_noise) as i32;
                    let rank_with_noise = mv.rank as i32 + noise;
                    (mv, rank_with_noise)
                })
                .collect();

            noisy_moves.sort_unstable_by(|a, b| b.1.cmp(&a.1));
            valid_moves = noisy_moves.into_iter().map(|(mv, _)| mv).collect();
        }

        // Check GameStatus
        if valid_moves.is_empty() && !only_captures {
            if !self.get_check_idx_list(board, board.white_to_move).is_empty() {
                board.game_status = if board.white_to_move {
                    GameStatus::BlackWin
                } else {
                    GameStatus::WhiteWin
                };
            } else {
                board.game_status = GameStatus::Draw;
            }
        }

        stats.add_created_nodes(valid_moves.len());
        valid_moves.truncate(config.truncate_bad_moves);
        valid_moves
    }

    fn get_en_passante_turns(&self, board: &Board, white_turn: bool) -> Vec<Turn> {
        let mut en_passante_turns = Vec::with_capacity(4);
        if board.field_for_en_passante != -1 {
            let target_piece = if white_turn { 20 } else { 10 };
            let ep_sq = board.field_for_en_passante;
            let file = ep_sq % 8;
            if white_turn {
                if file > 0 {
                    let from = ep_sq - 9;
                    if (board.bitboards[WHITE_PAWN] & (1u64 << from)) != 0 {
                        en_passante_turns.push(Turn::new(from as u8, ep_sq as u8, target_piece, 0, false, 0));
                    }
                }
                if file < 7 {
                    let from = ep_sq - 7;
                    if (board.bitboards[WHITE_PAWN] & (1u64 << from)) != 0 {
                        en_passante_turns.push(Turn::new(from as u8, ep_sq as u8, target_piece, 0, false, 0));
                    }
                }
            } else {
                if file > 0 {
                    let from = ep_sq + 7;
                    if (board.bitboards[BLACK_PAWN] & (1u64 << from)) != 0 {
                        en_passante_turns.push(Turn::new(from as u8, ep_sq as u8, target_piece, 0, false, 0));
                    }
                }
                if file < 7 {
                    let from = ep_sq + 9;
                    if (board.bitboards[BLACK_PAWN] & (1u64 << from)) != 0 {
                        en_passante_turns.push(Turn::new(from as u8, ep_sq as u8, target_piece, 0, false, 0));
                    }
                }
            }
        }
        en_passante_turns
    }

    fn validate_and_add_move(
        &self,
        board: &mut Board,
        stats: &mut Stats,
        turn: &mut Turn,
        config: &Config,
        valid_moves: &mut Vec<Turn>,
        zobrist_table_read: &ZobristTable,
        local_map: &DataMap,
    ) {
        let move_info = board.do_move(turn);
        let mut valid = true;

        if !*local_map.get_data::<bool>(DataMapKey::ForceSkipValidationFlag).unwrap_or(&false) {
            if self.gives_check(board) {
                valid = false;
            }
        }

        if valid {
            turn.hash = board.cached_hash;
            if let Some(eval) = self.get_hash(board, config, zobrist_table_read) {
                turn.eval = eval;
                turn.has_hashed_eval = true;
                turn.rank += config.is_hashed_rank_bonus * 10000;
                stats.add_zobrist_hit(1);
            }

            if self.is_in_check(board) {
                turn.gives_check = true;
                turn.rank += config.give_check_rank_bonus * 10000;
            }
            valid_moves.push(*turn);
        }
        board.undo_move(turn, move_info);
    }

    fn validate_and_add_promotion_moves(
        &self,
        board: &mut Board,
        stats: &mut Stats,
        turn: &mut Turn,
        config: &Config,
        valid_moves: &mut Vec<Turn>,
        white_turn: bool,
        zobrist_table_read: &ZobristTable,
        local_map: &DataMap,
    ) {
        let promotion_types = if white_turn { [12, 14] } else { [22, 24] };
        for &promotion in &promotion_types {
            turn.promotion = promotion;
            match promotion {
                12 | 22 => turn.rank += config.give_promotion_rank_bonus_knight * 10000,
                14 | 24 => turn.rank += config.give_promotion_rank_bonus_queen * 10000,
                _ => panic!("Promotion value not expected"),
            }
            self.validate_and_add_move(board, stats, turn, config, valid_moves, zobrist_table_read, local_map);
        }
    }

    fn is_valid_castling(&self, board: &Board, white_turn: bool, target: i32) -> bool {
        let check_squares = if white_turn {
            if target == 6 { vec![5, 6] } else { vec![3, 2] }
        } else {
            if target == 62 { vec![61, 62] } else { vec![59, 58] }
        };

        if !self.get_check_idx_list(board, white_turn).is_empty() {
            return false;
        }

        for &square in &check_squares {
            if !self.get_attack_idx_list(board, white_turn, square).is_empty() {
                return false;
            }
        }

        if white_turn {
            if target == 6 && !board.white_possible_to_castle_short {
                return false;
            }
            if target == 2 && !board.white_possible_to_castle_long {
                return false;
            }
        } else {
            if target == 62 && !board.black_possible_to_castle_short {
                return false;
            }
            if target == 58 && !board.black_possible_to_castle_long {
                return false;
            }
        }
        true
    }

    fn get_promotion_move(&self, board: &Board, white_turn: bool, idx0: i32, idx1: i32) -> Option<Turn> {
        if white_turn && idx0 / 8 == 6 && board.get_piece_at(idx0 as u8) == 10 {
            Some(Turn {
                from: idx0 as u8,
                to: idx1 as u8,
                capture: 0,
                promotion: 14,
                gives_check: false,
                eval: 0,
                hash: 0,
                has_hashed_eval: false,
                rank: 0,
            })
        } else if !white_turn && idx0 / 8 == 1 && board.get_piece_at(idx0 as u8) == 20 {
            Some(Turn {
                from: idx0 as u8,
                to: idx1 as u8,
                capture: 0,
                promotion: 24,
                gives_check: false,
                eval: 0,
                hash: 0,
                has_hashed_eval: false,
                rank: 0,
            })
        } else {
            None
        }
    }

    fn get_hash(&self, board: &mut Board, config: &Config, zobrist_table_read: &ZobristTable) -> Option<i16> {
        if !config.use_zobrist {
            return None;
        }
        zobrist_table_read.get_eval_for_hash(&board.cached_hash)
    }

    pub fn generate_moves_list_for_piece(&self, board: &Board, idx: i32) -> Vec<i32> {
        let white = board.white_to_move;
        let check_idx_list = self.get_check_idx_list(board, white);
        let double_check = check_idx_list.len() > 1;

        let own_pieces = if white { board.white_pieces } else { board.black_pieces };
        let opp_pieces = if white { board.black_pieces } else { board.white_pieces };
        let occupied = board.occupied;

        let mut moves = Vec::with_capacity(64);

        let single_sq = if idx > 0 { Some(idx as u8) } else { None };
        let piece_range = if let Some(sq) = single_sq {
            sq..=sq
        } else {
            0..=63
        };

        for sq in piece_range {
            if double_check {
                let king_sq = if white {
                    board.bitboards[WHITE_KING].trailing_zeros()
                } else {
                    board.bitboards[BLACK_KING].trailing_zeros()
                } as u8;
                if sq != king_sq {
                    continue;
                }
            }

            let piece = board.get_piece_at(sq);
            if piece == 0 {
                continue;
            }

            let is_white_piece = piece >= 10 && piece <= 15;
            if is_white_piece != white {
                continue;
            }

            match piece {
                10 | 20 => {
                    let rank = sq / 8;
                    let file = sq % 8;
                    if white {
                        let to = sq + 8;
                        if to < 64 && (occupied & (1u64 << to)) == 0 {
                            moves.push(sq as i32);
                            moves.push(to as i32);
                            if rank == 1 {
                                let to_double = sq + 16;
                                if (occupied & (1u64 << to_double)) == 0 {
                                    moves.push(sq as i32);
                                    moves.push(to_double as i32);
                                }
                            }
                        }
                        if file > 0 {
                            let to = sq + 7;
                            if to < 64 && (opp_pieces & (1u64 << to)) != 0 {
                                moves.push(sq as i32);
                                moves.push(to as i32);
                            }
                        }
                        if file < 7 {
                            let to = sq + 9;
                            if to < 64 && (opp_pieces & (1u64 << to)) != 0 {
                                moves.push(sq as i32);
                                moves.push(to as i32);
                            }
                        }
                    } else {
                        let to = sq - 8;
                        if (occupied & (1u64 << to)) == 0 {
                            moves.push(sq as i32);
                            moves.push(to as i32);
                            if rank == 6 {
                                let to_double = sq - 16;
                                if (occupied & (1u64 << to_double)) == 0 {
                                    moves.push(sq as i32);
                                    moves.push(to_double as i32);
                                }
                            }
                        }
                        if file > 0 {
                            let to = sq - 9;
                            if (opp_pieces & (1u64 << to)) != 0 {
                                moves.push(sq as i32);
                                moves.push(to as i32);
                            }
                        }
                        if file < 7 {
                            let to = sq - 7;
                            if (opp_pieces & (1u64 << to)) != 0 {
                                moves.push(sq as i32);
                                moves.push(to as i32);
                            }
                        }
                    }
                }
                12 | 22 => {
                    let mut targets = KNIGHT_ATTACKS[sq as usize] & !own_pieces;
                    while targets != 0 {
                        let to = targets.trailing_zeros() as u8;
                        moves.push(sq as i32);
                        moves.push(to as i32);
                        targets &= targets - 1;
                    }
                }
                13 | 23 => {
                    let mut targets = self.get_bishop_attacks(sq as usize, occupied) & !own_pieces;
                    while targets != 0 {
                        let to = targets.trailing_zeros() as u8;
                        moves.push(sq as i32);
                        moves.push(to as i32);
                        targets &= targets - 1;
                    }
                }
                11 | 21 => {
                    let mut targets = self.get_rook_attacks(sq as usize, occupied) & !own_pieces;
                    while targets != 0 {
                        let to = targets.trailing_zeros() as u8;
                        moves.push(sq as i32);
                        moves.push(to as i32);
                        targets &= targets - 1;
                    }
                }
                14 | 24 => {
                    let bishop_attacks = self.get_bishop_attacks(sq as usize, occupied);
                    let rook_attacks = self.get_rook_attacks(sq as usize, occupied);
                    let mut targets = (bishop_attacks | rook_attacks) & !own_pieces;
                    while targets != 0 {
                        let to = targets.trailing_zeros() as u8;
                        moves.push(sq as i32);
                        moves.push(to as i32);
                        targets &= targets - 1;
                    }
                }
                15 | 25 => {
                    let mut targets = KING_ATTACKS[sq as usize] & !own_pieces;
                    while targets != 0 {
                        let to = targets.trailing_zeros() as u8;
                        let opp_king_sq = if white {
                            board.bitboards[BLACK_KING].trailing_zeros()
                        } else {
                            board.bitboards[WHITE_KING].trailing_zeros()
                        } as usize;
                        if opp_king_sq < 64 && (KING_ATTACKS[to as usize] & (1u64 << opp_king_sq)) != 0 {
                            targets &= targets - 1;
                            continue;
                        }
                        moves.push(sq as i32);
                        moves.push(to as i32);
                        targets &= targets - 1;
                    }

                    if white {
                        if sq == 4 {
                            if board.white_possible_to_castle_short
                                && (occupied & ((1u64 << 5) | (1u64 << 6))) == 0
                                && (board.bitboards[WHITE_ROOK] & (1u64 << 7)) != 0
                            {
                                moves.push(4);
                                moves.push(6);
                            }
                            if board.white_possible_to_castle_long
                                && (occupied & ((1u64 << 1) | (1u64 << 2) | (1u64 << 3))) == 0
                                && (board.bitboards[WHITE_ROOK] & (1u64 << 0)) != 0
                            {
                                moves.push(4);
                                moves.push(2);
                            }
                        }
                    } else {
                        if sq == 60 {
                            if board.black_possible_to_castle_short
                                && (occupied & ((1u64 << 61) | (1u64 << 62))) == 0
                                && (board.bitboards[BLACK_ROOK] & (1u64 << 63)) != 0
                            {
                                moves.push(60);
                                moves.push(62);
                            }
                            if board.black_possible_to_castle_long
                                && (occupied & ((1u64 << 57) | (1u64 << 58) | (1u64 << 59))) == 0
                                && (board.bitboards[BLACK_ROOK] & (1u64 << 56)) != 0
                            {
                                moves.push(60);
                                moves.push(58);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        moves
    }

    fn get_attackers_mask(&self, board: &Board, white: bool, target_idx: u8, occupied: u64) -> u64 {
        let mut attackers = 0u64;
        let opp_pawn = if white { BLACK_PAWN } else { WHITE_PAWN };
        let opp_knight = if white { BLACK_KNIGHT } else { WHITE_KNIGHT };
        let opp_bishop = if white { BLACK_BISHOP } else { WHITE_BISHOP };
        let opp_rook = if white { BLACK_ROOK } else { WHITE_ROOK };
        let opp_queen = if white { BLACK_QUEEN } else { WHITE_QUEEN };
        let opp_king = if white { BLACK_KING } else { WHITE_KING };

        // Pawns
        let file = target_idx % 8;
        if white {
            if file > 0 && target_idx <= 56 {
                let sq = target_idx + 7;
                if (board.bitboards[opp_pawn] & (1u64 << sq)) != 0 {
                    attackers |= 1u64 << sq;
                }
            }
            if file < 7 && target_idx <= 54 {
                let sq = target_idx + 9;
                if (board.bitboards[opp_pawn] & (1u64 << sq)) != 0 {
                    attackers |= 1u64 << sq;
                }
            }
        } else {
            if file > 0 && target_idx >= 9 {
                let sq = target_idx - 9;
                if (board.bitboards[opp_pawn] & (1u64 << sq)) != 0 {
                    attackers |= 1u64 << sq;
                }
            }
            if file < 7 && target_idx >= 7 {
                let sq = target_idx - 7;
                if (board.bitboards[opp_pawn] & (1u64 << sq)) != 0 {
                    attackers |= 1u64 << sq;
                }
            }
        }

        // Knights
        let knight_attacks = KNIGHT_ATTACKS[target_idx as usize];
        attackers |= knight_attacks & board.bitboards[opp_knight];

        // King
        let king_attacks = KING_ATTACKS[target_idx as usize];
        attackers |= king_attacks & board.bitboards[opp_king];

        // Bishop / Queen diagonals
        let diag_attacks = self.get_bishop_attacks(target_idx as usize, occupied);
        attackers |= diag_attacks & (board.bitboards[opp_bishop] | board.bitboards[opp_queen]);

        // Rook / Queen straights
        let straight_attacks = self.get_rook_attacks(target_idx as usize, occupied);
        attackers |= straight_attacks & (board.bitboards[opp_rook] | board.bitboards[opp_queen]);

        attackers
    }

    /// Checks which OPPONENT pieces attack the given target_idx.
    pub fn get_attack_idx_list(&self, board: &Board, white: bool, target_idx: i32) -> Vec<i32> {
        if target_idx == -1 {
            return Vec::new();
        }
        let attackers_mask = self.get_attackers_mask(board, white, target_idx as u8, board.occupied);
        let mut attackers = Vec::new();
        let mut temp = attackers_mask;
        while temp != 0 {
            let sq = temp.trailing_zeros() as i32;
            attackers.push(sq);
            temp &= temp - 1;
        }
        attackers
    }

    /// Checks if the target index is under shadow attack.
    pub fn get_attack_idx_list_with_shadow(&self, board: &Board, white: bool, target_idx: i32) -> Vec<i32> {
        let mut current_occupied = board.occupied;
        let mut all_attackers = Vec::default();

        let mut attackers_mask = self.get_attackers_mask(board, white, target_idx as u8, current_occupied);
        while attackers_mask != 0 {
            let attacker = attackers_mask.trailing_zeros() as u8;
            all_attackers.push(attacker as i32);
            current_occupied &= !(1u64 << attacker);
            attackers_mask = self.get_attackers_mask(board, white, target_idx as u8, current_occupied);
            for &found in &all_attackers {
                attackers_mask &= !(1u64 << found);
            }
        }
        all_attackers
    }

    /// Checks if the king is under attack.
    pub fn get_check_idx_list(&self, board: &Board, white: bool) -> Vec<i32> {
        let king_pos = if white {
            board.get_king_positions().0
        } else {
            board.get_king_positions().1
        };
        self.get_attack_idx_list(board, white, king_pos)
    }

    /// Returns true if the side to move gives check, otherwise false.
    pub fn gives_check(&self, board: &Board) -> bool {
        self.__check_check(board, true)
    }

    /// Returns true if the king of side to move is in check, otherwise false.
    pub fn is_in_check(&self, board: &Board) -> bool {
        self.__check_check(board, false)
    }

    pub fn __check_check(&self, board: &Board, inverse: bool) -> bool {
        let king_positions = board.get_king_positions();
        let white = if inverse {
            !board.white_to_move
        } else {
            board.white_to_move
        };
        let target_idx = if white {
            king_positions.0
        } else {
            king_positions.1
        };
        if target_idx == -1 {
            return false;
        }
        let attackers_mask = self.get_attackers_mask(board, white, target_idx as u8, board.occupied);
        attackers_mask != 0
    }
}

#[cfg(test)]
mod tests {
    use crate::notation_util::NotationUtil;
    use crate::service::Service;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use super::*;

    // Test-only mailbox-to-lerf translator to preserve all test coordinates
    fn m2l(sq: i32) -> i32 {
        if sq < 0 {
            return sq;
        }
        let row = sq / 10;
        let col = sq % 10;
        if row < 2 || row > 9 || col < 1 || col > 8 {
            panic!("Invalid mailbox index: {}", sq);
        }
        (9 - row) * 8 + (col - 1)
    }

    fn generate_valid_moves_list(board: &mut Board) -> Vec<Turn> {
        let service = Service::new();
        let local_map = DataMap::new();
        let config = Config::for_tests();
        let zobrist_table = ZobristTable::new();
        let stop_flag = std::sync::atomic::AtomicBool::new(false);
        let pv_nodes = std::sync::Mutex::new(std::collections::HashMap::new());
        let history_table = [[0u32; 64]; 64];
        let context = SearchContext {
            zobrist_table: &zobrist_table,
            stop_flag: &stop_flag,
            pv_nodes: &pv_nodes,
            killer_moves: [None; 2],
            history_table: &history_table,
        };

        service.move_gen.generate_valid_moves_list(board, &mut Stats::new(), &config, &context, &local_map)
    }

    fn generate_valid_moves_list_capture(board: &mut Board) -> Vec<Turn> {
        let service = Service::new();
        let local_map = DataMap::new();
        let config = Config::for_tests();
        let zobrist_table = ZobristTable::new();
        let stop_flag = std::sync::atomic::AtomicBool::new(false);
        let pv_nodes = std::sync::Mutex::new(std::collections::HashMap::new());
        let history_table = [[0u32; 64]; 64];
        let context = SearchContext {
            zobrist_table: &zobrist_table,
            stop_flag: &stop_flag,
            pv_nodes: &pv_nodes,
            killer_moves: [None; 2],
            history_table: &history_table,
        };

        service.move_gen.generate_valid_moves_list_capture(board, &mut Stats::new(), &config, &context, &local_map)
    }

    #[test]
    fn get_check_idx_list_test() {
        let fen_service = Service::new().fen;
        let move_gen_service = Service::new().move_gen;

        // Test 1: Initial Board Setup - No Check
        let mut board = fen_service.set_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        assert!(move_gen_service.get_check_idx_list(&board, board.white_to_move).is_empty());

        // Test 2: Scenario where check occurs
        board = fen_service.set_fen("r1bqk1nr/pppp2pp/4p3/8/1b3P2/3PPn2/PPP2pPP/RNBQKBNR w KQkq - 0 1");
        let check_idx_list = move_gen_service.get_check_idx_list(&board, board.white_to_move);
        assert!(check_idx_list.contains(&m2l(86)), "Check index list should contain f2");
        assert!(check_idx_list.contains(&m2l(76)), "Check index list should contain f3");
        assert!(check_idx_list.contains(&m2l(62)), "Check index list should contain b4");

        // Test 3: Black turn - No check
        board = fen_service.set_fen("r1bqk1nr/pppp2pp/4p3/8/1b3P2/3PPn2/PPP2pPP/RNBQKBNR b KQkq - 0 1");
        assert!(move_gen_service.get_check_idx_list(&board, board.white_to_move).is_empty());

        // Test 4: Two checks, positions 36 and 37 (mapped to mailbox)
        board = fen_service.set_fen("r1bqk1nr/pppp1PNp/4p3/1Q5B/1b3P2/3PPn2/PPP2p1P/RN2KB1R b KQkq - 0 1");
        let check_idx_list = move_gen_service.get_check_idx_list(&board, board.white_to_move);
        assert_eq!(check_idx_list.len(), 2);
        assert!(check_idx_list.contains(&m2l(36)), "Check index list should contain f7");
        assert!(check_idx_list.contains(&m2l(37)), "Check index list should contain g7");

        // Test 5: Four checks in various positions
        board = fen_service.set_fen("r1Rqk2r/pppP2Np/5n2/3p3B/1b2QP2/3PPn2/PPP2p1P/RN2KB2 b KQkq - 0 1");
        let check_idx_list = move_gen_service.get_check_idx_list(&board, board.white_to_move);
        assert_eq!(check_idx_list.len(), 4);
        assert!(check_idx_list.contains(&m2l(37)), "Check index list should contain g7");
        assert!(check_idx_list.contains(&m2l(58)), "Check index list should contain h5");
        assert!(check_idx_list.contains(&m2l(65)), "Check index list should contain e4");
        assert!(check_idx_list.contains(&m2l(34)), "Check index list should contain d7");

        // Test 6: Four checks in another scenario
        board = fen_service.set_fen("2B5/6N1/4k3/8/2K2NP1/1B2Q2B/4R3/8 b - - 0 1");
        let check_idx_list = move_gen_service.get_check_idx_list(&board, board.white_to_move);
        assert!(check_idx_list.contains(&m2l(23)), "Check index list should contain c4");
        assert!(check_idx_list.contains(&m2l(37)), "Check index list should contain g4");
        assert!(check_idx_list.contains(&m2l(66)), "Check index list should contain f4");
        assert!(check_idx_list.contains(&m2l(75)), "Check index list should contain e3");
        assert_eq!(check_idx_list.len(), 4);

        // Test 7: Last check scenario with two checks
        board = fen_service.set_fen("8/1k6/8/1q6/2b1r3/8/1rn1K3/8 w - - 0 1");
        let check_idx_list = move_gen_service.get_check_idx_list(&board, board.white_to_move);
        assert!(check_idx_list.contains(&m2l(63)), "Check index list should contain c4");
        assert!(check_idx_list.contains(&m2l(65)), "Check index list should contain e4");
        assert_eq!(check_idx_list.len(), 2);
    }

    #[test]
    fn get_attack_idx_list_test() {
        let fen_service = Service::new().fen;
        let move_gen_service = Service::new().move_gen;

        let board = fen_service.set_fen("r1q2r1k/1pp1bpp1/p2p1n2/4P2p/2Q2B2/2N4P/PPPR1PP1/3R2K1 b - - 3 16");
        let attacks = move_gen_service.get_attack_idx_list(&board, board.white_to_move, m2l(44)).len() as i32;
        assert_eq!(2, attacks);

        let board = fen_service.set_fen("r1bqnr2/pp1nbpk1/2p1p3/3p2pp/2PP1P1N/2NBP1B1/PPQ3PP/2R1K2R w K - 0 14");
        let attacks = move_gen_service.get_attack_idx_list(&board, board.white_to_move, m2l(68)).len() as i32;
        assert_eq!(1, attacks);

        let board = fen_service.set_fen("r2qkb1r/pppp1ppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1");
        let attacks = move_gen_service.get_attack_idx_list(&board, board.white_to_move, m2l(24)).len() as i32;
        assert_eq!(0, attacks);
    }

    #[test]
    fn get_attack_idx_list_with_shadow_test() {
        let fen_service = Service::new().fen;
        let move_gen_service = Service::new().move_gen;

        let board = fen_service.set_fen("r1q2r1k/1pp1bpp1/p2p1n2/4P2p/2Q2B2/2N4P/PPPR1PP1/3R2K1 b - - 3 16");
        let attacks = move_gen_service.get_attack_idx_list_with_shadow(&board, board.white_to_move, m2l(44)).len() as i32;
        assert_eq!(4, attacks);

        let board = fen_service.set_fen("r1bqnr2/pp1nbpk1/2p1p3/3p2pp/2PP1P1N/2NBP1B1/PPQ3PP/2R1K2R w K - 0 14");
        let attacks = move_gen_service.get_attack_idx_list_with_shadow(&board, board.white_to_move, m2l(68)).len() as i32;
        assert_eq!(3, attacks);
    }

    #[test]
    fn generate_moves_list_for_fen_test() {
        let fen_service = Service::new().fen;
        let move_gen_service = Service::new().move_gen;

        // Test: Standard starting position
        let board = fen_service.set_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        let moves = move_gen_service.generate_moves_list_for_piece(&board, 0);

        let expected_moves_mailbox = vec![
            81, 71, 81, 61, 82, 72, 82, 62, 83, 73, 83, 63, 84, 74, 84, 64,
            85, 75, 85, 65, 86, 76, 86, 66, 87, 77, 87, 67, 88, 78, 88, 68,
            92, 71, 92, 73, 97, 76, 97, 78
        ];
        let expected_moves: Vec<i32> = expected_moves_mailbox.into_iter().map(m2l).collect();
        // Since the bitboard move list ordering might differ from mailbox ordering, we sort them before asserting
        let mut sorted_moves = moves.clone();
        let mut sorted_expected = expected_moves.clone();
        sorted_moves.sort();
        sorted_expected.sort();
        assert_eq!(sorted_moves, sorted_expected, "Move Gen for start-up setup is wrong");

        // Test: White in check and only a few moves are available for the king
        let board = fen_service.set_fen("rnbqk2r/pppp1ppp/4p3/8/1b6/3P1n1B/PPP1PPPP/RNBQK1NR w KQkq - 0 1");
        let moves = move_gen_service.generate_moves_list_for_piece(&board, 0);

        let expected_moves_in_check_mailbox = vec![95, 84, 95, 96];
        let mut expected_moves_in_check: Vec<i32> = expected_moves_in_check_mailbox.into_iter().map(m2l).collect();
        let mut sorted_moves_in_check = moves.clone();
        sorted_moves_in_check.sort();
        expected_moves_in_check.sort();
        assert_eq!(sorted_moves_in_check, expected_moves_in_check, "Check list is not working");
    }

    #[test]
    fn get_valid_moves_from_move_list_test_double_check() {
        let service = &Service::new();

        // Double check, only one king move is possible for white
        let mut board = service.fen.set_fen("rnbqk2r/pppp1ppp/4p3/8/1b6/3P1n1B/PPP1PPPP/RNBQK1NR w KQkq - 0 1");
        let valid_turn_list = generate_valid_moves_list(&mut board);
        assert_eq!(valid_turn_list.len(), 1);
        assert!(valid_turn_list[0].from == m2l(95) as u8 && valid_turn_list[0].to == m2l(96) as u8);

        // Double check, only one king move is possible for black
        let mut board = service.fen.set_fen("rnb1k1nr/ppppp1Np/2N5/7Q/8/4P3/PPPP1PPP/RNB1KB1R b KQkq - 0 1");
        let valid_turn_list = generate_valid_moves_list(&mut board);
        assert_eq!(valid_turn_list.len(), 1);
        assert!(valid_turn_list[0].from == m2l(25) as u8 && valid_turn_list[0].to == m2l(26) as u8);
    }

    #[test]
    fn get_valid_moves_when_in_check_easy() {
        test_fen("rnbqk1nr/pppp1ppp/4p3/8/1b6/3P1P2/PPP1P1PP/RNBQKBNR w KQkq - 1 3", 6);
        test_fen("rnbqkb1r/pppppppp/8/8/8/5n2/PPPPQ1PP/RNB1KBNR w KQkq - 0 1", 5);
        test_fen("8/5k2/3r4/5n2/3N4/3K4/1q6/8 w - - 0 1", 2);
        test_fen("8/5k2/3r4/5n2/8/3K1N2/1q6/8 w - - 0 1", 3);

        test_fen("rnbqkbnr/ppp1pppp/3p4/1B6/4P3/8/PPPP1PPP/RNBQK1NR b KQkq - 1 2", 5);
        test_fen("r1bqkbnr/pp1npppp/2Bp4/8/4P3/3P4/PPP2PPP/RNBQK1NR b KQkq - 0 4", 20);
        test_fen("rnbqk2r/pppp2pp/5p1n/2b1p2Q/2B1P3/P6N/1PPP1PPP/RNB1K2R b KQkq - 1 5", 4);
        test_fen("8/8/3k1N1p/3b4/3Q4/8/4R3/3K4 b - - 0 1", 3);
    }

    #[test]
    fn castling_test() {
        test_fen_with_move("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1", 25, "e1g1");
        test_fen_with_move("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1", 25, "e8g8");
        test_fen_with_move("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w Kk - 0 1", 24, "e1g1");
        test_fen_with_move("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b Kk - 0 1", 24, "e8g8");
        test_fen_with_move("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w Qq - 0 1", 24, "e1c1");
        test_fen_with_move("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b Qq - 0 1", 24, "e8c8");
        test_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w - - 0 1", 23);
        test_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b - - 0 1", 23);
        test_fen("r3k2r/pppppppp/8/8/8/1n4n1/PPPPPPPP/R3K2R w KQkq - 0 1", 22);
        test_fen("r3k2r/pppppppp/1N4N1/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1", 22);
    }

    #[test]
    fn promotion_test() {
        test_fen_with_move("5n2/4P3/8/2k5/8/8/2K5/8 w - - 0 1", 12, "e7f8q");

        let fen_service = Service::new().fen;

        let mut board = fen_service.set_fen("5n2/4P3/8/2k5/8/8/2K5/8 w - - 0 1");
        let board_copy = board.clone();

        let mut promotion_move = NotationUtil::get_turn_from_notation("e7f8q");
        promotion_move.capture = 22;
        let move_info = board.do_move(&promotion_move);

        assert_eq!(board.get_piece_at(m2l(26) as u8), 14, "White promotion should result in a queen (14)");
        board.undo_move(&promotion_move, move_info);
        assert_eq!(board.get_piece_at(m2l(26) as u8), 22, "Piece should revert to captured knight (22)");
        assert_eq!(board.get_piece_at(m2l(35) as u8), 10, "White pawn should be back at e7");
        assert_eq!(board, board_copy, "Board should be restored");

        let mut board = fen_service.set_fen("8/2p2p1p/p4kp1/1r6/8/K7/3p4/8 b - - 1 51");
        let turns = generate_valid_moves_list(&mut board);
        let promotion_move = turns.iter().find(|t| t.is_promotion()).expect("No Promotion move found");
        assert_eq!(24, promotion_move.promotion);
        let mi = board.do_move(promotion_move);
        assert_eq!(24, board.get_piece_at(m2l(94) as u8));
        assert_eq!(0, board.get_piece_at(m2l(84) as u8));

        board.undo_move(promotion_move, mi);
        assert_eq!(0, board.get_piece_at(m2l(94) as u8));
        assert_eq!(20, board.get_piece_at(m2l(84) as u8));

        let move_from_notation_util = NotationUtil::get_turn_from_notation("d2d1q");
        board.do_move(&move_from_notation_util);
        assert_eq!(24, board.get_piece_at(m2l(94) as u8));
        assert_eq!(0, board.get_piece_at(m2l(84) as u8));

        let move_list = test_fen_with_move("8/2k1P3/8/7b/8/4b3/2K3n1/8 w - - 0 1", 7, "e7e8n");
        assert_eq!(move_list.len(), 8, "Expected 8 moves after knight promotion");

        let move_list = test_fen_with_move("8/2k1P3/8/7b/8/4b3/2K3n1/8 w - - 0 1", 7, "e7e8q");
        assert_eq!(move_list.len(), 24, "Expected 24 moves after queen promotion");

        let move_list = test_fen_with_move("5k2/R6P/8/8/8/2K5/8/6r1 w - - 0 1", 23, "h7h8q");
        assert_eq!(move_list.len(), 1, "Expected 1 move after queen promotion on h8");

        let move_list = test_fen_with_move("8/8/8/8/8/1K6/5p2/4k3 b - - 0 1", 6, "f2f1q");
        assert_eq!(move_list.len(), 7, "Expected 7 moves after black queen promotion");

        let move_list = test_fen_with_move("8/8/8/8/8/1K6/5p2/4k3 b - - 0 1", 6, "f2f1n");
        assert_eq!(move_list.len(), 8, "Expected 8 moves after black knight promotion");

        let move_list = test_fen_with_move("8/8/3k4/8/8/8/1K5p/8 b - - 0 1", 10, "h2h1q");
        assert_eq!(move_list.len(), 5, "Expected 5 moves after black queen promotion on h1");
    }

    #[test]
    fn move_list_sort_test() {
        let fen_service = Service::new().fen;

        let mut board = fen_service.set_fen("r1bqk2r/pppp1ppp/2n2n2/2b5/2BpP3/2P2N2/PP3PPP/RNBQK2R w KQkq - 0 6");
        let move_list = generate_valid_moves_list(&mut board);
        assert!(move_list.get(0).unwrap().from == m2l(63) as u8 && move_list.get(0).unwrap().to == m2l(36) as u8);
        assert!(move_list.last().unwrap().rank == 0);

        let mut board = fen_service.set_fen("r1bqr1k1/ppp2pp1/2n2n1p/2bp4/2B1PB2/1NP4P/PP3PP1/RN1Q1RK1 b - - 1 10");
        let move_list = generate_valid_moves_list(&mut board);
        assert!(move_list.get(0).unwrap().from == m2l(53) as u8 && move_list.get(0).unwrap().to == m2l(86) as u8);
        assert!(move_list.get(1).unwrap().from == m2l(54) as u8 && move_list.get(1).unwrap().to == m2l(63) as u8);
        assert!(move_list.last().unwrap().rank == 0);
    }

    #[test]
    fn game_status_check_mate_test() {
        let board = test_fen("rnbqkbnr/ppppp2p/8/7B/8/8/PPPPPPPP/RNBQK1NR b KQkq - 0 1", 0);
        assert!(board.game_status == GameStatus::WhiteWin);

        let board = test_fen("3N3B/8/6k1/4K3/8/6RR/8/8 b - - 0 1", 0);
        assert!(board.game_status == GameStatus::WhiteWin);

        let board = test_fen("rn2k1nr/pppppppp/3b4/1b6/4P1PN/1B6/PPPP1P1q/RNBQR1K1 w Qkq - 0 1", 0);
        assert!(board.game_status == GameStatus::BlackWin);

        let board = test_fen("4r3/8/8/8/b4n1b/4p3/1k1K4/8 w - - 0 1", 0);
        assert!(board.game_status == GameStatus::BlackWin);

        let board = test_fen("3R3k/6pp/8/8/4P3/8/6PP/7K b - - 0 1", 0);
        assert!(board.game_status == GameStatus::WhiteWin);
    }

    #[test]
    fn game_status_pat_test() {
        let board = test_fen("3N3B/8/6k1/4K3/5P2/7R/8/8 b - - 0 1", 0);
        assert!(board.game_status == GameStatus::Draw);

        let board = test_fen("R7/R2pk3/Q5P1/8/8/8/4K3/8 b - - 0 1", 0);
        assert!(board.game_status == GameStatus::Draw);

        let board = test_fen("8/8/8/8/4k3/2p1n1p1/4K1n1/8 w - - 0 1", 0);
        assert!(board.game_status == GameStatus::Draw);

        let board = test_fen("8/8/8/8/3k2p1/8/r2PKN1r/r7 w - - 0 1", 0);
        assert!(board.game_status == GameStatus::Draw);
    }

    #[test]
    fn get_check_idx_list() {
        test_fen("8/1P4k1/1K5p/4p2P/4r3/8/8/6q1 w - - 0 59", 5);
    }

    #[test]
    fn hit_moves_count_and_undo_test() {
        let fen_service = Service::new().fen;

        let mut board = fen_service.set_fen("r1bqr1k1/2p2ppp/p1np1n2/1pb1p1N1/2BPP3/2P1B3/PPQ2PPP/RN3RK1 w - - 0 10");
        let capture_moves = generate_valid_moves_list_capture(&mut board);
        assert_eq!(6, capture_moves.len());
        let copy_board = board.clone();
        let capture_move = capture_moves.get(0).unwrap();
        let mi = board.do_move(capture_move);
        board.undo_move(capture_move, mi);
        assert_eq!(copy_board, board);

        let mut board = fen_service.set_fen("r1bqr1k1/2p2ppp/p1np1n2/1pb1p1N1/2BPP3/2P1B3/PPQ2PPP/RN3RK1 b - - 0 10");
        let capture_moves = generate_valid_moves_list_capture(&mut board);
        assert_eq!(5, capture_moves.len());
        let copy_board = board.clone();
        let capture_move = capture_moves.get(0).unwrap();
        let mi = board.do_move(capture_move);
        board.undo_move(capture_move, mi);
        assert_eq!(copy_board, board);
    }

    #[test]
    fn en_passante_test() {
        let fen_service = Service::new().fen;

        let board = fen_service.set_fen("rnbqkbnr/ppp1ppp1/8/3pP2p/8/7P/PPPP1PP1/RNBQKBNR w KQkq d6 0 4");
        assert_eq!(m2l(44) as i8, board.field_for_en_passante);

        let board = fen_service.set_fen("rnbqkbnr/ppp1pppp/8/8/3pP2P/6P1/PPPP1P2/RNBQKBNR b KQkq e3 0 3");
        assert_eq!(m2l(75) as i8, board.field_for_en_passante);

        let truncate = Config::new().truncate_bad_moves;

        test_fen_with_move("rnbqkbnr/pp1ppp2/7p/1PpP2p1/8/8/P1P1PPPP/RNBQKBNR w KQkq c6 0 5", 31.min(truncate), "b5c6");
        test_fen_with_move("rnbqkbnr/pp1ppp2/7p/1PpP2p1/8/8/P1P1PPPP/RNBQKBNR w KQkq c6 0 5", 31.min(truncate), "d5c6");
        test_fen("rnbqkbnr/pp1ppp2/7p/1PpP2p1/8/8/P1P1PPPP/RNBQKBNR w - KQkq 0 5", 29);

        test_fen_with_move("rnbqkbnr/ppp1p1pp/8/8/3pPp1P/PP6/2PP1PP1/RNBQKBNR b KQkq e3 0 5", 31.min(truncate), "d4e3");
        test_fen_with_move("rnbqkbnr/ppp1p1pp/8/8/3pPp1P/PP6/2PP1PP1/RNBQKBNR b KQkq e3 0 5", 31.min(truncate), "f4e3");
        test_fen("rnbqkbnr/ppp1p1pp/8/8/3pPp1P/PP6/2PP1PP1/RNBQKBNR b KQkq - 0 5", 29);
    }

    #[test]
    fn check_moves_when_in_check() {
        let service = Service::new();
        let board = &mut service.fen.set_fen("7r/p1p2p1p/P3k1p1/2K2r2/2P5/8/8/8 w - - 0 36");
        let turns = generate_valid_moves_list(board);
        assert_eq!(3, turns.len());
    }

    #[test]
    fn move_ordering_with_pv_nodes_test() {
        let service = Service::new();
        let config = Config::for_tests();
        let local_map = DataMap::new();

        let board = &mut service.fen.set_init_board();

        let mut move_row = Vec::default();
        move_row.push(Turn::_new_to_from(m2l(81) as u8, m2l(61) as u8));
        move_row.push(Turn::_new_to_from(m2l(38) as u8, m2l(58) as u8));

        let mut pv_nodes_map = HashMap::new();
        let old_board = board.clone();
        for turn in &move_row {
            let hash = zobrist::gen(board);
            pv_nodes_map.insert(hash, *turn);
            board.do_move(turn);
        }
        *board = old_board;
        let pv_nodes = Mutex::new(pv_nodes_map);

        let zobrist_table = ZobristTable::new();
        let stop_flag = std::sync::atomic::AtomicBool::new(false);
        let history_table = [[0u32; 64]; 64];
        let context = SearchContext {
            zobrist_table: &zobrist_table,
            stop_flag: &stop_flag,
            pv_nodes: &pv_nodes,
            killer_moves: [None; 2],
            history_table: &history_table,
        };

        let turns = service.move_gen.generate_valid_moves_list(board, &mut Stats::new(), &config, &context, &local_map);
        let first_turn = turns.get(0).unwrap();

        assert_eq!(m2l(81) as u8, first_turn.from);
        assert_eq!(m2l(61) as u8, first_turn.to);
    }

    #[test]
    fn skip_validation_and_check_game_end_test() {
        let service = Service::new();
        let config = Config::for_tests();
        let mut local_map = DataMap::new();

        let zobrist_table = ZobristTable::new();
        let stop_flag = std::sync::atomic::AtomicBool::new(false);
        let pv_nodes = std::sync::Mutex::new(std::collections::HashMap::new());
        let history_table = [[0u32; 64]; 64];
        let context = SearchContext {
            zobrist_table: &zobrist_table,
            stop_flag: &stop_flag,
            pv_nodes: &pv_nodes,
            killer_moves: [None; 2],
            history_table: &history_table,
        };

        let board = &mut service.fen.set_fen("r1bqk1nr/ppp2ppp/2P5/4p3/2B5/3P1N2/PPP2PPP/RNBQb2R w kq - 0 1");

        local_map.insert(DataMapKey::ForceSkipValidationFlag, true);
        let turns = service.move_gen.generate_valid_moves_list(board, &mut Stats::new(), &config, &context, &mut local_map);
        assert_eq!(38, turns.len());

        let board = &mut service.fen.set_fen("r1bqk1nr/ppp2ppp/2P5/4p3/1bB5/3P1N2/PPP2PPP/RNBQK2R b KQkq - 0 1");
        let turn = Turn::new(m2l(62) as u8, m2l(95) as u8, 15, 0, false, 0);
        let mi = board.do_move(&turn);
        assert_eq!(false, board._white_king_on_board);
        assert_eq!(true, board._black_king_on_board);
        assert_eq!(GameStatus::BlackWin, board.game_status);
        let turns = service.move_gen.generate_valid_moves_list(board, &mut Stats::new(), &config, &context, &mut local_map);
        assert_eq!(0, turns.len());
        board.undo_move(&turn, mi);
        assert_eq!(true, board._white_king_on_board);
        assert_eq!(true, board._black_king_on_board);
        assert_eq!(GameStatus::Normal, board.game_status);

        let board = &mut service.fen.set_fen("r2qk1nr/pPp2ppp/8/4p3/Qbb5/2PP1N2/PP3PPP/RNB1K2R w KQkq - 0 1");
        let turn = Turn::new(m2l(61) as u8, m2l(25) as u8, 25, 0, false, 0);
        let mi = board.do_move(&turn);
        assert_eq!(true, board._white_king_on_board);
        assert_eq!(false, board._black_king_on_board);
        assert_eq!(GameStatus::WhiteWin, board.game_status);
        assert_eq!(0, turns.len());
        board.undo_move(&turn, mi);
        assert_eq!(true, board._white_king_on_board);
        assert_eq!(true, board._black_king_on_board);
        assert_eq!(GameStatus::Normal, board.game_status);
    }

    #[test]
    fn is_in_check_test() {
        let fen = Service::new().fen;
        let movegen = Service::new().move_gen;

        let board = fen.set_fen("r1b1kbnr/1pp1qppp/p1n5/3Pp3/B7/5N2/PPPP1PPP/RNBQK2R w KQkq - 1 6");
        assert!(!movegen.is_in_check(&board));

        let board = fen.set_fen("r1b1kbnr/1pp2ppp/p1n5/3Pq3/B7/8/PPPP1PPP/RNBQK2R w KQkq - 0 7");
        assert!(movegen.is_in_check(&board));

        let board = fen.set_fen("r1b1k1nr/1pp2ppp/p1B5/3Pq3/1b6/8/PPPPQPPP/RNB1K2R b KQkq - 0 8");
        assert!(movegen.is_in_check(&board));

        let board = fen.set_fen("r1bk2nr/1pp3pp/2B1Np2/p2Pq3/8/b7/PPPPQPPP/R1B1K2R b KQ - 1 12");
        assert!(movegen.is_in_check(&board));

        let board = fen.set_fen("r1b4r/1pp1k1pp/2B1Np2/p2Pq3/8/b4nP1/PPPPQP2/R1B2RK1 w - - 1 17");
        assert!(movegen.is_in_check(&board));

        let board = fen.set_fen("r1b4r/1pp1k1pp/2B1Np2/p2P2q1/8/b4QP1/PPPP1P2/R1B2RK1 w - - 1 18");
        assert!(!movegen.is_in_check(&board));

        let board = fen.set_fen("r1b4r/1pp1k1pp/2B1Np2/p2P2q1/4Q3/b5P1/PPPP1P2/R1B2RK1 b - - 2 18");
        assert!(!movegen.is_in_check(&board));
    }

    fn test_fen(fen: &str, allowed_moves: usize) -> Board {
        let fen_service = Service::new().fen;
        let mut board = fen_service.set_fen(fen);
        let moves = generate_valid_moves_list(&mut board);
        assert_eq!(moves.len(), allowed_moves, "Expected {} moves, but got {} for FEN: {}", allowed_moves, moves.len(), fen);
        board
    }

    fn test_fen_with_move(fen: &str, allowed_moves: usize, notation: &str) -> Vec<Turn> {
        let mut board = test_fen(fen, allowed_moves);
        let board_copy = board.clone();
        let move_list = generate_valid_moves_list(&mut board);
        let move_turn = NotationUtil::_get_turn_from_list(&move_list, notation);
        let move_info = board.do_move(&move_turn);
        let opponent_moves = generate_valid_moves_list(&mut board);
        board.undo_move(&move_turn, move_info);
        assert_eq!(&board, &board_copy, "Board should be restored after undoing the move");
        opponent_moves
    }
}
