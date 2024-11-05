use crate::model::Board;
use crate::notation_util::NotationUtil;
use crate::zobrist::ZobristTable;


pub struct FenService;

impl FenService {
    /// Sets up a board from a given FEN string.
    pub fn set_fen(&self, fen: &str) -> Board {
        let mut field = [0; 120];
        self.clear_field(&mut field);

        let mut index = 21;
        let mut white_to_move = true;
        let mut white_possible_to_castle_long = false;
        let mut white_possible_to_castle_short = false;
        let mut black_possible_to_castle_long = false;
        let mut black_possible_to_castle_short = false;
        let mut line_for_en_passante = -1;
        let mut move_number = 1;

        let parts: Vec<&str> = fen.split_whitespace().collect();
        let board_part = parts[0];
        let turn_part = parts[1];
        let castling_part = parts[2];
        let en_passant_part = parts[3];
        let move_number_part = if parts.len() > 5 { parts[5] } else { "1" };

        // Process the board position
        for c in board_part.chars() {
            if c == '/' {
                index += 2; // Move to the next row
            } else if c.is_digit(10) {
                index += c.to_digit(10).unwrap() as usize; // Skip empty squares
            } else {
                let piece = match c {
                    'K' => 15,
                    'Q' => 14,
                    'R' => 11,
                    'B' => 13,
                    'N' => 12,
                    'P' => 10,
                    'k' => 25,
                    'q' => 24,
                    'r' => 21,
                    'b' => 23,
                    'n' => 22,
                    'p' => 20,
                    _ => 0, // Ignore invalid characters
                };
                if piece != 0 {
                    field[index] = piece;
                    index += 1;
                }
            }
        }

        // Process whose turn it is
        white_to_move = turn_part == "w";

        // Process castling possibilities
        white_possible_to_castle_short = castling_part.contains('K');
        white_possible_to_castle_long = castling_part.contains('Q');
        black_possible_to_castle_short = castling_part.contains('k');
        black_possible_to_castle_long = castling_part.contains('q');

        // Process en passant possibility
        if en_passant_part != "-" {
            line_for_en_passante = NotationUtil::get_index_from_notation_field(en_passant_part);
        }

        // Process move number
        move_number = move_number_part.parse::<i32>().unwrap_or(1);

        Board::new(
            field,
            white_possible_to_castle_long,
            white_possible_to_castle_short,
            black_possible_to_castle_long,
            black_possible_to_castle_short,
            line_for_en_passante,
            white_to_move,
            move_number,
            ZobristTable::new(),
        )
    }

    pub fn set_init_board(&self) -> Board {
        self.set_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
    }

    /// Clears the board by initializing all positions to -11 (out of bounds) or 0 (empty squares).
    fn clear_field(&self, field: &mut [i32; 120]) {
        for i in 0..field.len() {
            if i < 21 || i > 98 || i % 10 == 0 || i % 10 == 9 {
                field[i] = -11; // Set border squares to -11
            } else {
                field[i] = 0; // Set empty squares to 0
            }
        }
    }

    /// Generates a FEN string from a given Board.
    pub fn get_fen(&self, board: &Board) -> String {
        let mut fen = String::new();
        let mut empty_count = 0;

        // Process board positions
        for rank in 0..8 {
            for file in 0..8 {
                let index = 21 + rank * 10 + file;
                let piece = board.field[index];

                if piece == 0 {
                    empty_count += 1;
                } else {
                    if empty_count > 0 {
                        fen.push_str(&empty_count.to_string());
                        empty_count = 0;
                    }
                    let piece_char = match piece {
                        15 => 'K',
                        14 => 'Q',
                        11 => 'R',
                        13 => 'B',
                        12 => 'N',
                        10 => 'P',
                        25 => 'k',
                        24 => 'q',
                        21 => 'r',
                        23 => 'b',
                        22 => 'n',
                        20 => 'p',
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
            if rank < 7 {
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

    /// Converts a board index to a notation field (e.g., 34 -> "d6").
    pub fn get_notation_from_index(&self, index: i32) -> String {
        if index < 21 || index > 98 || index % 10 == 0 || index % 10 == 9 {
            return String::from("-"); // Invalid index
        }

        let rank_index = 7 - ((index - 21) / 10);
        let file_index = (index % 10) - 1;

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