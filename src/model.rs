use std::collections::{HashMap, VecDeque};

use crate::notation_util::NotationUtil;


#[derive(Debug, PartialEq, Clone)]
pub enum GameStatus {
    Normal,
    Draw,
    WhiteWin,
    BlackWin,
}


pub struct Uci_game {
    pub board: Board,
    pub  made_moves_str: String,
}

impl Uci_game {

    pub fn new(board: Board) -> Self {
        Uci_game {
            board,
            made_moves_str: String::from(""),
        }
    }

    pub fn do_move(&mut self, notation_move: &str) -> () {
        self.board.do_move(&NotationUtil::get_turn_from_notation(notation_move));
        self.made_moves_str.push_str(notation_move);
    }
}


#[derive(Debug, Clone)]
pub struct Turn {
    pub from: i32,
    pub to: i32,
    pub capture: i32,
    pub promotion: i32,
    pub eval: i16,
    pub gives_check: bool,
}

impl Turn {
    // Constructor with all fields
    pub fn new(from: i32, to: i32, capture: i32, promotion: i32, eval: i16, gives_check: bool) -> Self {
        Turn {
            from,
            to,
            capture,
            promotion,
            eval,
            gives_check,
        }
    }

    // Constructor with only 'from' and 'to' fields
    pub fn from_to(from: i32, to: i32) -> Self {
        Turn {
            from,
            to,
            capture: 0,
            promotion: 0,
            eval: 0,
            gives_check: false
        }
    }

    // Check if the move is a promotion
    pub fn is_promotion(&self) -> bool {
        self.promotion != 0
    }

    // Set promotion with fluent interface
    pub fn set_promotion(mut self, promotion: i32) -> Self {
        self.promotion = promotion;
        self
    }

    pub fn to_algebraic(&self) -> String {
        let column_from = (self.from % 10 + 96) as u8;
        let row_from = (10 - (self.from / 10) + 48) as u8;
        let column_to = (self.to % 10 + 96) as u8;
        let row_to = (10 - (self.to / 10) + 48) as u8;
        let mut promotional_lit = "";
        if self.promotion != 0 {
            promotional_lit = if self.promotion % 10 == 4 { "q" }
                                else {
                                    "k"
                                };
        }
        format!("{}{}{}{}{}", column_from as char, row_from as char, column_to as char, row_to as char, &promotional_lit)
    }
}



#[derive(Debug, Copy, Clone)]
pub struct MoveInformation {
    pub castle_information: CastleInformation,
    pub hash: u64,
    pub en_passante: i32,
}

impl MoveInformation {
    // Constructor
    pub fn new(castle_information: CastleInformation, hash: u64, en_passante: i32) -> Self {
        MoveInformation {
            castle_information,
            hash,
            en_passante,
        }
    }
}


#[derive(Debug, Copy, Clone)]
pub struct CastleInformation {
    pub white_possible_to_castle_long: bool,
    pub white_possible_to_castle_short: bool,
    pub black_possible_to_castle_long: bool,
    pub black_possible_to_castle_short: bool,
}

impl CastleInformation {
    // Constructor
    pub fn new(white_possible_to_castle_long: bool, white_possible_to_castle_short: bool,
               black_possible_to_castle_long: bool, black_possible_to_castle_short: bool) -> Self {
        CastleInformation {
            white_possible_to_castle_long,
            white_possible_to_castle_short,
            black_possible_to_castle_long,
            black_possible_to_castle_short,
        }
    }
}


#[derive(Debug, Clone)]
pub struct Board {
    pub field: [i32; 120],
    pub white_possible_to_castle_long: bool,
    pub white_possible_to_castle_short: bool,
    pub black_possible_to_castle_long: bool,
    pub black_possible_to_castle_short: bool,
    pub field_for_en_passante: i32,  // 0 if no en passant possible, only used by fen import
    pub white_to_move: bool,
    pub move_count: i32,
    pub game_status: GameStatus,
    pub move_repetition_map: HashMap<u64, i32>,
    pub current_best_eval: i16,
}

impl Board {
    // Constructor
    pub fn new(
        field: [i32; 120],
        white_possible_to_castle_long: bool,
        white_possible_to_castle_short: bool,
        black_possible_to_castle_long: bool,
        black_possible_to_castle_short: bool,
        field_for_en_passante: i32,
        white_to_move: bool,
        move_count: i32,
    ) -> Self {
        Board {
            field,
            white_possible_to_castle_long,
            white_possible_to_castle_short,
            black_possible_to_castle_long,
            black_possible_to_castle_short,
            field_for_en_passante,
            white_to_move,
            move_count,
            game_status: GameStatus::Normal,
            move_repetition_map: HashMap::new(),
            current_best_eval: 0,
        }
    }


    // Method for performing a move
    pub fn do_move(&mut self, turn: &Turn) -> MoveInformation {
        self.field_for_en_passante = 0;

        let old_castle_information = self.get_castle_information();

        // Handling castling for white and black
        if self.field[turn.from as usize] == 15 || self.field[turn.from as usize] == 25 {
            match (turn.from, turn.to) {
                (25, 27) => {
                    self.field[28] = 0;
                    self.field[26] = 21;
                }
                (25, 23) => {
                    self.field[21] = 0;
                    self.field[24] = 21;
                }
                (95, 97) => {
                    self.field[98] = 0;
                    self.field[96] = 11;
                }
                (95, 93) => {
                    self.field[91] = 0;
                    self.field[94] = 11;
                }
                _ => {}
            }
        }

        // Update castling rights
        match turn.from {
            21 => self.black_possible_to_castle_long = false,
            28 => self.black_possible_to_castle_short = false,
            25 => {
                self.black_possible_to_castle_long = false;
                self.black_possible_to_castle_short = false;
            }
            91 => self.white_possible_to_castle_long = false,
            98 => self.white_possible_to_castle_short = false,
            95 => {
                self.white_possible_to_castle_long = false;
                self.white_possible_to_castle_short = false;
            }
            _ => {}
        }

        // Handle promotion
        if turn.is_promotion() {
            self.field[turn.to as usize] = turn.promotion;
        } else {
            self.field[turn.to as usize] = self.field[turn.from as usize];
        }

        self.field[turn.from as usize] = 0;

        // Increment move count if it's black's turn
        if !self.white_to_move {
            self.move_count += 1;
        }
        self.white_to_move = !self.white_to_move;

        // Calculate the board hash and update the move repetition map
        let board_hash = self.hash();
        self.move_repetition_map
            .entry(board_hash)
            .and_modify(|count| *count += 1)
            .or_insert(1);

        // Check for 3-move repetition
        if let Some(&count) = self.move_repetition_map.get(&board_hash) {
            if count == 3 {
                self.game_status = GameStatus::Draw;
            }
        }
        MoveInformation::new(old_castle_information, board_hash, self.field_for_en_passante)
    }


    // Undo move
    pub fn undo_move(&mut self, turn: &Turn, move_information: MoveInformation) {
        self.game_status = GameStatus::Normal;

        let castle_information = move_information.castle_information;

        // Handle promotion undo
        if turn.is_promotion() {
            self.field[turn.from as usize] = 10; // Reset to pawn
            if self.white_to_move {
                self.field[turn.from as usize] += 10; // Black pawn for black promotion
            }
        } else {
            self.field[turn.from as usize] = self.field[turn.to as usize];
        }

        self.field[turn.to as usize] = turn.capture.max(0);

        // Restore castling rights
        self.white_possible_to_castle_long = castle_information.white_possible_to_castle_long;
        self.white_possible_to_castle_short = castle_information.white_possible_to_castle_short;
        self.black_possible_to_castle_long = castle_information.black_possible_to_castle_long;
        self.black_possible_to_castle_short = castle_information.black_possible_to_castle_short;

        // Handle castling undo
        if self.field[turn.from as usize] == 15 || self.field[turn.from as usize] == 25 {
            match (turn.from, turn.to) {
                (25, 27) => {
                    self.field[28] = 21;
                    self.field[26] = 0;
                }
                (25, 23) => {
                    self.field[21] = 21;
                    self.field[24] = 0;
                }
                (95, 97) => {
                    self.field[98] = 11;
                    self.field[96] = 0;
                }
                (95, 93) => {
                    self.field[91] = 11;
                    self.field[94] = 0;
                }
                _ => {}
            }
        }

        // Decrement move count if it was white's move
        if self.white_to_move {
            self.move_count -= 1;
        }
        self.white_to_move = !self.white_to_move;

        // Update the move repetition map
        if let Some(count) = self.move_repetition_map.get_mut(&move_information.hash) {
            if *count > 1 {
                *count -= 1;
            } else {
                self.move_repetition_map.remove(&move_information.hash);
            }
        }
    }

    // Generate the castle information based on the current state
    pub fn get_castle_information(&self) -> CastleInformation {
        CastleInformation {
            white_possible_to_castle_long: self.white_possible_to_castle_long,
            white_possible_to_castle_short: self.white_possible_to_castle_short,
            black_possible_to_castle_long: self.black_possible_to_castle_long,
            black_possible_to_castle_short: self.black_possible_to_castle_short,
        }
    }

    // Hash function for the board (used for 3-move repetition)
    pub fn hash(&self) -> u64 {
        let mut hash = 17u64;
        hash = hash.wrapping_mul(31).wrapping_add(self.field.iter().fold(0, |acc, &x| acc.wrapping_add(x as u64)));
        hash = hash.wrapping_mul(31).wrapping_add(self.white_possible_to_castle_long as u64);
        hash = hash.wrapping_mul(31).wrapping_add(self.white_possible_to_castle_short as u64);
        hash = hash.wrapping_mul(31).wrapping_add(self.black_possible_to_castle_long as u64);
        hash = hash.wrapping_mul(31).wrapping_add(self.black_possible_to_castle_short as u64);
        hash = hash.wrapping_mul(31).wrapping_add(self.field_for_en_passante as u64);
        hash = hash.wrapping_mul(31).wrapping_add(self.white_to_move as u64);
        hash
    }
}

// Implement `PartialEq` manually for the `Board` struct, for unittests
impl PartialEq for Board {
    fn eq(&self, other: &Self) -> bool {
        // Check if all the fields of the Board match
        self.white_possible_to_castle_long == other.white_possible_to_castle_long &&
            self.white_possible_to_castle_short == other.white_possible_to_castle_short &&
            self.black_possible_to_castle_long == other.black_possible_to_castle_long &&
            self.black_possible_to_castle_short == other.black_possible_to_castle_short &&
            self.field_for_en_passante == other.field_for_en_passante &&
            self.white_to_move == other.white_to_move &&
            self.move_count == other.move_count &&
            self.game_status == other.game_status &&
            self.field == other.field &&  // Direct comparison of arrays (fixed-size arrays implement PartialEq)
            self.move_repetition_map == other.move_repetition_map  // HashMap comparison
    }
}

pub struct Stats {
    pub created_nodes: usize,
    calculated_nodes: usize,
    eval_nodes: usize,
    calc_time_ms: usize,
    zobrist_hit: usize,
}

impl Stats {
    pub fn new() -> Stats {
        Stats { calc_time_ms: 0, calculated_nodes: 0, created_nodes: 0, eval_nodes: 0, zobrist_hit: 0 }
    }

    pub fn add_created_nodes(&mut self, value: usize) {
        self.created_nodes += value;
    }

    pub fn add_calculated_nodes(&mut self, value: usize) {
        self.calculated_nodes += value;
    }

    pub fn add_eval_nodes(&mut self, value: usize) {
        self.eval_nodes += value;
    }

    pub fn add_zobrist_hit(&mut self, value: usize) {
        self.zobrist_hit += value;
    }

    pub fn set_calc_time(&mut self, value: usize) {
        self.calc_time_ms = value;
    }

    pub fn reset_stats(&mut self) {
        self.created_nodes = 0;
        self.calculated_nodes = 0;
        self.eval_nodes = 0;
        self.calc_time_ms = 0;
        self.zobrist_hit = 0;
    }

    pub fn to_string(&self) -> String {
        format!("Cre_{}\tCalc_{}\tEva_{}\tN/s_{}K CF_0.{}\tZb_0.{}",
                self.created_nodes,
                self.calculated_nodes,
                self.eval_nodes,
                self.created_nodes / (self.calc_time_ms + 1),
                100 - (self.calculated_nodes * 100 / if self.created_nodes == 0 { 1 } else { self.created_nodes }),
                //self.zobrist_hit)
                self.zobrist_hit * 100 / self.eval_nodes)
    }
}

#[derive(Default)]
pub struct SearchResult {
    pub variants: Vec<Variant>
}

#[derive(Debug)]
pub struct Variant {
    pub eval: i16,
    pub best_move: Option<Turn>,
    pub move_row: VecDeque<Option<Turn>>,
}

impl SearchResult {
    pub fn add_variant(&mut self, variant: Variant) {
        self.variants.push(variant);
    }

    pub fn get_eval(&self) -> i16 {
        if let Some(variant) = self.variants.get(0) {
            variant.eval
        } else {
            0
        }
    }

    pub fn print_debug(&self) {
        if let Some(variant) = self.variants.get(0) {
            print!("{:?}", variant);
        } else {
            println!("No variants available");
        }
    }

    pub fn print_best_variant(&self) {
        if let Some(variant) = self.variants.get(0) {
            print!("{} ", self.get_eval());
            let move_row = variant.move_row.clone();            
            move_row.iter()
                .map(|turn_option| {
                    turn_option.as_ref().map(|turn| turn.to_algebraic()).unwrap_or_default()
                })
                .for_each(|algebraic_turn| print!("{} ", algebraic_turn));
        }
    }

    pub fn print_all_variants(&self) {
        self.variants.iter().for_each(|variant| {
            print!("{:>6} ", variant.eval);
            let move_row = variant.move_row.clone();            
            move_row.iter()
                .map(|turn_option| {
                    turn_option.as_ref().map(|turn| turn.to_algebraic()).unwrap_or_default()
                })
                .for_each(|algebraic_turn| print!("{} ", algebraic_turn));
            println!();
        });
    }

    pub fn get_best_move_algebraic(&self) -> String {
        self.variants.get(0)
            .and_then(|variant| variant.best_move.as_ref())
            .map(|best_move| best_move.to_algebraic())
            .unwrap_or_else(|| "N/A".to_string())
    }
    
    
}




#[cfg(test)]
mod tests {
    use crate::notation_util::NotationUtil;
    use crate::service::Service;

    #[test]
    fn board_properties_move_count_test() {
        let fen_service = Service::new().fen;
        // Create a new board with the FEN string using your chess library.
        let mut board = fen_service.set_fen("r1bqkb1r/ppppn2p/2n2pp1/4p3/2B1P3/5N1P/PPPP1PP1/RNBQ1RK1 w kq - 0 6");

        // Assert the initial move count is 6
        assert_eq!(board.move_count, 6);

        // Get two turns from notation, d2d3 and f8g7 (e.g. pawn move and bishop move)
        let turn1 = &NotationUtil::get_turn_from_notation("d2d3");
        let turn2 = &NotationUtil::get_turn_from_notation("f8g7");

        // Execute the first move and store the move information (mi1)
        let mi1 = board.do_move(turn1);
        // Ensure move count hasn't changed yet
        assert_eq!(board.move_count, 6);

        // Execute the second move and store the move information (mi2)
        let mi2 = board.do_move(turn2);

        // Check black castling rights after the move
        assert!(mi2.castle_information.black_possible_to_castle_long);
        assert!(mi2.castle_information.black_possible_to_castle_short);

        // Check white castling rights
        assert!(!mi2.castle_information.white_possible_to_castle_long);
        assert!(!mi2.castle_information.white_possible_to_castle_short);

        assert_eq!(board.move_count, 7);

        // Undo the second move
        board.undo_move(turn2, mi2);
        assert_eq!(board.move_count, 6);

        // Undo the first move
        board.undo_move(turn1, mi1);
        // Move count should remain at 6
        assert_eq!(board.move_count, 6);

        // Castling rights should be restored as before
        assert!(mi2.castle_information.black_possible_to_castle_long);
        assert!(mi2.castle_information.black_possible_to_castle_short);
        assert!(!mi2.castle_information.white_possible_to_castle_long);
        assert!(!mi2.castle_information.white_possible_to_castle_short);
    }

    #[test]
    fn do_move_castle_test() {
        let fen_service = Service::new().fen;

        let mut board = fen_service.set_fen("r3k2r/pppqbppp/2npbn2/1B2p3/3PP3/2N1BN2/PPP1QPPP/R3K2R w KQkq - 0 6");
        let init_board = board.clone();

        // Get the four castle moves for black and white, short and long
        let castle_white_short = &NotationUtil::get_turn_from_notation("e1g1");
        let castle_white_long = &NotationUtil::get_turn_from_notation("e1c1");
        let castle_black_short = &NotationUtil::get_turn_from_notation("e8g8");
        let castle_black_long = &NotationUtil::get_turn_from_notation("e8c8");

        // White short castle
        let mi1 = board.do_move(castle_white_short);
        assert_eq!(board.field[97], 15);
        assert_eq!(board.field[96], 11);
        assert!(!board.get_castle_information().white_possible_to_castle_short);
        assert!(!board.get_castle_information().white_possible_to_castle_long);
        assert!(board.get_castle_information().black_possible_to_castle_short);
        assert!(board.get_castle_information().black_possible_to_castle_long);
        assert_ne!(board, init_board);

        // Undo white short castle
        board.undo_move(castle_white_short, mi1);
        assert!(board.get_castle_information().white_possible_to_castle_short);
        assert!(board.get_castle_information().white_possible_to_castle_long);
        assert!(board.get_castle_information().black_possible_to_castle_short);
        assert!(board.get_castle_information().black_possible_to_castle_long);
        assert_eq!(board.field[95], 15);
        assert_eq!(board.field[98], 11);
        assert_eq!(board, init_board);

        // White long castle
        let mi2 = board.do_move(castle_white_long);
        assert_eq!(board.field[93], 15);
        assert_eq!(board.field[94], 11);
        assert!(!board.get_castle_information().white_possible_to_castle_short);
        assert!(!board.get_castle_information().white_possible_to_castle_long);
        assert!(board.get_castle_information().black_possible_to_castle_short);
        assert!(board.get_castle_information().black_possible_to_castle_long);
        assert_ne!(board, init_board);

        // Undo white long castle
        board.undo_move(castle_white_long, mi2);
        assert!(board.get_castle_information().white_possible_to_castle_short);
        assert!(board.get_castle_information().white_possible_to_castle_long);
        assert!(board.get_castle_information().black_possible_to_castle_short);
        assert!(board.get_castle_information().black_possible_to_castle_long);
        assert_eq!(board.field[95], 15);
        assert_eq!(board.field[98], 11);
        assert_eq!(board, init_board);

        // Black short castle
        let mi3 = board.do_move(castle_black_short);
        assert_eq!(board.field[27], 25);
        assert_eq!(board.field[26], 21);
        assert!(board.get_castle_information().white_possible_to_castle_short);
        assert!(board.get_castle_information().white_possible_to_castle_long);
        assert!(!board.get_castle_information().black_possible_to_castle_short);
        assert!(!board.get_castle_information().black_possible_to_castle_long);
        assert_ne!(board, init_board);

        // Undo black short castle
        board.undo_move(castle_black_short, mi3);
        assert!(board.get_castle_information().white_possible_to_castle_short);
        assert!(board.get_castle_information().white_possible_to_castle_long);
        assert!(board.get_castle_information().black_possible_to_castle_short);
        assert!(board.get_castle_information().black_possible_to_castle_long);
        assert_eq!(board.field[25], 25);
        assert_eq!(board.field[28], 21);
        assert_eq!(board, init_board);

        // Black long castle
        let mi4 = board.do_move(castle_black_long);
        assert_eq!(board.field[23], 25);
        assert_eq!(board.field[24], 21);
        assert!(board.get_castle_information().white_possible_to_castle_short);
        assert!(board.get_castle_information().white_possible_to_castle_long);
        assert!(!board.get_castle_information().black_possible_to_castle_short);
        assert!(!board.get_castle_information().black_possible_to_castle_long);
        assert_ne!(board, init_board);

        // Undo black long castle
        board.undo_move(castle_black_long, mi4);
        assert!(board.get_castle_information().white_possible_to_castle_short);
        assert!(board.get_castle_information().white_possible_to_castle_long);
        assert!(board.get_castle_information().black_possible_to_castle_short);
        assert!(board.get_castle_information().black_possible_to_castle_long);
        assert_eq!(board.field[25], 25);
        assert_eq!(board.field[28], 21);
        assert_eq!(board, init_board);
    }

    #[test]
    fn board_properties_castle_information_test() {
        let fen_service = Service::new().fen;

        let mut board = fen_service.set_fen("r1bqk2r/ppppn1bp/2n2pp1/1B2p3/4P3/3P1N1P/PPP2PP1/RNBQ1RK1 b kq - 0 6");
        let init_board = board.clone();

        // Check that the initial board is the same as the cloned board
        assert_eq!(board, init_board);

        // Perform black short castle
        let ci1 = board.do_move(&NotationUtil::get_turn_from_notation("e8g8"));
        assert!(ci1.castle_information.black_possible_to_castle_long);
        assert!(ci1.castle_information.black_possible_to_castle_short);
        assert!(!ci1.castle_information.white_possible_to_castle_long);
        assert!(!ci1.castle_information.white_possible_to_castle_short);
        assert_ne!(board, init_board);

        // Perform the move f1e1 and check castling rights again
        let ci2 = board.do_move(&NotationUtil::get_turn_from_notation("f1e1"));
        assert!(!ci2.castle_information.black_possible_to_castle_long);
        assert!(!ci2.castle_information.black_possible_to_castle_short);
        assert!(!ci2.castle_information.white_possible_to_castle_long);
        assert!(!ci2.castle_information.white_possible_to_castle_short);

        // Undo the last move f1e1
        board.undo_move(&NotationUtil::get_turn_from_notation("f1e1"), ci2);

        // Undo the black short castle
        board.undo_move(&NotationUtil::get_turn_from_notation("e8g8"), ci1);

        // After undoing both moves, the castling rights should be restored
        assert!(ci1.castle_information.black_possible_to_castle_long);
        assert!(ci1.castle_information.black_possible_to_castle_short);
        assert!(!ci1.castle_information.white_possible_to_castle_long);
        assert!(!ci1.castle_information.white_possible_to_castle_short);

        // The board should now be identical to the initial state
        assert_eq!(board, init_board);
    }

    #[test]
    fn hash_test() {
        let fen_service = Service::new().fen;

        let mut board = fen_service.set_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        let org_hash = board.hash();

        // Get the move "e2e4" and apply it to the board
        let turn = &NotationUtil::get_turn_from_notation("e2e4");
        let mi = board.do_move(turn);

        // Ensure the hash has changed after the move
        assert_ne!(org_hash, board.hash());
        assert_eq!(board.move_repetition_map.len(), 1);
        board.undo_move(turn, mi);
        assert_eq!(org_hash, board.hash());
        assert_eq!(board.move_repetition_map.len(), 0);
    }

}