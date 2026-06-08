use crate::model::{
    Board, INIT_BOARD_FEN,
    WHITE_PAWN, WHITE_ROOK, WHITE_KNIGHT, WHITE_BISHOP, WHITE_QUEEN, WHITE_KING,
    BLACK_PAWN, BLACK_ROOK, BLACK_KNIGHT, BLACK_BISHOP, BLACK_QUEEN, BLACK_KING,
};
use crate::notation_util::NotationUtil;

pub struct FenService;

impl FenService {
    /// Sets up a board from a given FEN string.
    pub fn set_fen(&self, fen: &str) -> Board {
        let mut bitboards = [0u64; 12];

        let parts: Vec<&str> = fen.split_whitespace().collect();
        let board_part = parts[0];
        let turn_part = parts[1];
        let castling_part = parts[2];
        let en_passant_part = parts[3];
        let move_number_part = if parts.len() > 5 { parts[5] } else { "1" };

        // Process the board position
        let mut rank = 7i32;
        let mut file = 0i32;

        for c in board_part.chars() {
            if c == '/' {
                rank -= 1;
                file = 0;
            } else if c.is_ascii_digit() {
                file += c.to_digit(10).unwrap() as i32;
            } else {
                let piece_idx = match c {
                    'P' => Some(WHITE_PAWN),
                    'R' => Some(WHITE_ROOK),
                    'N' => Some(WHITE_KNIGHT),
                    'B' => Some(WHITE_BISHOP),
                    'Q' => Some(WHITE_QUEEN),
                    'K' => Some(WHITE_KING),
                    'p' => Some(BLACK_PAWN),
                    'r' => Some(BLACK_ROOK),
                    'n' => Some(BLACK_KNIGHT),
                    'b' => Some(BLACK_BISHOP),
                    'q' => Some(BLACK_QUEEN),
                    'k' => Some(BLACK_KING),
                    _ => None,
                };
                if let Some(idx) = piece_idx {
                    let square = rank * 8 + file;
                    bitboards[idx] |= 1u64 << square;
                    file += 1;
                }
            }
        }

        // Process whose turn it is
        let white_to_move = turn_part == "w";

        // Process castling possibilities
        let white_possible_to_castle_short = castling_part.contains('K');
        let white_possible_to_castle_long = castling_part.contains('Q');
        let black_possible_to_castle_short = castling_part.contains('k');
        let black_possible_to_castle_long = castling_part.contains('q');

        // Process en passant possibility
        let mut field_for_en_passante = -1i8;
        if en_passant_part != "-" {
            field_for_en_passante = NotationUtil::get_index_from_notation_field(en_passant_part) as i8;
        }

        // Process move number
        let move_number = move_number_part.parse::<i32>().unwrap_or(1);

        let white_king = bitboards[WHITE_KING] != 0;
        let black_king = bitboards[BLACK_KING] != 0;

        Board::new(
            bitboards,
            white_possible_to_castle_long,
            white_possible_to_castle_short,
            black_possible_to_castle_long,
            black_possible_to_castle_short,
            field_for_en_passante,
            white_to_move,
            move_number,
            white_king,
            black_king,
        )
    }

    pub fn set_init_board(&self) -> Board {
        self.set_fen(INIT_BOARD_FEN)
    }

    /// Generates a FEN string from a given Board.
    pub fn get_fen(&self, board: &Board) -> String {
        let mut fen = String::new();
        let mut empty_count = 0;

        // Process board positions (ranks 7 down to 0, files 0 to 7)
        for rank in (0..8).rev() {
            for file in 0..8 {
                let square = rank * 8 + file;
                let piece = board.get_piece_at(square as u8);

                if piece == 0 {
                    empty_count += 1;
                } else {
                    if empty_count > 0 {
                        fen.push_str(&empty_count.to_string());
                        empty_count = 0;
                    }
                    let piece_char = match piece {
                        10 => 'P',
                        11 => 'R',
                        12 => 'N',
                        13 => 'B',
                        14 => 'Q',
                        15 => 'K',
                        20 => 'p',
                        21 => 'r',
                        22 => 'n',
                        23 => 'b',
                        24 => 'q',
                        25 => 'k',
                        _ => ' ',
                    };
                    if piece_char != ' ' {
                        fen.push(piece_char);
                    }
                }
            }
            if empty_count > 0 {
                fen.push_str(&empty_count.to_string());
                empty_count = 0;
            }
            if rank > 0 {
                fen.push('/');
            }
        }

        // Whose turn is it?
        fen.push(' ');
        fen.push(if board.white_to_move { 'w' } else { 'b' });

        // Castling rights
        fen.push(' ');
        let mut castling_rights = String::new();
        if board.white_possible_to_castle_short {
            castling_rights.push('K');
        }
        if board.white_possible_to_castle_long {
            castling_rights.push('Q');
        }
        if board.black_possible_to_castle_short {
            castling_rights.push('k');
        }
        if board.black_possible_to_castle_long {
            castling_rights.push('q');
        }
        if castling_rights.is_empty() {
            castling_rights.push('-');
        }
        fen.push_str(&castling_rights);

        // En passant target square
        fen.push(' ');
        if board.field_for_en_passante != -1 {
            fen.push_str(self.get_notation_from_index(board.field_for_en_passante).as_str());
        } else {
            fen.push('-');
        }

        // Halfmove clock and fullmove number
        fen.push_str(" 0 "); // Assuming halfmove clock is always 0 for now
        fen.push_str(&board.move_count.to_string());

        fen
    }

    /// Converts a board index to a notation field (e.g., 28 -> "e4").
    pub fn get_notation_from_index(&self, index: i8) -> String {
        if !(0..=63).contains(&index) {
            return String::from("-"); // Invalid index
        }

        let file_index = index % 8;
        let rank_index = index / 8;

        let file = (b'a' + file_index as u8) as char;
        let rank = (b'1' + rank_index as u8) as char;

        format!("{}{}", file, rank)
    }
}

#[cfg(test)]
mod tests {
    use crate::service::Service;

    #[test]
    fn fen_comparing_test() {
        let fen_service = Service::new().fen;

        let test_fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let board = fen_service.set_fen(&test_fen);
        let result_fen = fen_service.get_fen(&board);
        assert_eq!(test_fen, result_fen);

        let test_fen = "rnbq1rk1/pp2n1bp/2pppp2/6p1/3P4/2PBPP1P/PP1NN1PB/R2QK2R b KQ - 0 10";
        let board = fen_service.set_fen(&test_fen);
        let result_fen = fen_service.get_fen(&board);
        assert_eq!(test_fen, result_fen);

        let test_fen = "rnbqkbnr/ppp1pp1p/6p1/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3";
        let board = fen_service.set_fen(&test_fen);
        let result_fen = fen_service.get_fen(&board);
        assert_eq!(test_fen, result_fen);

        let test_fen = "rnbqk1nr/pp2ppbp/6p1/3pP3/1PpP4/2P2N2/P4PPP/RNBQKB1R b KQkq b3 0 6";
        let board = fen_service.set_fen(&test_fen);
        let result_fen = fen_service.get_fen(&board);
        assert_eq!(test_fen, result_fen);
    }
}