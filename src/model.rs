use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use std::sync::Mutex;
use std::sync::mpsc::Sender;
use std::time::Instant;

use crate::zobrist;
use crate::{notation_util::NotationUtil, zobrist::ZobristTable};


pub type ThreadSafeDataMap = Arc<RwLock<DataMap>>;
pub type LoggerFnType = Arc<dyn Fn(String) + Send + Sync>;

pub const INIT_BOARD_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

pub const RIP_COULDN_LOCK_ZOBRIST: &str = "RIP Could not lock zobrist mutex";
pub const RIP_COULDN_LOCK_GLOBAL_MAP: &str = "RIP Could not lock global map";
pub const RIP_COULDN_SEND_TO_HASH_QUEUE: &str = "RIP Could not Send hashes in hash queue";
pub const RIP_MISSED_DM_KEY: &str = "RIP Missed Data Map key";

#[derive(Clone)]
pub enum ValueType {
    Integer(i32),
    ArcMutexBool(Arc<Mutex<bool>>),
    LoggerFn(Arc<dyn Fn(String) + Send + Sync>),
    ArcRwZobrist(Arc<RwLock<ZobristTable>>),
    SenderU64I16(Sender<(u64, i16)>),
    Instant(Instant),
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum DataMapKey {
    WhiteThreshold,
    BlackThreshold,
    StopFlag,
    DebugFlag,
    Logger,
    ZobristTable,
    HashSender,
    CalcTime,
}

#[derive(Clone)]
pub struct DataMap {
    data_map: HashMap<DataMapKey, ValueType>,
}

impl DataMap {
    pub fn new() -> Self {
        DataMap {
            data_map: HashMap::new(),
        }
    }

    pub fn insert<T>(&mut self, key: DataMapKey, value: T)
    where
        DataMapKey: KeyToType<T>,
    {
        let data_value = key.create_value(value);
        self.data_map.insert(key, data_value);
    }

    pub fn get_data<'a, T>(&'a self, key: DataMapKey) -> Option<&'a T>
    where
        DataMapKey: KeyToType<T>,
    {
        self.data_map.get(&key).and_then(|value| key.get_value(value))
    }
}

pub trait KeyToType<T> {
    fn get_value<'a>(&self, value: &'a ValueType) -> Option<&'a T>;
    fn create_value(&self, value: T) -> ValueType;
}

impl KeyToType<i32> for DataMapKey {
    fn get_value<'a>(&self, value: &'a ValueType) -> Option<&'a i32> {
        match (self, value) {
            (DataMapKey::WhiteThreshold, ValueType::Integer(i))
            | (DataMapKey::BlackThreshold, ValueType::Integer(i)) => Some(i),
            _ => None,
        }
    }
    fn create_value(&self, value: i32) -> ValueType {
        ValueType::Integer(value)
    }
}

impl KeyToType<Arc<Mutex<bool>>> for DataMapKey {
    fn get_value<'a>(&self, value: &'a ValueType) -> Option<&'a Arc<Mutex<bool>>> {
        match (self, value) {
            (DataMapKey::StopFlag, ValueType::ArcMutexBool(a))
            | (DataMapKey::DebugFlag, ValueType::ArcMutexBool(a)) => Some(a),
            _ => None,
        }
    }
    fn create_value(&self, value: Arc<Mutex<bool>>) -> ValueType {
        ValueType::ArcMutexBool(value)
    }
}

impl KeyToType<Arc<dyn Fn(String) + Send + Sync>> for DataMapKey {
    fn get_value<'a>(&self, value: &'a ValueType) -> Option<&'a Arc<dyn Fn(String) + Send + Sync>> {
        match (self, value) {
            (DataMapKey::Logger, ValueType::LoggerFn(a)) => Some(a),
            _ => None,
        }
    }
    fn create_value(&self, value: Arc<dyn Fn(String) + Send + Sync>) -> ValueType {
        ValueType::LoggerFn(value)
    }
}

impl KeyToType<Arc<RwLock<ZobristTable>>> for DataMapKey {
    fn get_value<'a>(&self, value: &'a ValueType) -> Option<&'a Arc<RwLock<ZobristTable>>> {
        match (self, value) {
            (DataMapKey::ZobristTable, ValueType::ArcRwZobrist(a)) => Some(a),
            _ => None,
        }
    }
    fn create_value(&self, value: Arc<RwLock<ZobristTable>>) -> ValueType {
        ValueType::ArcRwZobrist(value)
    }
}

impl KeyToType<Sender<(u64, i16)>> for DataMapKey {
    fn get_value<'a>(&self, value: &'a ValueType) -> Option<&'a Sender<(u64, i16)>> {
        match (self, value) {
            (DataMapKey::HashSender, ValueType::SenderU64I16(a)) => Some(a),
            _ => None,
        }
    }
    fn create_value(&self, value: Sender<(u64, i16)>) -> ValueType {
        ValueType::SenderU64I16(value)
    }
}

impl KeyToType<Instant> for DataMapKey {
    fn get_value<'a>(&self, value: &'a ValueType) -> Option<&'a Instant> {
        match (self, value) {
            (DataMapKey::CalcTime, ValueType::Instant(a)) => Some(a),
            _ => None,
        }
    }
    fn create_value(&self, value: Instant) -> ValueType {
        ValueType::Instant(value)
    }
}


#[derive(Debug, PartialEq, Clone)]
pub enum GameStatus {
    Normal,
    Draw,
    WhiteWin,
    BlackWin,
}

#[derive(Debug, PartialEq, Clone)]
pub enum QuiescenceSearchMode {
    Alpha1,
    Alpha2,
    Alpha3,
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
    pub hash: u64,
}

impl PartialEq for Turn {
    fn eq(&self, other: &Self) -> bool {
        self.from == other.from
            && self.to == other.to
            && self.capture == other.capture
            && self.promotion == other.promotion
            && self.eval == other.eval
    }
}

impl PartialEq<&Turn> for Turn {
    fn eq(&self, other: &&Turn) -> bool {
        self == *other
    }
}

impl Turn {
    // Constructor with all fields
    pub fn new(from: i32, to: i32, capture: i32, promotion: i32, eval: i16) -> Self {
        Turn {
            from,
            to,
            capture,
            promotion,
            eval,
            hash: 0,
        }
    }

    pub fn new_to_from(from: i32, to: i32) -> Self {
        Turn {
            from,
            to,
            capture: 0,
            promotion: 0,
            eval: 0,
            hash: 0,
        }
    }

    // Check if the move is a promotion
    pub fn is_promotion(&self) -> bool {
        self.promotion != 0
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
    pub cached_hash: u64,
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
            cached_hash: 0,
        }
    }


    pub fn do_move(&mut self, turn: &Turn) -> MoveInformation {

        // validation
        if self.field[turn.from as usize] == 0 {
            panic!("RIP do_move(): Field on turn.from is 0\n{:?}", turn);
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
        if turn.hash == 0 {
            self.cached_hash = self.hash();
        } else {
            self.cached_hash = turn.hash;
        }
        self.move_repetition_map
            .entry(self.cached_hash)
            .and_modify(|count| *count += 1)
            .or_insert(1);

        // Check for 3-move repetition
        if let Some(&count) = self.move_repetition_map.get(&self.cached_hash) {
            if count > 3 { panic!("RIP move_repetition_map vale {}", count) }
            if count == 3 {
                self.game_status = GameStatus::Draw;
            }
        }
        MoveInformation::new(old_castle_information, self.cached_hash, old_field_for_en_passante)
    }


    pub fn undo_move(&mut self, turn: &Turn, move_information: MoveInformation) {

        self.cached_hash = 0;

        // validation
        if self.field[turn.to as usize] == 0 {
            panic!("RIP undo_move(): Field on turn.to is 0\n{:?}", turn);
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
        zobrist::gen(&self)
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

#[derive(Debug)]
pub struct Stats {
    pub best_turn_nr: i8,
    pub turn_number_gt_threshold: i32,
    pub created_nodes: usize,
    pub created_capture_node: usize,
    pub calculated_nodes: usize,
    pub eval_nodes: usize,
    pub calc_time_ms: usize,
    pub zobrist_hit: usize,
    pub cuts: i32,
    pub capture_share: i32,
    pub nodes_per_ms: i32,
    pub logging: Vec<String>,
}

impl Stats {
    pub fn new() -> Stats {
        Stats {
            best_turn_nr: 0,
            turn_number_gt_threshold: 0,
            calculated_nodes: 0,
            created_capture_node: 0,
            created_nodes: 0,
            eval_nodes: 0,
            calc_time_ms: 0,
            zobrist_hit: 0,
            cuts: 0,
            capture_share: 0,
            nodes_per_ms: 0,
            logging: Vec::default(),
         }
    }

    pub fn add_log(&mut self, msg: String) {
        self.logging.push(msg.to_string());
    }

    pub fn calculate(&mut self) {
        self.cuts = 100 - (self.calculated_nodes as i32 * 100 / self.created_nodes as i32);
        self.capture_share = self.created_capture_node as i32 * 100 / self.created_nodes as i32;
        self.nodes_per_ms = self.created_nodes as i32 / (self.calc_time_ms as i32 + 1);
        self.zobrist_hit = self.zobrist_hit * 100 / self.eval_nodes;
    }

    pub fn add_created_nodes(&mut self, value: usize) {
        self.created_nodes += value;
    }

    pub fn add_created_capture_nodes(&mut self, value: usize) {
        self.created_capture_node += value;
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

    pub fn add_turn_nr_gt_threshold(&mut self, value: i32) {
        self.turn_number_gt_threshold += value;
    }

    pub fn reset_stats(&mut self) {
        self.best_turn_nr = 0;
        self.turn_number_gt_threshold = 0;
        self.created_nodes = 0;
        self.created_capture_node = 0;
        self.calculated_nodes = 0;
        self.eval_nodes = 0;
        self.calc_time_ms = 0;
        self.zobrist_hit = 0;
        self.logging = Vec::default();
    }
}

#[derive(Default)]
pub struct SearchResult {
    pub variants: Vec<Variant>,
    pub is_white_move: bool,
}

#[derive(Debug)]
pub struct Variant {
    pub eval: i16,
    pub best_move: Option<Turn>,
    pub move_row: VecDeque<Option<Turn>>,
}

impl SearchResult {

    pub fn new() -> Self {
        SearchResult{
            variants: Vec::default(),
            is_white_move: true,        
        }
    }

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