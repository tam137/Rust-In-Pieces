
use std::sync::RwLockReadGuard;
use rand::Rng;

use crate::{global_map_handler, zobrist};
use crate::config::Config;
use crate::model::{Board, DataMapKey, GameStatus, Stats, ThreadSafeDataMap, Turn};
use crate::zobrist::ZobristTable;
use crate::DataMap;

const KNIGHT_OFFSETS: [i32; 8] = [-12, -21, -8, -19, 12, 21, 8, 19];
const BISHOP_OFFSETS: [i32; 4] = [-11, -9, 9, 11];
const ROOK_OFFSETS: [i32; 4] = [-10, -1, 1, 10];

pub struct MoveGenService {
    
}

impl MoveGenService {

    pub fn new() -> Self {
        MoveGenService {}
    }


    /// Generates a list of valid capture moves for a given board state.
    pub fn generate_valid_moves_list_capture(&self, board: &mut Board, stats: &mut Stats, config: &Config,
        global_map: &ThreadSafeDataMap, local_map: &DataMap) -> Vec<Turn> {

        if board.game_status != GameStatus::Normal {
            return vec![]
        }
        let move_list = self.generate_moves_list_for_piece(board, 0);
        let capture_moves: Vec<Turn> = self.get_valid_moves_from_move_list(&move_list, board, stats,
            config, true, global_map, local_map);

        stats.add_created_capture_nodes(capture_moves.len());
        capture_moves
    }

    /// Generates a list of valid moves for a given board state.
    pub fn generate_valid_moves_list(&self, board: &mut Board, stats: &mut Stats, config: &Config,
        global_map: &ThreadSafeDataMap, local_map: &DataMap)-> Vec<Turn> {
        if board.game_status != GameStatus::Normal {
            return vec![]
        }
        let move_list = self.generate_moves_list_for_piece(board, 0);
        self.get_valid_moves_from_move_list(&move_list, board, stats, config, false, global_map, local_map)
    }

    fn get_valid_moves_from_move_list(&self, move_list: &[i32], board: &mut Board, stats: &mut Stats, config: &Config,
        only_captures: bool, global_map: &ThreadSafeDataMap, local_map: &DataMap) -> Vec<Turn> {

        let mut valid_moves = Vec::with_capacity(64);
        let white_turn = board.white_to_move;
        let king_value = if white_turn { 15 } else { 25 };

        // get pv node
        let mut pv_node = None;
        if !only_captures && config.use_pv_nodes {
            if board.cached_hash == 0 {
                board.cached_hash = zobrist::gen(board);
            }
            if let Some(pv_node_result) = global_map_handler::get_pv_node_for_hash(global_map, board.cached_hash) {
                pv_node = Some(pv_node_result); // TODO assign direct
                //let logger = global_map_handler::get_log_buffer_sender(global_map);
                //logger.send(format!("Found Pv Node {:?}", pv_node)).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
            }
            
        }
        
        let zobrist_table_read = global_map_handler::get_zobrist_table(&global_map);
    
        for i in (0..move_list.len()).step_by(2) {
            let idx0 = move_list[i];
            let idx1 = move_list[i + 1];

            if board.field[idx1 as usize] == 0 && only_captures {
                continue;
            }

            let mut move_turn = Turn::new(idx0, idx1, board.field[idx1 as usize], 0, false, 0);

            if let Some(pv) = &pv_node {
                if *pv == move_turn {
                    move_turn.rank = config.is_pv_node_rank_bonus;
                }
            }

            // Check for castling
            if !only_captures && (board.field[idx0 as usize] == king_value && (idx1 == idx0 + 2 || idx1 == idx0 - 2)) {
                if !self.is_valid_castling(board, white_turn, idx1) {
                    continue;
                }
            }

            move_turn.rank += match move_turn.capture {
                10 | 20 => 2,
                11 | 21 => 5,
                12 | 22 => 3,
                13 | 23 => 3,
                14 | 24 => 9,
                _ => 0,
            };

            if move_turn.capture != 0 {
                move_turn.rank +=  match board.field[move_turn.from as usize] {
                    11 | 21 => -1,
                    14 | 24 => -3,
                    _ => 0,
                }
            };

            if move_turn.rank < 0 {
                move_turn.rank = 0;
            }
    
            // Check for promotion
            if let Some(promotion_move) = self.get_promotion_move(board, white_turn, idx0, idx1) {
                move_turn.promotion = promotion_move.promotion;
                // Validate and add the promotion moves (e.g., Queen, Knight)
                self.validate_and_add_promotion_moves(board, stats, &mut move_turn, config, &mut valid_moves, white_turn,
                    &zobrist_table_read, local_map);
            } else {
                // Validate and add the regular move
                // only if we are not in quiescence search
                self.validate_and_add_move(board, stats, &mut move_turn, config, &mut valid_moves, &zobrist_table_read, local_map);
            }
        }
    
        // Add en passant moves
        if !only_captures {
            let en_passante_turns = self.get_en_passante_turns(board, white_turn);
            for mut turn in en_passante_turns {
                self.validate_and_add_move(board, stats, &mut turn, config, &mut valid_moves, &zobrist_table_read, local_map);
            }
        }        
    
        // Move sorting for PV threads (default) or normal threads if PV config is off
        if *local_map.get_data::<bool>(DataMapKey::MoveOrderingFlag).unwrap_or(&true) {
            valid_moves.sort_unstable_by(|a, b| b.rank.cmp(&a.rank));
        }
        else { // SMP threads (only if PV config is on (default))
            let mut rng = rand::thread_rng();
            let mut noisy_values: Vec<(i32, i32)> = valid_moves
                .iter()
                .map(|mv| (mv.rank as i32, rng.gen_range(-config.smp_thread_eval_noise..=config.smp_thread_eval_noise) as i32))
                .collect();
            
            noisy_values.sort_unstable_by(|a, b| (b.0 + b.1).cmp(&(a.0 + a.1)));
        }        
    
        // check Gamestatus
        if valid_moves.is_empty() && !only_captures {
            if self.get_check_idx_list(&board, board.white_to_move).len() > 0 {
                board.game_status = if board.white_to_move { GameStatus::BlackWin } else { GameStatus::WhiteWin }
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
            let offsets = if white_turn { [9, 11] } else { [-9, -11] };
            for &offset in &offsets {
                if board.field[(board.field_for_en_passante + offset) as usize] == if white_turn { 10 } else { 20 } {
                    en_passante_turns.push(
                        Turn::new(board.field_for_en_passante + offset,
                            board.field_for_en_passante,
                            target_piece,
                            0,
                            false,
                            0)
                    );
                }
            }
        }
        en_passante_turns
    }
    
    fn validate_and_add_move(&self, board: &mut Board, stats: &mut Stats, turn: &mut Turn, config: &Config,
        valid_moves: &mut Vec<Turn>, zobrist_table_read: &ZobristTable, local_map: &DataMap) {

        let move_info = board.do_move(turn); // TODO store hash only in turn and calculate it once
        let mut valid = true;

        // can skip validation here
        if !*local_map.get_data::<bool>(DataMapKey::ForceSkipValidationFlag).unwrap_or(&false) {
            if self.gives_check(&board) {
                valid = false;
            }
        }

        if valid {
            turn.hash = board.cached_hash;
            if let Some(eval) = self.get_hash(board, config, zobrist_table_read) {
                turn.eval = eval;
                turn.has_hashed_eval = true;
                turn.rank = turn.rank + config.is_hashed_rank_bonus;
                stats.add_zobrist_hit(1);
            }

            if self.is_in_check(&board) {
                turn.gives_check = true;
                turn.rank = turn.rank + config.give_check_rank_bonus;
            }
            valid_moves.push(turn.clone());
        }
        board.undo_move(turn, move_info);
    }
    
    fn validate_and_add_promotion_moves(&self, board: &mut Board, stats: &mut Stats, turn: &mut Turn, config: &Config,
        valid_moves: &mut Vec<Turn>, white_turn: bool, zobrist_table_read: &ZobristTable, local_map: &DataMap) {

        let promotion_types = if white_turn { [12, 14] } else { [22, 24] }; // Knight and Queen promotions for white and black
        for &promotion in &promotion_types {
            turn.promotion = promotion;
            match promotion {
                12 | 22 => turn.rank += config.give_promotion_rank_bonus_queen,
                14 | 24 => turn.rank += config.give_promotion_rank_bonus_knight,
                _ => panic!("Promotion value not expected"),
            }
            
            self.validate_and_add_move(board, stats, turn, config, valid_moves, zobrist_table_read, local_map);
        }
    }    

    fn is_valid_castling(&self, board: &Board, white_turn: bool, target: i32) -> bool {
        let check_squares = if white_turn {
            if target == 97 { vec![96, 97] } else { vec![94, 93] }
        } else {
            if target == 27 { vec![26, 27] } else { vec![24, 23] }
        };

        // Check if the king is currently in check
        if !self.get_check_idx_list(&board, white_turn).is_empty() {
            return false;
        }

        // Check if the king would pass through check squares
        for &square in &check_squares {
            if !self.get_attack_idx_list(&board.field, white_turn, square).is_empty() {
                return false;
            }
        }

        // Check if castling is allowed
        if white_turn {
            if target == 97 && !board.white_possible_to_castle_short {
                return false;
            }
            if target == 93 && !board.white_possible_to_castle_long {
                return false;
            }
        } else {
            if target == 27 && !board.black_possible_to_castle_short {
                return false;
            }
            if target == 23 && !board.black_possible_to_castle_long {
                return false;
            }
        }
        true
    }

    fn get_promotion_move(&self, board: &Board, white_turn: bool, idx0: i32, idx1: i32) -> Option<Turn> {
        if white_turn && idx0 / 10 == 3 && board.field[idx0 as usize] == 10 {
            Some(Turn {
                from: idx0,
                to: idx1,
                capture: 0,
                promotion: 14,
                gives_check: false,
                eval: 0,
                hash: 0,
                has_hashed_eval: false,
                rank: 0,
            })
        } else if !white_turn && idx0 / 10 == 8 && board.field[idx0 as usize] == 20 {
            Some(Turn {
                from: idx0,
                to: idx1,
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
            return zobrist_table_read.get_eval_for_hash(&board.cached_hash);
        }

    fn _check_if_hash_exists(&self, board: &mut Board, config: &Config, zobrist_table_read: &RwLockReadGuard<'_, ZobristTable>)
    -> bool {
        if config.use_zobrist {             
            match zobrist_table_read.get_eval_for_hash(&board.cached_hash) {
                Some(_) => {
                    true
                },
                None => {
                    false
                }
            }
        } else {
            false
        }
    }


    pub fn generate_moves_list_for_piece(&self, board: &Board, idx: i32) -> Vec<i32> {
        let check_idx_list = self.get_check_idx_list(&board, board.white_to_move);
        let field = board.field;

        let white = if idx == 0 {
            board.white_to_move
        } else {
            if board.field[idx as usize] <= 0 { panic!("RIP no piece in idx {}", idx) };
            if board.field[idx as usize] / 10 == 1 { true } else { false }
        };

        let king_value = if white { 15 } else { 25 };
        let queen_value = if white { 14 } else { 24 };
        let rook_value = if white { 11 } else { 21 };
        let bishop_value = if white { 13 } else { 23 };
        let knight_value = if white { 12 } else { 22 };
        let pawn_value = if white { 10 } else { 20 };

        let mut moves = Vec::with_capacity(64);

        let start_idx = if idx == 0 { 21 } else { idx };
        let end_idx = if idx == 0 { 99 } else { idx + 1 };

        for i in start_idx..end_idx {
            // Skip other pieces if the king is in check from multiple pieces
            if check_idx_list.len() > 1 && field[i as usize] != king_value {
                continue;
            }

            // Skip empty squares or enemy pieces
            if field[i as usize] <= 0 {
                continue;
            }
            if (field[i as usize] >= 10 && field[i as usize] <= 15 && !white)
                || (field[i as usize] >= 20 && field[i as usize] <= 25 && white) {
                continue;
            }

            // King moves
            if field[i as usize] == king_value {
                let offsets = [-11, -10, -9, -1, 1, 9, 10, 11];
                for offset in offsets {
                    let target = i + offset;
                    if (field[target as usize] == 0 || field[target as usize] / 10 == if white { 2 } else { 1 })
                        && field[target as usize] != -11
                    {
                        let mut valid = true;
                        for target_offset in offsets {
                            if field[(target + target_offset) as usize] == if white { 25 } else { 15 } {
                                valid = false;
                                break;
                            }
                        }
                        if valid {
                            moves.push(i);
                            moves.push(target);
                        }
                    }
                }

                // Castling moves for White and Black
                if field[i as usize] == king_value {
                    if i == 95 && field[96] == 0 && field[97] == 0 && field[98] == 11 {
                        moves.push(95);
                        moves.push(97);
                    }
                    if i == 25 && field[26] == 0 && field[27] == 0 && field[28] == 21 {
                        moves.push(25);
                        moves.push(27);
                    }
                    if i == 95 && field[94] == 0 && field[93] == 0 && field[92] == 0 && field[91] == 11 {
                        moves.push(95);
                        moves.push(93);
                    }
                    if i == 25 && field[24] == 0 && field[23] == 0 && field[22] == 0 && field[21] == 21 {
                        moves.push(25);
                        moves.push(23);
                    }
                }
            }

            // Pawn moves
            if field[i as usize] == pawn_value {
                if white {
                    if field[(i - 10) as usize] == 0 {
                        moves.push(i);
                        moves.push(i - 10);
                        if i >= 81 && i <= 88 && field[(i - 20) as usize] == 0 {
                            moves.push(i);
                            moves.push(i - 20);
                        }
                    }
                    if field[(i - 9) as usize] >= 20 {
                        moves.push(i);
                        moves.push(i - 9);
                    }
                    if field[(i - 11) as usize] >= 20 {
                        moves.push(i);
                        moves.push(i - 11);
                    }
                } else {
                    if field[(i + 10) as usize] == 0 {
                        moves.push(i);
                        moves.push(i + 10);
                        if i >= 31 && i <= 38 && field[(i + 20) as usize] == 0 {
                            moves.push(i);
                            moves.push(i + 20);
                        }
                    }
                    if field[(i + 9) as usize] < 20 && field[(i + 9) as usize] > 0 {
                        moves.push(i);
                        moves.push(i + 9);
                    }
                    if field[(i + 11) as usize] < 20 && field[(i + 11) as usize] > 0 {
                        moves.push(i);
                        moves.push(i + 11);
                    }
                }
            }

            // Knight moves
            if field[i as usize] == knight_value {
                let offsets = [-21, -19, -12, -8, 8, 12, 19, 21];
                for offset in offsets {
                    let target = i + offset;
                    if field[target as usize] == 0
                        || (field[target as usize] / 10 == if white { 2 } else { 1 } && field[target as usize] != -11)
                    {
                        moves.push(i);
                        moves.push(target);
                    }
                }
            }

            // Bishop moves
            if field[i as usize] == bishop_value {
                let offsets = [-11, -9, 9, 11];
                for offset in offsets {
                    let mut target = i + offset;
                    while field[target as usize] == 0 || field[target as usize] / 10 == if white { 2 } else { 1 } {
                        moves.push(i);
                        moves.push(target);
                        if field[target as usize] != 0 {
                            break;
                        }
                        target += offset;
                    }
                }
            }

            // Queen moves
            if field[i as usize] == queen_value {
                let offsets = [-11, -10, -9, -1, 1, 9, 10, 11];
                for offset in offsets {
                    let mut target = i + offset;
                    while (field[target as usize] == 0 || field[target as usize] / 10 == if white { 2 } else { 1 })
                        && field[target as usize] != -11
                    {
                        moves.push(i);
                        moves.push(target);
                        if field[target as usize] != 0 {
                            break;
                        }
                        target += offset;
                    }
                }
            }

            // Rook moves
            if field[i as usize] == rook_value {
                let offsets = [-10, 10, -1, 1];
                for offset in offsets {
                    let mut target = i + offset;
                    while (field[target as usize] == 0 || field[target as usize] / 10 == if white { 2 } else { 1 })
                        && field[target as usize] != -11
                    {
                        moves.push(i);
                        moves.push(target);
                        if field[target as usize] != 0 {
                            break;
                        }
                        target += offset;
                    }
                }
            }
        }
        moves
    }


    // checks wich OPPONENT pieces attack the given target_idx
    pub fn get_attack_idx_list(&self, field: &[i32], white: bool, target_idx: i32) -> Vec<i32> {

        // king is missing
        if target_idx == -1 {
            return Vec::new(); // TODO use Some or save on stack
        }

        let mut check_idx_list = Vec::new();

        // Opponent's piece values
        let opponent_pawn = if white { 20 } else { 10 };
        let opponent_rook = if white { 21 } else { 11 };
        let opponent_knight = if white { 22 } else { 12 };
        let opponent_bishop = if white { 23 } else { 13 };
        let opponent_queen = if white { 24 } else { 14 };

        // Pawns attacking
        if white {
            let idx1 = (target_idx - 9) as usize;
            let idx2 = (target_idx - 11) as usize;
            if field[idx1] == opponent_pawn {
                check_idx_list.push(target_idx - 9);
            }
            if field[idx2] == opponent_pawn {
                check_idx_list.push(target_idx - 11);
            }
        } else {
            let idx1 = (target_idx + 9) as usize;
            let idx2 = (target_idx + 11) as usize;
            if field[idx1] == opponent_pawn {
                check_idx_list.push(target_idx + 9);
            }
            if field[idx2] == opponent_pawn {
                check_idx_list.push(target_idx + 11);
            }
        }

        // Knights attacking
        for &offset in &KNIGHT_OFFSETS {
            let idx = (target_idx + offset) as usize;
            if field[idx] == opponent_knight {
                check_idx_list.push(target_idx + offset);
            }
        }

        // Bishops and Queen attacking (Diagonals)
        for &offset in &BISHOP_OFFSETS {
            let mut pos = target_idx + offset;
            loop {
                let piece = field[pos as usize];
                if piece == 0 {
                    pos += offset;
                    continue;
                } else if piece == opponent_bishop || piece == opponent_queen {
                    check_idx_list.push(pos);
                    break;
                } else {
                    break;
                }
            }
        }

        // Rooks and Queen attacking (Horizontals and Verticals)
        for &offset in &ROOK_OFFSETS {
            let mut pos = target_idx + offset;
            loop {
                let piece = field[pos as usize];
                if piece == 0 {
                    pos += offset;
                    continue;
                } else if piece == opponent_rook || piece == opponent_queen {
                    check_idx_list.push(pos);
                    break;
                } else {
                    break;
                }
            }
        }

        check_idx_list
    }


    /// Checks if the [white] target index is under attack.
    /// Return a list of Fields, that attacks the [white] target index
    /// // includes also fields that attack from behind of a attacker
    pub fn get_attack_idx_list_with_shadow(&self, field: &[i32], white: bool, target_idx: i32) -> Vec<i32> {
        let mut my_field: [i32; 120] = field.try_into().expect("RIP Invalid field size");
        let mut all_attackers = Vec::default();
    
        loop {
            let attackers = self.get_attack_idx_list(&my_field, white, target_idx);
            if attackers.len() == 0 { break; }
            for attacker in attackers {
                my_field[attacker as usize] = 0;
                all_attackers.push(attacker);
            }
        }
        all_attackers
    }

    /// Checks if the king is under attack.
    /// Return a list of Fields, that give [white] check
    pub fn get_check_idx_list(&self, board: &Board, white: bool) -> Vec<i32> {
        // TODO store king pos on board
        let king_pos = if white { board.get_king_positions().0 } else { board.get_king_positions().1 };
        self.get_attack_idx_list(&board.field, white, king_pos)
    }

    /// returns true if the side to move gives check, otherwise false
    pub fn gives_check(&self, board: &Board) -> bool  {
        self.__check_check(board, true)
    }

    /// returns true if the king of side to move is in check, otherwise false
    pub fn is_in_check(&self, board: &Board) -> bool {
        self.__check_check(board, false)
    }

    /// returns true if the king of side to move is in check, otherwise false
    /// inverse flag inverses the side to gives check (use function gives_check() instead)
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

        // Opponent's piece values
        let opponent_pawn = if white { 20 } else { 10 };
        let opponent_rook = if white { 21 } else { 11 };
        let opponent_knight = if white { 22 } else { 12 };
        let opponent_bishop = if white { 23 } else { 13 };
        let opponent_queen = if white { 24 } else { 14 };

        // Pawns attacking
        if white {
            let idx1 = (target_idx - 9) as usize;
            let idx2 = (target_idx - 11) as usize;
            if board.field[idx1] == opponent_pawn {
                return true;
            }
            if board.field[idx2] == opponent_pawn {
                return true;
            }
        } else {
            let idx1 = (target_idx + 9) as usize;
            let idx2 = (target_idx + 11) as usize;
            if board.field[idx1] == opponent_pawn {
                return true;
            }
            if board.field[idx2] == opponent_pawn {
                return true;
            }
        }

        // Knights attacking
        for &offset in &KNIGHT_OFFSETS {
            let idx = target_idx + offset;
            if board.field[idx as usize] == opponent_knight {
                return true;
            }
        }

        // Bishops and Queen attacking (Diagonals)
        for &offset in &BISHOP_OFFSETS {
            let mut pos = target_idx + offset;
            loop {
                let piece = board.field[pos as usize];
                if piece == 0 {
                    pos += offset;
                    continue;
                } else if piece == opponent_bishop || piece == opponent_queen {
                    return true;
                } else {
                    break;
                }
            }
        }

        // Rooks and Queen attacking (Horizontals and Verticals)
        for &offset in &ROOK_OFFSETS {
            let mut pos = target_idx + offset;
            loop {
                let piece = board.field[pos as usize];
                if piece == 0 {
                    pos += offset;
                    continue;
                } else if piece == opponent_rook || piece == opponent_queen {
                    return true;
                } else {
                    break;
                }
            }
        }
        false
    }

}


#[cfg(test)]
mod tests {
    use crate::notation_util::NotationUtil;
    use crate::Service;
    use super::*;

    fn generate_valid_moves_list(board: &mut Board) -> Vec<Turn> {
        let service = Service::new();
        let global_map = global_map_handler::create_new_global_map();
        let local_map = DataMap::new();
        let config = Config::for_tests();

        service.move_gen.generate_valid_moves_list(board, &mut Stats::new(), &config, &global_map, &local_map)
    }

    fn generate_valid_moves_list_capture(board: &mut Board) -> Vec<Turn> {
        let service = Service::new();
        let global_map = global_map_handler::create_new_global_map();
        let local_map = DataMap::new();
        let config = Config::for_tests();

        service.move_gen.generate_valid_moves_list_capture(board, &mut Stats::new(), &config, &global_map, &local_map)
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
        assert!(check_idx_list.contains(&86), "Check index list should contain 86");
        assert!(check_idx_list.contains(&76), "Check index list should contain 76");
        assert!(check_idx_list.contains(&62), "Check index list should contain 62");

        // Test 3: Black turn - No check
        board = fen_service.set_fen("r1bqk1nr/pppp2pp/4p3/8/1b3P2/3PPn2/PPP2pPP/RNBQKBNR b KQkq - 0 1");
        assert!(move_gen_service.get_check_idx_list(&board, board.white_to_move).is_empty());

        // Test 4: Two checks, positions 36 and 37
        board = fen_service.set_fen("r1bqk1nr/pppp1PNp/4p3/1Q5B/1b3P2/3PPn2/PPP2p1P/RN2KB1R b KQkq - 0 1");
        let check_idx_list = move_gen_service.get_check_idx_list(&board, board.white_to_move);
        assert_eq!(check_idx_list.len(), 2);
        assert!(check_idx_list.contains(&36), "Check index list should contain 36");
        assert!(check_idx_list.contains(&37), "Check index list should contain 37");

        // Test 5: Four checks in various positions
        board = fen_service.set_fen("r1Rqk2r/pppP2Np/5n2/3p3B/1b2QP2/3PPn2/PPP2p1P/RN2KB2 b KQkq - 0 1");
        let check_idx_list = move_gen_service.get_check_idx_list(&board, board.white_to_move);
        assert_eq!(check_idx_list.len(), 4);
        assert!(check_idx_list.contains(&37), "Check index list should contain 37");
        assert!(check_idx_list.contains(&58), "Check index list should contain 58");
        assert!(check_idx_list.contains(&65), "Check index list should contain 65");
        assert!(check_idx_list.contains(&34), "Check index list should contain 34");

        // Test 6: Four checks in another scenario
        board = fen_service.set_fen("2B5/6N1/4k3/8/2K2NP1/1B2Q2B/4R3/8 b - - 0 1");
        let check_idx_list = move_gen_service.get_check_idx_list(&board, board.white_to_move);
        assert!(check_idx_list.contains(&23), "Check index list should contain 23");
        assert!(check_idx_list.contains(&37), "Check index list should contain 37");
        assert!(check_idx_list.contains(&66), "Check index list should contain 66");
        assert!(check_idx_list.contains(&75), "Check index list should contain 75");
        assert_eq!(check_idx_list.len(), 4);

        // Test 7: Last check scenario with two checks
        board = fen_service.set_fen("8/1k6/8/1q6/2b1r3/8/1rn1K3/8 w - - 0 1");
        let check_idx_list = move_gen_service.get_check_idx_list(&board, board.white_to_move);
        assert!(check_idx_list.contains(&63), "Check index list should contain 63");
        assert!(check_idx_list.contains(&65), "Check index list should contain 65");
        assert_eq!(check_idx_list.len(), 2);
    }

    #[test]
    fn get_attack_idx_list_test() {
        let fen_service = Service::new().fen;
        let move_gen_service = Service::new().move_gen;

        let board = fen_service.set_fen("r1q2r1k/1pp1bpp1/p2p1n2/4P2p/2Q2B2/2N4P/PPPR1PP1/3R2K1 b - - 3 16");
        let attacks =  move_gen_service.get_attack_idx_list(&board.field, board.white_to_move, 44).len() as i32;
        assert_eq!(2 , attacks);

        let board = fen_service.set_fen("r1bqnr2/pp1nbpk1/2p1p3/3p2pp/2PP1P1N/2NBP1B1/PPQ3PP/2R1K2R w K - 0 14");
        let attacks =  move_gen_service.get_attack_idx_list(&board.field, board.white_to_move, 68).len() as i32;
        assert_eq!(1 , attacks);

        let board = fen_service.set_fen("r2qkb1r/pppp1ppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1");
        let attacks =  move_gen_service.get_attack_idx_list(&board.field, board.white_to_move, 24).len() as i32;
        assert_eq!(0 , attacks);
    }

    #[test]
    fn get_attack_idx_list_with_shadow_test() {
        let fen_service = Service::new().fen;
        let move_gen_service = Service::new().move_gen;

        let board = fen_service.set_fen("r1q2r1k/1pp1bpp1/p2p1n2/4P2p/2Q2B2/2N4P/PPPR1PP1/3R2K1 b - - 3 16");
        let attacks =  move_gen_service.get_attack_idx_list_with_shadow(&board.field, board.white_to_move, 44).len() as i32;
        assert_eq!(4 , attacks);

        let board = fen_service.set_fen("r1bqnr2/pp1nbpk1/2p1p3/3p2pp/2PP1P1N/2NBP1B1/PPQ3PP/2R1K2R w K - 0 14");
        let attacks =  move_gen_service.get_attack_idx_list_with_shadow(&board.field, board.white_to_move, 68).len() as i32;
        assert_eq!(3 , attacks);
    }

    #[test]
    fn generate_moves_list_for_fen_test() {
        let fen_service = Service::new().fen;
        let move_gen_service = Service::new().move_gen;

        // Test: Standard starting position of a chess game
        let board = fen_service.set_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        let moves = move_gen_service.generate_moves_list_for_piece(&board, 0);

        // Expected moves for the initial position (pawns and knights)
        let expected_moves = vec![
            81, 71, 81, 61, 82, 72, 82, 62, 83, 73, 83, 63, 84, 74, 84, 64,
            85, 75, 85, 65, 86, 76, 86, 66, 87, 77, 87, 67, 88, 78, 88, 68,
            92, 71, 92, 73, 97, 76, 97, 78
        ];
        assert_eq!(moves, expected_moves, "Move Gen for start-up setup is wrong");

        // Test: White is in check and only a few moves are available for the king
        let board = fen_service.set_fen("rnbqk2r/pppp1ppp/4p3/8/1b6/3P1n1B/PPP1PPPP/RNBQK1NR w KQkq - 0 1");
        let moves = move_gen_service.generate_moves_list_for_piece(&board, 0);

        // Expected king moves in double check situation
        let expected_moves_in_check = vec![95, 84, 95, 96];
        assert_eq!(moves, expected_moves_in_check, "Check list is not working");
    }


    #[test]
    fn get_valid_moves_from_move_list_test_double_check() {
        let service = &Service::new();

        // Double check, only one king move is possible for white
        let mut board = service.fen.set_fen("rnbqk2r/pppp1ppp/4p3/8/1b6/3P1n1B/PPP1PPPP/RNBQK1NR w KQkq - 0 1");
        let valid_turn_list = generate_valid_moves_list(&mut board);
        assert_eq!(valid_turn_list.len(), 1);  // Check that only one valid move exists
        assert!(valid_turn_list[0].from == 95 && valid_turn_list[0].to == 96);  // King move from 95 to 96

        // Double check, only one king move is possible for black
        let mut board = service.fen.set_fen("rnb1k1nr/ppppp1Np/2N5/7Q/8/4P3/PPPP1PPP/RNB1KB1R b KQkq - 0 1");
        let valid_turn_list = generate_valid_moves_list(&mut board);
        assert_eq!(valid_turn_list.len(), 1);  // Check that only one valid move exists
        assert!(valid_turn_list[0].from == 25 && valid_turn_list[0].to == 26);  // King move from 25 to 26
    }

    #[test]
    fn get_valid_moves_when_in_check_easy() {
        // Test cases for white in check
        // Expected number of valid moves in these positions
        test_fen("rnbqk1nr/pppp1ppp/4p3/8/1b6/3P1P2/PPP1P1PP/RNBQKBNR w KQkq - 1 3", 6);
        test_fen("rnbqkb1r/pppppppp/8/8/8/5n2/PPPPQ1PP/RNB1KBNR w KQkq - 0 1", 5);
        test_fen("8/5k2/3r4/5n2/3N4/3K4/1q6/8 w - - 0 1", 2);
        test_fen("8/5k2/3r4/5n2/8/3K1N2/1q6/8 w - - 0 1", 3);

        // Test cases for black in check
        test_fen("rnbqkbnr/ppp1pppp/3p4/1B6/4P3/8/PPPP1PPP/RNBQK1NR b KQkq - 1 2", 5);
        test_fen("r1bqkbnr/pp1npppp/2Bp4/8/4P3/3P4/PPP2PPP/RNBQK1NR b KQkq - 0 4", 20);
        test_fen("rnbqk2r/pppp2pp/5p1n/2b1p2Q/2B1P3/P6N/1PPP1PPP/RNB1K2R b KQkq - 1 5", 4);
        test_fen("8/8/3k1N1p/3b4/3Q4/8/4R3/3K4 b - - 0 1", 3);
    }

    #[test]
    fn castling_test() {
        // Test castling moves and expected valid move counts
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
        // White promotion to queen
        test_fen_with_move("5n2/4P3/8/2k5/8/8/2K5/8 w - - 0 1", 12, "e7f8q");

        let fen_service = Service::new().fen;

        let mut board = fen_service.set_fen("5n2/4P3/8/2k5/8/8/2K5/8 w - - 0 1");
        let board_copy = board.clone();

        let mut promotion_move = NotationUtil::get_turn_from_notation("e7f8q");
        promotion_move.capture = 22;
        let move_info = board.do_move(&promotion_move);

        assert_eq!(board.field[26], 14, "White promotion should result in a queen (14)");
        board.undo_move(&promotion_move, move_info);
        assert_eq!(board.field[26], 22, "Piece should revert to captured knight (22)");
        assert_eq!(board.field[35], 10, "White pawn should be back at e7");
        assert_eq!(board, board_copy, "Board should be restored");

        // black Promotion to queen
        let mut board = fen_service.set_fen("8/2p2p1p/p4kp1/1r6/8/K7/3p4/8 b - - 1 51");
        let turns = generate_valid_moves_list(&mut board);
        let promotion_move = turns.iter().find(|t| t.is_promotion()).expect("No Promotion move found");
        assert_eq!(24, promotion_move.promotion);
        let mi = board.do_move(promotion_move);
        assert_eq!(24, board.field[94]);
        assert_eq!(0, board.field[84]);

        board.undo_move(promotion_move, mi);
        assert_eq!(0, board.field[94]);
        assert_eq!(20, board.field[84]);

        let move_from_notation_util = NotationUtil::get_turn_from_notation("d2d1q");
        board.do_move(&move_from_notation_util);
        //assert_eq!(move_from_notation_util, promotion_move);
        assert_eq!(24, board.field[94]);
        assert_eq!(0, board.field[84]);


        // Testing knight promotion
        let move_list = test_fen_with_move("8/2k1P3/8/7b/8/4b3/2K3n1/8 w - - 0 1", 7, "e7e8n");
        assert_eq!(move_list.len(), 8, "Expected 8 moves after knight promotion");

        let move_list = test_fen_with_move("8/2k1P3/8/7b/8/4b3/2K3n1/8 w - - 0 1", 7, "e7e8q");
        assert_eq!(move_list.len(), 24, "Expected 24 moves after queen promotion");

        let move_list = test_fen_with_move("5k2/R6P/8/8/8/2K5/8/6r1 w - - 0 1", 23, "h7h8q");
        assert_eq!(move_list.len(), 1, "Expected 1 move after queen promotion on h8");

        // Black promotion to queen
        let move_list = test_fen_with_move("8/8/8/8/8/1K6/5p2/4k3 b - - 0 1", 6, "f2f1q");
        assert_eq!(move_list.len(), 7, "Expected 7 moves after black queen promotion");

        // Black promotion to knight
        let move_list = test_fen_with_move("8/8/8/8/8/1K6/5p2/4k3 b - - 0 1", 6, "f2f1n");
        assert_eq!(move_list.len(), 8, "Expected 8 moves after black knight promotion");

        // Black promotion on h1
        let move_list = test_fen_with_move("8/8/3k4/8/8/8/1K5p/8 b - - 0 1", 10, "h2h1q");
        assert_eq!(move_list.len(), 5, "Expected 5 moves after black queen promotion on h1");
    }

    #[test]
    fn move_list_sort_test() {
        let fen_service = Service::new().fen;

        let mut board = fen_service.set_fen("r1bqk2r/pppp1ppp/2n2n2/2b5/2BpP3/2P2N2/PP3PPP/RNBQK2R w KQkq - 0 6");
        let move_list = generate_valid_moves_list(&mut board);
        assert!(move_list.get(0).unwrap().from == 63 && move_list.get(0).unwrap().to == 36);
        assert!(move_list.last().unwrap().rank == 0);

        let mut board = fen_service.set_fen("r1bqr1k1/ppp2pp1/2n2n1p/2bp4/2B1PB2/1NP4P/PP3PP1/RN1Q1RK1 b - - 1 10");
        let move_list = generate_valid_moves_list(&mut board);
        assert!(move_list.get(0).unwrap().from == 53 && move_list.get(0).unwrap().to == 86);
        assert!(move_list.get(1).unwrap().from == 54 && move_list.get(1).unwrap().to == 63);
        assert!(move_list.last().unwrap().rank == 0);
    }

    #[test]
    fn game_status_check_mate_test() {
        // Black mate
        let board = test_fen("rnbqkbnr/ppppp2p/8/7B/8/8/PPPPPPPP/RNBQK1NR b KQkq - 0 1", 0);
        assert!(board.game_status == GameStatus::WhiteWin);

        let board = test_fen("3N3B/8/6k1/4K3/8/6RR/8/8 b - - 0 1", 0);
        assert!(board.game_status == GameStatus::WhiteWin);

        // White Mate
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

        // TODO for black please
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

        // Test if fen works
        let board = fen_service.set_fen("rnbqkbnr/ppp1ppp1/8/3pP2p/8/7P/PPPP1PP1/RNBQKBNR w KQkq d6 0 4");
        assert_eq!(44, board.field_for_en_passante);

        let board = fen_service.set_fen("rnbqkbnr/ppp1pppp/8/8/3pP2P/6P1/PPPP1P2/RNBQKBNR b KQkq e3 0 3");
        assert_eq!(75, board.field_for_en_passante);

        let truncate = Config::new().truncate_bad_moves;

        // test if movegen finds en passante move
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
        let global_map = global_map_handler::create_new_global_map();
        let local_map = DataMap::new();

        let board = &mut service.fen.set_init_board();

        let mut move_row = Vec::default();
        move_row.push(Turn::_new_to_from(81, 61));
        move_row.push(Turn::_new_to_from(38, 58));
        global_map_handler::set_pv_nodes(&global_map, &move_row, board);

        let turns = service.move_gen.generate_valid_moves_list(board, &mut Stats::new(), &config, &global_map, &local_map);
        let first_turn = turns.get(0).unwrap();
        
        assert_eq!(81, first_turn.from);
        assert_eq!(61, first_turn.to);
    }

    #[test]
    fn skip_validation_and_check_game_end_test() {
        let service = Service::new();
        let config = Config::for_tests();
        let global_map = global_map_handler::create_new_global_map();
        let mut local_map = DataMap::new();

        // missing king
        let board = &mut service.fen.set_fen("r1bqk1nr/ppp2ppp/2P5/4p3/2B5/3P1N2/PPP2PPP/RNBQb2R w kq - 0 1");

        // find moves if fen has missing king
        local_map.insert(DataMapKey::ForceSkipValidationFlag, true);
        let turns = service.move_gen.generate_valid_moves_list(board, &mut Stats::new(), &config, &global_map, &mut local_map);
        assert_eq!(38, turns.len());

        // avoid finding moves if uses do_move() funktion
        let board = &mut service.fen.set_fen("r1bqk1nr/ppp2ppp/2P5/4p3/1bB5/3P1N2/PPP2PPP/RNBQK2R b KQkq - 0 1");
        let turn = Turn::new(62, 95, 15, 0, false, 0);
        let mi = board.do_move(&turn); // hit white king
        assert_eq!(false, board._white_king_on_board);
        assert_eq!(true, board._black_king_on_board);
        assert_eq!(GameStatus::BlackWin, board.game_status);
        let turns = service.move_gen.generate_valid_moves_list(board, &mut Stats::new(), &config, &global_map, &mut local_map);
        assert_eq!(0, turns.len());
        board.undo_move(&turn, mi);
        assert_eq!(true, board._white_king_on_board);
        assert_eq!(true, board._black_king_on_board);
        assert_eq!(GameStatus::Normal, board.game_status);

        let board = &mut service.fen.set_fen("r2qk1nr/pPp2ppp/8/4p3/Qbb5/2PP1N2/PP3PPP/RNB1K2R w KQkq - 0 1");
        let turn = Turn::new(61, 25, 25, 0, false, 0);
        let mi = board.do_move(&turn); // hit black king
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
        let fen= Service::new().fen;
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


    fn _check_turn_order_of(notation: &str, turns: Vec<Turn>, expected_below: i32) -> () {
        let mut counter = 0;
        let target_turn = NotationUtil::get_turn_from_notation(notation);
        for t in &turns {
            counter += 1;
            if t.from == target_turn.from {
                if counter <= expected_below {
                    return;
                } else {
                    panic!("RIP Counter = {} allowed expected {}", counter, expected_below);
                }
            }
        }
        panic!("RIP Did not found target move");
    }

    fn _from(fen: &str) -> Vec<Turn> {
        let mut board = Service::new().fen.set_fen(fen);
        generate_valid_moves_list(&mut board)
    }

    // Function to test FEN position and check if the allowed moves match the expected count
    fn test_fen(fen: &str, allowed_moves: usize) -> Board {
        let fen_service = Service::new().fen;

        let mut board = fen_service.set_fen(fen);
        let moves = generate_valid_moves_list(&mut board);
        assert_eq!(moves.len(), allowed_moves, "Expected {} moves, but got {} for FEN: {}", allowed_moves, moves.len(), fen);
        board
    }

    /**
     * Function to test a FEN position, check the number of valid moves, and make a specific move.
     * After the move, it checks the possible moves for the opponent.
     *
     * @param fen the board's FEN string
     * @param allowed_moves the number of allowed moves for the given FEN position
     * @param notation the move expected to be allowed for the given FEN position
     * @return a vector of moves that are possible after the notation move for the opponent
     */
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
