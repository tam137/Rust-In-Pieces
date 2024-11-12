use crate::model::Turn;
use regex::Regex;

pub struct NotationUtil;

impl NotationUtil {
    /// Converts a notation field (like "e2") to an index on the 10x12 board.
    pub fn get_index_from_notation_field(notation: &str) -> i32 {
        let col = match notation.chars().nth(0) {
            Some('a') => 1,
            Some('b') => 2,
            Some('c') => 3,
            Some('d') => 4,
            Some('e') => 5,
            Some('f') => 6,
            Some('g') => 7,
            Some('h') => 8,
            _ => -1
        };
        let row = 10 - notation.chars().nth(1).unwrap().to_digit(10).unwrap_or(0) as i32;
        (row * 10) + col
    }

    /// Converts a notation move (like "e2e4") to a `Turn` object.
    pub fn get_turn_from_notation(notation_move: &str) -> Turn {

        let valid_move_regex = Regex::new(r"^[a-h][1-8][a-h][1-8][qkbnr]?$").unwrap();
        if !valid_move_regex.is_match(notation_move) {
            panic!("Invalid chess move notation: Must be in standard algebraic format. But: '{}'", notation_move);
        }

        let from = NotationUtil::get_index_from_notation_field(&notation_move[0..2]);
        let to = NotationUtil::get_index_from_notation_field(&notation_move[2..4]);
        let mut promotion = 0;

        // Promotion logic for white
        if notation_move.len() == 5 && notation_move.chars().nth(3) == Some('8') {
            promotion = match notation_move.chars().nth(4) {
                Some('n') => 12,
                _ => 14, // default to queen
            };
        }

        // Promotion logic for black
        if notation_move.len() == 5 && notation_move.chars().nth(3) == Some('1') {
            promotion = match notation_move.chars().nth(4) {
                Some('n') => 22,
                _ => 24, // default to queen
            };
        }
        Turn::new(from, to, 0, promotion, 0, false)
    }

    /// Converts a space-separated list of notation moves (like "e2e4 e7e5") to a list of `Turn` objects.
    pub fn get_turn_list_from_notation(notation_move_list: &str) -> Vec<Turn> {
        let mut turn_list = Vec::new();
        let algebraic_move_list: Vec<&str> = notation_move_list.split_whitespace().collect();

        for algebraic_move in algebraic_move_list {
            let turn = NotationUtil::get_turn_from_notation(algebraic_move);
            turn_list.push(turn);
        }

        turn_list
    }

    /// Finds a specific move in the move list based on the notation.
    pub fn get_turn_from_list(move_list: &Vec<Turn>, notation: &str) -> Turn {
        let mut target_turn = NotationUtil::get_turn_from_notation(notation);

        // Handle promotion
        if notation.len() == 5 {
            match notation.chars().nth(4) {
                Some('q') => target_turn.promotion = 14,
                Some('n') => target_turn.promotion = 12,
                Some('Q') => target_turn.promotion = 14,
                Some('N') => target_turn.promotion = 12,
                _ => panic!("Invalid promotion"),
            }

            if target_turn.to / 90 == 1 {
                target_turn.promotion = target_turn.promotion + 10; // for black promotion
            }
        }

        for move_turn in move_list {
            if move_turn.from == target_turn.from
                && move_turn.to == target_turn.to
                && move_turn.promotion == target_turn.promotion
            {
                return move_turn.clone(); // Return the found move
            }
        }
        panic!("Turn not found in the move list for notation: {}", notation);
    }
}


#[cfg(test)]
mod tests {
    use crate::notation_util::NotationUtil;

    #[test]
    fn normal_notation_test() {
        let turn = NotationUtil::get_turn_from_notation("d7d5");
        assert_eq!(34, turn.from);
        assert_eq!(54, turn.to);
        assert_eq!(0, turn.capture);
        assert_eq!(false, turn.is_promotion());
        assert_eq!(false, turn.gives_check);        
    }

    #[test]
    fn get_index_from_notation_test() {
        // Test the conversion from notation ("a1", "h8", etc.) to the board index
        let idx = NotationUtil::get_index_from_notation_field("a1");
        assert_eq!(91, idx);

        let idx = NotationUtil::get_index_from_notation_field("h8");
        assert_eq!(28, idx);
    }

    #[test]
    fn get_turn_list_from_notation_test() {
        // Get a list of turns from the notation string
        let turn_list = NotationUtil::get_turn_list_from_notation("e2e4 d7d5 e4d5");

        // Check if the list has the correct number of turns
        assert_eq!(turn_list.len(), 3);

        // First turn (e2e4)
        let turn1 = &turn_list[0];
        assert_eq!(85, turn1.from);
        assert_eq!(65, turn1.to);
        assert_eq!(0, turn1.capture); // No capture
        assert_eq!(0, turn1.promotion); // Not a promotion move

        // Second turn (d7d5)
        let turn2 = &turn_list[1];
        assert_eq!(34, turn2.from);
        assert_eq!(54, turn2.to);

        // Third turn (e4d5)
        let turn3 = &turn_list[2];
        assert_eq!(65, turn3.from);
        assert_eq!(54, turn3.to);
        assert_eq!(0, turn3.capture); // No capture
        assert_eq!(0, turn3.promotion); // Not a promotion move
    }

    #[test]
    fn get_promotional_turn_test() {
        // Test promotion moves for white
        let mut turn = NotationUtil::get_turn_from_notation("e7e8q");
        assert_eq!(14, turn.promotion); // Promotion to queen

        turn = NotationUtil::get_turn_from_notation("e7e8n");
        assert_eq!(12, turn.promotion); // Promotion to knight

        turn = NotationUtil::get_turn_from_notation("e7d8b");
        assert_eq!(14, turn.promotion); // Promotion to bishop

        // Test promotion moves for black
        turn = NotationUtil::get_turn_from_notation("e2e1q");
        assert_eq!(24, turn.promotion); // Promotion to queen

        turn = NotationUtil::get_turn_from_notation("e2e1n");
        assert_eq!(22, turn.promotion); // Promotion to knight

        turn = NotationUtil::get_turn_from_notation("e2d1b");
        assert_eq!(24, turn.promotion); // Promotion to bishop        
    }

    #[test]
    #[should_panic(expected = "Invalid chess move notation: Must be in standard algebraic format. But: 'g1=Q+'")]
    fn test_invalid_notation_hash() {
        NotationUtil::get_turn_from_notation("g1=Q+");
    }


}