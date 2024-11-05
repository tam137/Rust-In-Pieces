use std::collections::{HashMap, VecDeque};

use crate::{notation_util::NotationUtil, zobrist::ZobristTable};


#[derive(Debug, PartialEq, Clone)]
pub enum GameStatus {
    Normal,
    Draw,
    WhiteWin,
    BlackWin,
}


pub struct UciGame {
    pub board: Board,
    pub made_moves_str: String,
}

impl UciGame {

    pub fn new(board: Board) -> Self {
        UciGame {
            board,
            made_moves_str: String::from(""),
        }
    }

    pub fn do_move(&mut self, notation_move: &str) {
        self.board.do_move(&NotationUtil::get_turn_from_notation(notation_move));
        
        if self.made_moves_str.is_empty() {
            self.made_moves_str.push_str(notation_move);
        } else {
            self.made_moves_str.push(' ');
            self.made_moves_str.push_str(notation_move);
        }
    }

    pub fn white_to_move(&self) -> bool  {
        self.board.white_to_move
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

impl PartialEq for Turn {
    fn eq(&self, other: &Self) -> bool {
        self.from == other.from
            && self.to == other.to
            && self.capture == other.capture
            && self.promotion == other.promotion
            && self.eval == other.eval
            && self.gives_check == other.gives_check
    }
}

impl PartialEq<&Turn> for Turn {
    fn eq(&self, other: &&Turn) -> bool {
        self == *other
    }
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
            promotional_lit = if self.promotion % 10 == 4 { "q" } else { "k" };
        }
        format!(
            "{}{}{}{}{}",
            column_from as char, row_from as char, column_to as char, row_to as char, &promotional_lit
        )
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
    pub zobrist: ZobristTable,
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
        zobrist: ZobristTable,
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
            zobrist: ZobristTable::new(),
        }
    }


    pub fn do_move(&mut self, turn: &Turn) -> MoveInformation {

        // validation
        if self.field[turn.from as usize] == 0 {
            panic!("do_move(): Field on turn.from is 0\n{:?}", turn);
        }
        
        let old_castle_information = self.get_castle_information();
        let old_field_for_en_passante = self.field_for_en_passante;

        // Handling en passante information
        self.field_for_en_passante = -1;
        if (self.white_to_move && turn.from / 10 == 8 && turn.to / 10 == 6 && self.field[turn.from as usize] == 10) 
            || (!self.white_to_move && turn.from / 10 == 3 && turn.to / 10 == 5 && self.field[turn.from as usize] == 20) {
            
            let base = if self.white_to_move { 70 } else { 40 };
            self.field_for_en_passante = base + (turn.from % 10);
        }

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

        // Handle en passante
        if old_field_for_en_passante == turn.to && self.field[turn.to as usize] == 10 {
            self.field[(turn.to + 10) as usize] = 0;
        } else if old_field_for_en_passante == turn.to && self.field[turn.to as usize] == 20 {
            self.field[(turn.to - 10) as usize] = 0;
        }

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
            if count > 3 { panic!("move_repetition_map vale {}", count) }
            if count == 3 {
                self.game_status = GameStatus::Draw;
            }
        }
        MoveInformation::new(old_castle_information, board_hash, old_field_for_en_passante)
    }


    pub fn undo_move(&mut self, turn: &Turn, move_information: MoveInformation) {

        // validation
        if self.field[turn.to as usize] == 0 {
            panic!("undo_move(): Field on turn.to is 0\n{:?}", turn);
        }

        self.game_status = GameStatus::Normal;

        let castle_information = move_information.castle_information;
        let mut is_en_passante_move = false;

        // Handle en passante undo
        if turn.to == move_information.en_passante && self.field[turn.to as usize] == 10 {
            self.field[(turn.to + 10) as usize] = 20;
            is_en_passante_move = true;
        } else if turn.to == move_information.en_passante && self.field[turn.to as usize] == 20 {
            self.field[(turn.to - 10) as usize] = 10;
            is_en_passante_move = true;
        }

        // Handle promotion undo
        if turn.is_promotion() {
            self.field[turn.from as usize] = 10; // Reset to pawn
            if self.white_to_move {
                self.field[turn.from as usize] += 10; // Black pawn for black promotion
            }
        } else {
            self.field[turn.from as usize] = self.field[turn.to as usize];
        }

        if is_en_passante_move {
            self.field[turn.to as usize] = 0;
        } else {
            self.field[turn.to as usize] = turn.capture.max(0);
        }        

        // Restore castling rights and en passante information
        self.white_possible_to_castle_long = castle_information.white_possible_to_castle_long;
        self.white_possible_to_castle_short = castle_information.white_possible_to_castle_short;
        self.black_possible_to_castle_long = castle_information.black_possible_to_castle_long;
        self.black_possible_to_castle_short = castle_information.black_possible_to_castle_short;
        self.field_for_en_passante = move_information.en_passante;

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

    /// Generate the castle information based on the current state
    pub fn get_castle_information(&self) -> CastleInformation {
        CastleInformation {
            white_possible_to_castle_long: self.white_possible_to_castle_long,
            white_possible_to_castle_short: self.white_possible_to_castle_short,
            black_possible_to_castle_long: self.black_possible_to_castle_long,
            black_possible_to_castle_short: self.black_possible_to_castle_short,
        }
    }

    /// gives an indicator wich depth to the search can be applied. 100 is maximum
    pub fn calculate_complexity(&self) -> i32 {
        let mut complexity = 0;
        
        for &field in self.field.iter() {
            if field == 0 { 
                continue;
            }
            
            complexity += match field % 10 {
                0 => 1,  // pawn
                1 => 6,  // rook
                2 => 3,  // knight
                3 => 6,  // bishop
                4 => 12, // queen
                _ => 0,
            };
        }
        complexity
    }

    /// Zobrist-Hash function for the board (used for 3-move repetition and Zobrist-Hash Table)
    pub fn hash(&self) -> u64 {
        self.zobrist.gen(&self)
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

    pub fn get_depth(&self) -> i32 {
        if let Some(variant) = self.variants.get(0) {
            variant.move_row.len() as i32
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

    pub fn get_best_move_row(&self) -> String {
        if let Some(variant) = self.variants.get(0) {
            let move_row = variant.move_row.clone();
            return move_row.iter()
                .map(|turn_option| {
                    turn_option.as_ref().map(|turn| turn.to_algebraic()).unwrap_or_default()
                })
                .collect::<Vec<String>>()
                .join(" ");
        }
        String::new()
    }
    
    
}




#[cfg(test)]
mod tests {
    use crate::notation_util::NotationUtil;
    use crate::service::Service;
    use crate::UciGame;

    use super::{Board, GameStatus, Stats};

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
    fn do_move_en_passante_test() {
        let fen_service = Service::new().fen;

        let mut board = fen_service.set_init_board();
        let turn1 = NotationUtil::get_turn_from_notation("e2e4");
        let mi1 = board.do_move(&turn1);
        assert_eq!(75, board.field_for_en_passante);
        assert_eq!(-1, mi1.en_passante);

        let turn2 = NotationUtil::get_turn_from_notation("e7e5");
        let mi2 = board.do_move(&turn2);
        assert_eq!(45, board.field_for_en_passante);
        assert_eq!(75, mi2.en_passante);

        let turn3 = NotationUtil::get_turn_from_notation("d7d6");
        let mi3 = board.do_move(&turn3);
        assert_eq!(-1, board.field_for_en_passante);
        assert_eq!(45, mi3.en_passante);

        board.undo_move(&turn3, mi3);
        assert_eq!(45, board.field_for_en_passante);

        board.undo_move(&turn2, mi2);
        assert_eq!(75, board.field_for_en_passante);

        board.undo_move(&turn1, mi1);
        assert_eq!(-1, board.field_for_en_passante);
    }

    #[test]
    fn undo_move_en_passante_test() {
        let fen_service = Service::new().fen;

        // for white
        let mut board = fen_service.set_fen("rnbqkbnr/p1pppp1p/8/1p4pP/7R/8/PPPPPPP1/RNBQKBN1 w Qkq g6 0 4");
        let mut turn = NotationUtil::get_turn_from_notation("h5g6");
        turn.capture = 20;
        let mi = board.do_move(&turn);
        assert_eq!(47, mi.en_passante);
        assert_eq!(0, board.field[57]);

        board.undo_move(&turn, mi);
        assert_eq!(0, board.field[47]);
        assert_eq!(20, board.field[57]);
        assert_eq!(10, board.field[58]);

        // for black
        let mut board = fen_service.set_fen("rnbqkbnr/ppp1pppp/8/8/P1Pp4/8/1P1PPPPP/RNBQKBNR b KQkq c3 0 3");
        let mut turn = NotationUtil::get_turn_from_notation("d4c3");
        turn.capture = 10;
        let mi = board.do_move(&turn);
        assert_eq!(73, mi.en_passante);
        assert_eq!(0, board.field[63]);

        board.undo_move(&turn, mi);
        assert_eq!(0, board.field[73]);
        assert_eq!(10, board.field[63]);
        assert_eq!(20, board.field[64]);    
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

        let mut board = fen_service.set_init_board();
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

    #[test]
    fn repetition_map_test() {
        let fen_service = Service::new().fen;
        let move_gen = Service::new().move_gen;
    
        let mut game = UciGame::new(fen_service.set_init_board());
        let moves = "e2e4 d7d6 f1e2 b8d7 b1c3 g8f6 g1h3 a7a5 e1g1 c7c5 d2d4 c5d4 d1d4 e7e5 d4a4 h7h5 c1e3 f6g4 e3d2 g4f6 c3d5 f6d5 e4d5 d8b6 f1b1 f8e7 h3g5 b6d4 a4d4 e5d4 g5f3 a5a4 f3d4 g7g6 d4b5 a8b8 b2b3 a4b3 a2b3 h5h4 c2c4 d7e5 d2b4 c8d7 b4d6 e7d6 b5d6 e8e7 c4c5 f7f6 a1a7 d7f5 d6f5 g6f5 c5c6 e7d6 c6b7 f5f4 b1d1 d6d7 d5d6 h8g8 b3b4 f4f3 e2f3 e5f3 g1h1 f3e5 b4b5 e5f7 d1d4 f7d6 a7a6 b8b7 a6d6 d7c8 d4h4 b7b5 h4c4 c8b8 g2g4 f6f5 f2f3 b5b1 h1g2 f5g4 f3g4 b1b2 g2g3 b2b3 g3g2 b3b2 g2g3 b2b3 g3g2 b3b2";
    
        for notation_move in moves.split_whitespace() {
            game.do_move(notation_move);
            if game.board.game_status == GameStatus::Draw {
                assert_eq!("b3b2", notation_move);
            }
        }
        let moves = move_gen.generate_valid_moves_list(&mut game.board, &mut Stats::new(), &Service::new());
        assert_eq!(0, moves.len());
        assert_eq!(GameStatus::Draw, game.board.game_status);
    }
    

    #[test]
    fn uci_game_test() {
        let service = Service::new();

        let mut game = UciGame::new(service.fen.set_init_board());

        assert_eq!(true, game.white_to_move());
        assert_eq!("", game.made_moves_str);
        assert_eq!(1, game.board.move_count);

        game.do_move("e2e4");
        assert_eq!(false, game.white_to_move());
        assert_eq!("e2e4", game.made_moves_str);
        
        game.do_move("e7e5");
        assert_eq!(true, game.white_to_move());
        assert_eq!(2, game.board.move_count);

        game.do_move("d2d3");
        assert_eq!(false, game.white_to_move());
        assert_eq!(2, game.board.move_count);
        assert_eq!("e2e4 e7e5 d2d3", game.made_moves_str);
    }

    #[test]
    fn uci_game_en_passante_test() {
        let service = Service::new();
        let stats = &mut Stats::new();

        let mut game = UciGame::new(service.fen.set_init_board());
        game.do_move("e2e4");
        game.do_move("h7h6");
        game.do_move("e4e5");
        game.do_move("d7d5");

        let turns = service.move_gen.generate_valid_moves_list(&mut game.board, stats, &service);
        assert_eq!(31, turns.len());
        assert_eq!(20, turns.get(0).unwrap().capture);

        game.do_move("e5d6");
        assert_eq!(0, game.board.field[54]);
        assert_eq!(10, game.board.field[44]);
        assert_eq!(0, game.board.field[55]);


        let mut game = UciGame::new(service.fen.set_init_board());
        game.do_move("a2a4");
        game.do_move("d7d5");
        game.do_move("a4a5");
        game.do_move("d5d4");
        game.do_move("e2e4");

        let turns = service.move_gen.generate_valid_moves_list(&mut game.board, stats, &service);
        assert_eq!(29, turns.len());
        assert_eq!(10, turns.get(0).unwrap().capture);

        game.do_move("d4e3");
        assert_eq!(20, game.board.field[75]);
        assert_eq!(0, game.board.field[65]);
        assert_eq!(0, game.board.field[55]);
    }

    #[test]
    fn undo_capture_move_test_white() {
        let fen_service = Service::new().fen;

        let board = fen_service.set_init_board();
        let mut game = UciGame::new(board);
        game.do_move("e2e4");
        game.do_move("d7d5");
        game.do_move("b1c3");
        game.do_move("d5d4");
        game.do_move("e4e5");
        
        let mut capture_move = NotationUtil::get_turn_from_notation("d4c3");
        capture_move.capture = 12;
        let mi = game.board.do_move(&capture_move);
        assert_eq!(20, game.board.field[73]);

        
        game.board.undo_move(&capture_move, mi);
        assert_eq!(12, game.board.field[73]);
    }


    #[test]
    fn calculate_complexity_test() {
        let fen_service = Service::new().fen;

        let board = fen_service.set_init_board();
        assert_eq!(100, board.calculate_complexity());

        // midgame. Queen + 3 light pieces each + 2 rook
        let board = fen_service.set_fen("r2q1rk1/ppp2ppp/2n5/3p1b2/3Pn3/2PB1N2/P1Q2PPP/R1B2RK1 w - - 4 12");
        assert_eq!(88, board.calculate_complexity());

        // late midgame. Queen + 1 light pieces each + 1 rook
        let board = fen_service.set_fen("3q1rk1/Q1p2pp1/3n2p1/3p4/3P1B2/2P5/P4PPP/5RK1 b - - 0 20");
        assert_eq!(56, board.calculate_complexity());

        // rook endgame + 1 light peace each
        let board = fen_service.set_fen("r5k1/2B2pp1/6p1/3p4/3P4/2n5/P4PPP/R5K1 w - - 2 25");
        assert_eq!(30, board.calculate_complexity());
    }
}