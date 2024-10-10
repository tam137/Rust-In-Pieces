use crate::model::{Board, Turn};

pub struct MoveGenService;

impl MoveGenService {
    /// Generates a list of valid moves for a given board state.
    pub fn generate_valid_moves_list(&self, board: &mut Board) -> Vec<Turn> {
        let move_list = self.generate_moves_list_for_piece(board, 0);
        self.get_valid_moves_from_move_list(&move_list, board)
    }

    fn get_valid_moves_from_move_list(&self, move_list: &[i32], board: &mut Board) -> Vec<Turn> {
        let mut valid_moves = Vec::new();
        let white_turn = board.white_to_move;
        let king_value = if white_turn { 15 } else { 25 };

        for i in (0..move_list.len()).step_by(2) {
            let idx0 = move_list[i];
            let idx1 = move_list[i + 1];
            let mut move_turn = Turn::new(idx0, idx1, board.field[idx1 as usize], 0, 0);

            // Check for castling
            if board.field[idx0 as usize] == king_value && (idx1 == idx0 + 2 || idx1 == idx0 - 2) {
                if self.is_valid_castling(board, white_turn, idx1) {
                    continue;
                }
            }

            // Check for promotion
            if let Some(promotion_move) = self.get_promotion_move(board, white_turn, idx0, idx1) {
                move_turn.promotion = promotion_move.promotion;
            }

            // Perform the move
            let move_info = board.do_move(&move_turn);
            let mut valid = true;

            // Check if the move leads to check
            if !self.get_check_idx_list(&board.field, white_turn).is_empty() {
                valid = false;
            }

            // If valid, add the move to the list
            if valid {
                valid_moves.push(move_turn.clone());
                if let Some(promotion_move) = self.get_promotion_move(board, white_turn, idx0, idx1) {
                    let mut turn = move_turn.clone();
                    turn.promotion = promotion_move.promotion - 2;
                    valid_moves.push(turn); // Knight promotion
                }
            }

            board.undo_move(&move_turn, move_info);
        }
        valid_moves
    }

    fn is_valid_castling(&self, board: &Board, white_turn: bool, target: i32) -> bool {
        let check_squares = if white_turn {
            if target == 97 { vec![96, 97] } else { vec![94, 93] }
        } else {
            if target == 27 { vec![26, 27] } else { vec![24, 23] }
        };

        // Check if the king is currently in check
        if !self.get_check_idx_list(&board.field, white_turn).is_empty() {
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
                eval: 0,
            })
        } else if !white_turn && idx0 / 10 == 8 && board.field[idx0 as usize] == 20 {
            Some(Turn {
                from: idx0,
                to: idx1,
                capture: 0,
                promotion: 24,
                eval: 0,
            })
        } else {
            None
        }
    }


    pub fn generate_moves_list_for_piece(&self, board: &Board, idx: i32) -> Vec<i32> {
        let check_idx_list = self.get_check_idx_list(&board.field, board.white_to_move);
        let field = board.field;
        let white = board.white_to_move;

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


    pub fn get_attack_idx_list(&self, field: &[i32], white: bool, mut target_idx: i32) -> Vec<i32> {
        let (white_king_pos, black_king_pos) = self.calc_king_positions(field);

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
    fn calc_king_positions(&self, field: &[i32]) -> (i32, i32) {
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
    pub fn get_check_idx_list(&self, field: &[i32], white: bool) -> Vec<i32> {
        self.get_attack_idx_list(field, white, 0)
    }
}


#[cfg(test)]
mod tests {
    use crate::fen_service::FenServiceImpl;
    use super::*;

    #[test]
    fn get_check_idx_list_test() {
        let fen_service = FenServiceImpl;
        let move_gen_service = MoveGenService;

        // Test 1: Initial Board Setup - No Check
        let mut board = fen_service.set_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        assert!(move_gen_service.get_check_idx_list(&board.field, board.white_to_move).is_empty());

        // Test 2: Scenario where check occurs
        board = fen_service.set_fen("r1bqk1nr/pppp2pp/4p3/8/1b3P2/3PPn2/PPP2pPP/RNBQKBNR w KQkq - 0 1");
        let check_idx_list = move_gen_service.get_check_idx_list(&board.field, board.white_to_move);
        assert!(check_idx_list.contains(&86), "Check index list should contain 86");
        assert!(check_idx_list.contains(&76), "Check index list should contain 76");
        assert!(check_idx_list.contains(&62), "Check index list should contain 62");

        // Test 3: Black turn - No check
        board = fen_service.set_fen("r1bqk1nr/pppp2pp/4p3/8/1b3P2/3PPn2/PPP2pPP/RNBQKBNR b KQkq - 0 1");
        assert!(move_gen_service.get_check_idx_list(&board.field, board.white_to_move).is_empty());

        // Test 4: Two checks, positions 36 and 37
        board = fen_service.set_fen("r1bqk1nr/pppp1PNp/4p3/1Q5B/1b3P2/3PPn2/PPP2p1P/RN2KB1R b KQkq - 0 1");
        let check_idx_list = move_gen_service.get_check_idx_list(&board.field, board.white_to_move);
        assert_eq!(check_idx_list.len(), 2);
        assert!(check_idx_list.contains(&36), "Check index list should contain 36");
        assert!(check_idx_list.contains(&37), "Check index list should contain 37");

        // Test 5: Four checks in various positions
        board = fen_service.set_fen("r1Rqk2r/pppP2Np/5n2/3p3B/1b2QP2/3PPn2/PPP2p1P/RN2KB2 b KQkq - 0 1");
        let check_idx_list = move_gen_service.get_check_idx_list(&board.field, board.white_to_move);
        assert_eq!(check_idx_list.len(), 4);
        assert!(check_idx_list.contains(&37), "Check index list should contain 37");
        assert!(check_idx_list.contains(&58), "Check index list should contain 58");
        assert!(check_idx_list.contains(&65), "Check index list should contain 65");
        assert!(check_idx_list.contains(&34), "Check index list should contain 34");

        // Test 6: Four checks in another scenario
        board = fen_service.set_fen("2B5/6N1/4k3/8/2K2NP1/1B2Q2B/4R3/8 b - - 0 1");
        let check_idx_list = move_gen_service.get_check_idx_list(&board.field, board.white_to_move);
        assert!(check_idx_list.contains(&23), "Check index list should contain 23");
        assert!(check_idx_list.contains(&37), "Check index list should contain 37");
        assert!(check_idx_list.contains(&66), "Check index list should contain 66");
        assert!(check_idx_list.contains(&75), "Check index list should contain 75");
        assert_eq!(check_idx_list.len(), 4);

        // Test 7: Last check scenario with two checks
        board = fen_service.set_fen("8/1k6/8/1q6/2b1r3/8/1rn1K3/8 w - - 0 1");
        let check_idx_list = move_gen_service.get_check_idx_list(&board.field, board.white_to_move);
        assert!(check_idx_list.contains(&63), "Check index list should contain 63");
        assert!(check_idx_list.contains(&65), "Check index list should contain 65");
        assert_eq!(check_idx_list.len(), 2);
    }
}
