use crate::model::Board;
use crate::notation_util::NotationUtil;


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
        let mut line_for_en_passante = 0;
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
        )
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
}
