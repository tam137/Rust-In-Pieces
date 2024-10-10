use crate::model::{Board, Turn, MoveInformation, Pair};
use std::collections::HashMap;

pub struct MoveGenService;

impl MoveGenService {
    /// Generates a list of valid moves for a given board state.
    pub fn generate_valid_moves_list(board: &mut Board) -> Vec<Turn> {
        let move_list = Self::generate_moves_list_for_piece(board, 0);
        Self::get_valid_moves_from_move_list(&move_list, board)
    }

    fn get_valid_moves_from_move_list(move_list: &[i32], board: &mut Board) -> Vec<Turn> {
        let mut valid_moves = Vec::new();
        let white_turn = board.is_white_to_move();
        let king_value = if white_turn { 15 } else { 25 };

        for i in (0..move_list.len()).step_by(2) {
            let idx0 = move_list[i];
            let idx1 = move_list[i + 1];
            let mut move_turn = Turn::new(idx0, idx1, board.get_field()[idx1 as usize], 0, 0);

            // Check for castling
            if board.get_field()[idx0 as usize] == king_value && (idx1 == idx0 + 2 || idx1 == idx0 - 2) {
                if !Self::is_valid_castling(board, white_turn, idx1) {
                    continue;
                }
            }

            // Check for promotion
            if let Some(promotion_move) = Self::get_promotion_move(board, white_turn, idx0, idx1) {
                move_turn.set_promotion(promotion_move.get_promotion());
            }

            // Perform the move
            let move_info = board.do_move(&move_turn);
            let mut valid = true;

            // Check if the move leads to check
            if !Self::get_check_idx_list(board.get_field(), white_turn).is_empty() {
                valid = false;
            }

            // If valid, add the move to the list
            if valid {
                valid_moves.push(move_turn.clone());
                if let Some(promotion_move) = Self::get_promotion_move(board, white_turn, idx0, idx1) {
                    valid_moves.push(Turn::new_from_existing(&move_turn).set_promotion(promotion_move.get_promotion() - 2)); // Knight promotion
                }
            }

            board.undo_move(&move_turn, move_info);
        }
        valid_moves
    }

    fn is_valid_castling(board: &Board, white_turn: bool, target: i32) -> bool {
        let check_squares = if white_turn {
            if target == 97 { vec![96, 97] } else { vec![94, 93] }
        } else {
            if target == 27 { vec![26, 27] } else { vec![24, 23] }
        };

        // Check if the king is currently in check
        if !Self::get_check_idx_list(board.get_field(), white_turn).is_empty() {
            return false;
        }

        // Check if the king would pass through check squares
        for &square in &check_squares {
            if !Self::get_attack_idx_list(board.get_field(), white_turn, square).is_empty() {
                return false;
            }
        }

        // Check if castling is allowed
        if white_turn {
            if target == 97 && !board.is_white_possible_to_castle_short() {
                return false;
            }
            if target == 93 && !board.is_white_possible_to_castle_long() {
                return false;
            }
        } else {
            if target == 27 && !board.is_black_possible_to_castle_short() {
                return false;
            }
            if target == 23 && !board.is_black_possible_to_castle_long() {
                return false;
            }
        }

        true
    }

    fn get_promotion_move(board: &Board, white_turn: bool, idx0: i32, idx1: i32) -> Option<Turn> {
        if white_turn && idx0 / 10 == 3 && board.get_field()[idx0 as usize] == 10 {
            Some(Turn::new(idx0, idx1).set_promotion(14)) // Promotion to Queen
        } else if !white_turn && idx0 / 10 == 8 && board.get_field()[idx0 as usize] == 20 {
            Some(Turn::new(idx0, idx1).set_promotion(24)) // Promotion to Queen
        } else {
            None
        }
    }

    pub fn generate_moves_list_for_piece(board: &Board, idx: i32) -> Vec<i32> {
        let check_idx_list = get_check_idx_list(board);
        let field = board.get_field();
        let white = board.is_white_to_move();

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


    pub fn get_attack_idx_list(field: &[i32], white: bool, mut target_idx: i32) -> Vec<i32> {
        let (white_king_pos, black_king_pos) = calc_king_positions(field);

        let mut check_idx_list = Vec::new();

        // Opponent's piece values
        let opponent_pawn = if white { 20 } else { 10 };
        let opponent_rook = if white { 21 } else { 11 };
        let opponent_knight = if white { 22 } else { 12 };
        let opponent_bishop = if white { 23 } else { 13 };
        let opponent_queen = if white { 24 } else { 14 };

        if target_idx == 0 {
            target_idx = if white { white_king_pos } else { black_king_pos };
        }

        // Pawns attacking
        if white {
            if field[(target_idx - 9) as usize] == opponent_pawn {
                check_idx_list.push(target_idx - 9);
            }
            if field[(target_idx - 11) as usize] == opponent_pawn {
                check_idx_list.push(target_idx - 11);
            }
        } else {
            if field[(target_idx + 9) as usize] == opponent_pawn {
                check_idx_list.push(target_idx + 9);
            }
            if field[(target_idx + 11) as usize] == opponent_pawn {
                check_idx_list.push(target_idx + 11);
            }
        }

        // Knights attacking
        let knight_offsets = [-12, -21, -8, -19, 12, 21, 8, 19];
        for &offset in &knight_offsets {
            if field[(target_idx + offset) as usize] == opponent_knight {
                check_idx_list.push(target_idx + offset);
            }
        }

        // Bishops and Queen attacking (Diagonals)
        let bishop_offsets = [-11, -9, 9, 11];
        for &offset in &bishop_offsets {
            let mut pos = target_idx;
            while field[(pos + offset) as usize] == 0 {
                pos += offset;
            }
            if field[(pos + offset) as usize] == opponent_bishop || field[(pos + offset) as usize] == opponent_queen {
                check_idx_list.push(pos + offset);
            }
        }

        // Rooks and Queen attacking (Horizontals and Verticals)
        let rook_offsets = [-10, -1, 1, 10];
        for &offset in &rook_offsets {
            let mut pos = target_idx;
            while field[(pos + offset) as usize] == 0 {
                pos += offset;
            }
            if field[(pos + offset) as usize] == opponent_rook || field[(pos + offset) as usize] == opponent_queen {
                check_idx_list.push(pos + offset);
            }
        }
        check_idx_list
    }

    /// Helper function to calculate the positions of the white and black kings.
    fn calc_king_positions(field: &[i32]) -> (i32, i32) {
        let mut white_king_pos = -1;
        let mut black_king_pos = -1;

        for i in 21..99 {
            if field[i] == 15 {
                white_king_pos = i as i32;
            }
            if field[i] == 25 {
                black_king_pos = i as i32;
            }
        }
        (white_king_pos, black_king_pos)
    }


    /// Checks if the king is under attack.
    pub fn get_check_idx_list(field: &[i32], white: bool) -> Vec<i32> {
        Self::get_attack_idx_list(field, white, 0)
    }
}
