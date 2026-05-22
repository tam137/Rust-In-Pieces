use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use crate::zobrist;
use crate::{notation_util::NotationUtil, zobrist::ZobristTable};

pub const INIT_BOARD_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

pub const RIP_COULDN_LOCK_MUTEX: &str = "RIP Could not lock mutex";

pub const RIP_COULDN_SEND_TO_STD_IN_QUEUE: &str = "RIP Could not Send commands to std in queue";
pub const RIP_COULDN_SEND_TO_GAME_CMD_QUEUE: &str = "RIP Could not Send commands to game command queue";
pub const RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE: &str = "RIP Could not Send msg to log buffer queue";
pub const RIP_MISSED_DM_KEY: &str = "RIP Missed Data Map key";
pub const RIP_ERR_READING_STD_IN: &str = "RIP Error reading std input";
pub const RIP_COULDN_JOIN_THREAD: &str = "RIP Could not join thread";

#[derive(Clone)]
pub enum ValueType {
    Integer(i32),
    Bool(bool),
    Instant(Instant),
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum DataMapKey {
    WhiteThreshold,
    BlackThreshold,
    CalcTime,
    MoveOrderingFlag,
    ForceSkipValidationFlag,
    WhiteGivesCheck,
    BlackGivesCheck,
    PvFlag,
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
            (DataMapKey::WhiteThreshold, ValueType::Integer(i)) |
            (DataMapKey::BlackThreshold, ValueType::Integer(i)) => Some(i),
            _ => None,
        }
    }
    fn create_value(&self, value: i32) -> ValueType {
        ValueType::Integer(value)
    }
}

impl KeyToType<bool> for DataMapKey {
    fn get_value<'a>(&self, value: &'a ValueType) -> Option<&'a bool> {
        match (self, value) {
            (DataMapKey::PvFlag, ValueType::Bool(i)) |
            (DataMapKey::MoveOrderingFlag, ValueType::Bool(i)) |
            (DataMapKey::ForceSkipValidationFlag, ValueType::Bool(i)) |
            (DataMapKey::WhiteGivesCheck, ValueType::Bool(i)) |
            (DataMapKey::BlackGivesCheck, ValueType::Bool(i)) => Some(i),
            _ => None,
        }
    }
    fn create_value(&self, value: bool) -> ValueType {
        ValueType::Bool(value)
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

pub struct EngineState {
    pub stop_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub debug_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub zobrist_table: std::sync::Arc<ZobristTable>,
    pub pv_nodes: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<u64, Turn>>>,
    pub pv_nodes_len: std::sync::Arc<std::sync::atomic::AtomicI32>,
    pub logger: std::sync::Arc<std::sync::RwLock<std::sync::Arc<dyn Fn(String) + Send + Sync>>>,
    pub log_sender: std::sync::mpsc::Sender<String>,
}

pub struct SearchContext<'a> {
    pub zobrist_table: &'a ZobristTable,
    pub stop_flag: &'a std::sync::atomic::AtomicBool,
    pub pv_nodes: &'a std::sync::Mutex<std::collections::HashMap<u64, Turn>>,
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
    Alpha2,
    Alpha3,
}

#[derive(Clone)]
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


#[derive(Debug, PartialEq, Clone)]
pub enum TimeMode {
    Movetime,
    MoveToGo,
    HourGlas,
    Depth,
    None,
}

#[derive(Debug, PartialEq, Clone)]
pub struct TimeInfo {
    pub wtime: i32,
    pub btime: i32,
    pub winc: i32,
    pub binc: i32,
    pub moves_to_go: i32,
    pub depth: i32,
    pub time_mode: TimeMode,    
}


pub const WHITE_PAWN: usize = 0;
pub const WHITE_ROOK: usize = 1;
pub const WHITE_KNIGHT: usize = 2;
pub const WHITE_BISHOP: usize = 3;
pub const WHITE_QUEEN: usize = 4;
pub const WHITE_KING: usize = 5;
pub const BLACK_PAWN: usize = 6;
pub const BLACK_ROOK: usize = 7;
pub const BLACK_KNIGHT: usize = 8;
pub const BLACK_BISHOP: usize = 9;
pub const BLACK_QUEEN: usize = 10;
pub const BLACK_KING: usize = 11;

#[derive(Debug, Clone)]
pub struct Turn {
    pub from: u8,
    pub to: u8,
    pub capture: u8,
    pub promotion: u8,
    pub gives_check: bool,
    pub eval: i16,
    pub hash: u64,
    pub has_hashed_eval: bool,
    pub rank: i32,
}

impl PartialEq for Turn {
    fn eq(&self, other: &Self) -> bool {
        self.from == other.from
            && self.to == other.to
            && self.capture == other.capture
            && self.promotion == other.promotion
    }
}

impl PartialEq<&Turn> for Turn {
    fn eq(&self, other: &&Turn) -> bool {
        self == *other
    }
}

impl Turn {
    // Constructor with all fields
    pub fn new(from: u8, to: u8, capture: u8, promotion: u8, gives_check: bool, eval: i16) -> Self {
        Turn {
            from,
            to,
            capture,
            promotion,
            gives_check,
            eval,
            hash: 0,
            has_hashed_eval: false,
            rank: 0,
        }
    }

    pub fn _new_to_from(from: u8, to: u8) -> Self {
        Turn {
            from,
            to,
            capture: 0,
            promotion: 0,
            gives_check: false,
            eval: 0,
            hash: 0,
            has_hashed_eval: false,
            rank: 0,
        }
    }

    // Check if the move is a promotion
    pub fn is_promotion(&self) -> bool {
        self.promotion != 0
    }

    pub fn to_algebraic(&self) -> String {
        let col_from = self.from % 8;
        let row_from = self.from / 8;
        let col_to = self.to % 8;
        let row_to = self.to / 8;

        let char_from_col = (col_from + b'a') as char;
        let char_from_row = (row_from + b'1') as char;
        let char_to_col = (col_to + b'a') as char;
        let char_to_row = (row_to + b'1') as char;

        let promotion_lit = if self.promotion != 0 {
            match self.promotion % 10 {
                4 => "q",
                2 => "n",
                3 => "b",
                1 => "r",
                _ => "",
            }
        } else {
            ""
        };

        format!(
            "{}{}{}{}{}",
            char_from_col, char_from_row, char_to_col, char_to_row, promotion_lit
        )
    }
}



#[derive(Debug, Copy, Clone)]
pub struct MoveInformation {
    pub castle_information: CastleInformation,
    pub hash: u64,
    pub en_passante: i8,
    pub capture: u8,
    pub moved_piece: u8,
}

impl MoveInformation {
    // Constructor
    pub fn new(castle_information: CastleInformation, hash: u64, en_passante: i8, capture: u8, moved_piece: u8) -> Self {
        MoveInformation {
            castle_information,
            hash,
            en_passante,
            capture,
            moved_piece,
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
    pub bitboards: [u64; 12],
    pub white_pieces: u64,
    pub black_pieces: u64,
    pub occupied: u64,
    pub white_possible_to_castle_long: bool,
    pub white_possible_to_castle_short: bool,
    pub black_possible_to_castle_long: bool,
    pub black_possible_to_castle_short: bool,
    pub field_for_en_passante: i8,  // -1 if no en passant possible, 0..63
    pub white_to_move: bool,
    pub move_count: i32,
    pub game_status: GameStatus,
    pub move_repetition_map: HashMap<u64, i32>,
    pub cached_hash: u64,
    pub _white_king_on_board: bool,
    pub _black_king_on_board: bool,
}

impl Board {
    // Constructor
    pub fn new(
        bitboards: [u64; 12],
        white_possible_to_castle_long: bool,
        white_possible_to_castle_short: bool,
        black_possible_to_castle_long: bool,
        black_possible_to_castle_short: bool,
        field_for_en_passante: i8,
        white_to_move: bool,
        move_count: i32,
        _white_king_on_board: bool,
        _black_king_on_board: bool,
    ) -> Self {
        let white_pieces = bitboards[WHITE_PAWN] | bitboards[WHITE_ROOK] | bitboards[WHITE_KNIGHT] |
                           bitboards[WHITE_BISHOP] | bitboards[WHITE_QUEEN] | bitboards[WHITE_KING];
        let black_pieces = bitboards[BLACK_PAWN] | bitboards[BLACK_ROOK] | bitboards[BLACK_KNIGHT] |
                           bitboards[BLACK_BISHOP] | bitboards[BLACK_QUEEN] | bitboards[BLACK_KING];
        let occupied = white_pieces | black_pieces;

        Board {
            bitboards,
            white_pieces,
            black_pieces,
            occupied,
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
            _white_king_on_board,
            _black_king_on_board,
        }
    }

    pub fn piece_to_bb_idx(piece: u8) -> usize {
        match piece {
            10 => WHITE_PAWN,
            11 => WHITE_ROOK,
            12 => WHITE_KNIGHT,
            13 => WHITE_BISHOP,
            14 => WHITE_QUEEN,
            15 => WHITE_KING,
            20 => BLACK_PAWN,
            21 => BLACK_ROOK,
            22 => BLACK_KNIGHT,
            23 => BLACK_BISHOP,
            24 => BLACK_QUEEN,
            25 => BLACK_KING,
            _ => panic!("Invalid piece ID: {}", piece),
        }
    }

    pub fn get_piece_at(&self, square: u8) -> u8 {
        let mask = 1u64 << square;
        if (self.occupied & mask) == 0 {
            return 0;
        }
        for i in 0..12 {
            if (self.bitboards[i] & mask) != 0 {
                return match i {
                    WHITE_PAWN => 10,
                    WHITE_ROOK => 11,
                    WHITE_KNIGHT => 12,
                    WHITE_BISHOP => 13,
                    WHITE_QUEEN => 14,
                    WHITE_KING => 15,
                    BLACK_PAWN => 20,
                    BLACK_ROOK => 21,
                    BLACK_KNIGHT => 22,
                    BLACK_BISHOP => 23,
                    BLACK_QUEEN => 24,
                    BLACK_KING => 25,
                    _ => 0,
                };
            }
        }
        0
    }

    /// return the index of kings (white_king, black_king)
    pub fn get_king_positions(&self) -> (i32, i32) {
        let white_king_pos = self.bitboards[WHITE_KING].trailing_zeros() as i32;
        let black_king_pos = self.bitboards[BLACK_KING].trailing_zeros() as i32;
        (
            if white_king_pos < 64 { white_king_pos } else { -1 },
            if black_king_pos < 64 { black_king_pos } else { -1 }
        )
    }

    /// It only panics if the from field is != 0
    /// calculate hash -> cached_hash
    pub fn do_move(&mut self, turn: &Turn) -> MoveInformation {
        let from = turn.from;
        let to = turn.to;
        let from_mask = 1u64 << from;
        let to_mask = 1u64 << to;

        let moved_piece = self.get_piece_at(from);
        if moved_piece == 0 {
            panic!("RIP do_move(): Field on turn.from is 0\n{:?}", turn);
        }

        let old_castle_information = self.get_castle_information();
        let old_field_for_en_passante = self.field_for_en_passante;

        let mut actual_capture = turn.capture;
        if actual_capture == 0 {
            let piece_at_to = self.get_piece_at(to);
            if piece_at_to != 0 && (piece_at_to >= 10 && piece_at_to <= 15) != self.white_to_move {
                actual_capture = piece_at_to;
            } else if (moved_piece == 10 || moved_piece == 20) && (to as i8 == old_field_for_en_passante) {
                actual_capture = if self.white_to_move { 20 } else { 10 };
            }
        }

        // handle king counter
        if actual_capture == 15 {
            self._white_king_on_board = false;
            self.game_status = GameStatus::BlackWin;
        } else if actual_capture == 25 {
            self._black_king_on_board = false;
            self.game_status = GameStatus::WhiteWin;
        }

        let moved_bb_idx = Board::piece_to_bb_idx(moved_piece);

        // Move the piece
        if turn.is_promotion() {
            self.bitboards[moved_bb_idx] ^= from_mask;
            let promo_bb_idx = Board::piece_to_bb_idx(turn.promotion);
            self.bitboards[promo_bb_idx] ^= to_mask;
        } else {
            self.bitboards[moved_bb_idx] ^= from_mask | to_mask;
        }

        // Handle capture
        if actual_capture != 0 {
            // Check if it was an en passant capture
            let is_en_passant = (moved_piece == 10 || moved_piece == 20) && (to as i8 == old_field_for_en_passante);
            if is_en_passant {
                if self.white_to_move {
                    let victim_sq = to - 8;
                    self.bitboards[BLACK_PAWN] &= !(1u64 << victim_sq);
                } else {
                    let victim_sq = to + 8;
                    self.bitboards[WHITE_PAWN] &= !(1u64 << victim_sq);
                }
            } else {
                let capture_bb_idx = Board::piece_to_bb_idx(actual_capture);
                self.bitboards[capture_bb_idx] &= !to_mask;
            }
        }

        // Handling castling for white and black (rook movements)
        if moved_piece == 15 || moved_piece == 25 {
            let is_castling = (to as i8 - from as i8).abs() == 2;
            if is_castling {
                match to {
                    6 => { // White short
                        self.bitboards[WHITE_ROOK] ^= (1u64 << 7) | (1u64 << 5);
                    }
                    2 => { // White long
                        self.bitboards[WHITE_ROOK] ^= (1u64 << 0) | (1u64 << 3);
                    }
                    62 => { // Black short
                        self.bitboards[BLACK_ROOK] ^= (1u64 << 63) | (1u64 << 61);
                    }
                    58 => { // Black long
                        self.bitboards[BLACK_ROOK] ^= (1u64 << 56) | (1u64 << 59);
                    }
                    _ => {}
                }
            }
        }

        // Update castling rights
        match from {
            56 => self.black_possible_to_castle_long = false,
            63 => self.black_possible_to_castle_short = false,
            60 => {
                self.black_possible_to_castle_long = false;
                self.black_possible_to_castle_short = false;
            }
            0 => self.white_possible_to_castle_long = false,
            7 => self.white_possible_to_castle_short = false,
            4 => {
                self.white_possible_to_castle_long = false;
                self.white_possible_to_castle_short = false;
            }
            _ => {}
        }
        match to {
            56 => self.black_possible_to_castle_long = false,
            63 => self.black_possible_to_castle_short = false,
            0 => self.white_possible_to_castle_long = false,
            7 => self.white_possible_to_castle_short = false,
            _ => {}
        }

        // Handling en passante target square
        self.field_for_en_passante = -1;
        if moved_piece == 10 && from / 8 == 1 && to / 8 == 3 {
            self.field_for_en_passante = (from + 8) as i8;
        } else if moved_piece == 20 && from / 8 == 6 && to / 8 == 4 {
            self.field_for_en_passante = (from - 8) as i8;
        }

        // Increment move count if it's black's turn
        if !self.white_to_move {
            self.move_count += 1;
        }
        self.white_to_move = !self.white_to_move;

        // Recalculate occupied bitboards
        self.white_pieces = self.bitboards[WHITE_PAWN] | self.bitboards[WHITE_ROOK] | self.bitboards[WHITE_KNIGHT] |
                           self.bitboards[WHITE_BISHOP] | self.bitboards[WHITE_QUEEN] | self.bitboards[WHITE_KING];
        self.black_pieces = self.bitboards[BLACK_PAWN] | self.bitboards[BLACK_ROOK] | self.bitboards[BLACK_KNIGHT] |
                           self.bitboards[BLACK_BISHOP] | self.bitboards[BLACK_QUEEN] | self.bitboards[BLACK_KING];
        self.occupied = self.white_pieces | self.black_pieces;

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
            if count > 3 { panic!("RIP move_repetition_map value {}", count) }
            if count == 3 {
                self.game_status = GameStatus::Draw;
            }
        }
        MoveInformation::new(old_castle_information, self.cached_hash, old_field_for_en_passante, actual_capture, moved_piece)
    }


    pub fn undo_move(&mut self, turn: &Turn, move_information: MoveInformation) {
        self.cached_hash = 0;

        let from = turn.from;
        let to = turn.to;
        let from_mask = 1u64 << from;
        let to_mask = 1u64 << to;

        // handle king counter
        if move_information.capture == 15 {
            self._white_king_on_board = true;
        } else if move_information.capture == 25 {
            self._black_king_on_board = true;
        }

        self.game_status = GameStatus::Normal;
        // assert!(self._white_king_on_board && self._black_king_on_board, "RIP at least one King missing on the board");

        let castle_information = move_information.castle_information;

        // Find the moved piece
        let moved_piece = move_information.moved_piece;

        let moved_bb_idx = Board::piece_to_bb_idx(moved_piece);

        // Undo move
        if turn.is_promotion() {
            let promo_bb_idx = Board::piece_to_bb_idx(turn.promotion);
            self.bitboards[promo_bb_idx] ^= to_mask;
            self.bitboards[moved_bb_idx] ^= from_mask;
        } else {
            self.bitboards[moved_bb_idx] ^= from_mask | to_mask;
        }

        // Handle capture undo
        if move_information.capture != 0 {
            let is_en_passant = (moved_piece == 10 || moved_piece == 20) && (to as i8 == move_information.en_passante);
            if is_en_passant {
                if !self.white_to_move { // White played the EP capture
                    let victim_sq = to - 8;
                    self.bitboards[BLACK_PAWN] |= 1u64 << victim_sq;
                } else { // Black played the EP capture
                    let victim_sq = to + 8;
                    self.bitboards[WHITE_PAWN] |= 1u64 << victim_sq;
                }
            } else {
                let capture_bb_idx = Board::piece_to_bb_idx(move_information.capture);
                self.bitboards[capture_bb_idx] |= to_mask;
            }
        }

        // Handle castling undo (rook movements)
        if moved_piece == 15 || moved_piece == 25 {
            let is_castling = (to as i8 - from as i8).abs() == 2;
            if is_castling {
                match to {
                    6 => { // White short
                        self.bitboards[WHITE_ROOK] ^= (1u64 << 7) | (1u64 << 5);
                    }
                    2 => { // White long
                        self.bitboards[WHITE_ROOK] ^= (1u64 << 0) | (1u64 << 3);
                    }
                    62 => { // Black short
                        self.bitboards[BLACK_ROOK] ^= (1u64 << 63) | (1u64 << 61);
                    }
                    58 => { // Black long
                        self.bitboards[BLACK_ROOK] ^= (1u64 << 56) | (1u64 << 59);
                    }
                    _ => {}
                }
            }
        }

        // Restore castling rights and en passante information
        self.white_possible_to_castle_long = castle_information.white_possible_to_castle_long;
        self.white_possible_to_castle_short = castle_information.white_possible_to_castle_short;
        self.black_possible_to_castle_long = castle_information.black_possible_to_castle_long;
        self.black_possible_to_castle_short = castle_information.black_possible_to_castle_short;
        self.field_for_en_passante = move_information.en_passante;

        // Decrement move count if it was white's move (meaning we are undoing black's move, so white_to_move will become true)
        if self.white_to_move {
            self.move_count -= 1;
        }
        self.white_to_move = !self.white_to_move;

        // Recalculate occupied bitboards
        self.white_pieces = self.bitboards[WHITE_PAWN] | self.bitboards[WHITE_ROOK] | self.bitboards[WHITE_KNIGHT] |
                           self.bitboards[WHITE_BISHOP] | self.bitboards[WHITE_QUEEN] | self.bitboards[WHITE_KING];
        self.black_pieces = self.bitboards[BLACK_PAWN] | self.bitboards[BLACK_ROOK] | self.bitboards[BLACK_KNIGHT] |
                           self.bitboards[BLACK_BISHOP] | self.bitboards[BLACK_QUEEN] | self.bitboards[BLACK_KING];
        self.occupied = self.white_pieces | self.black_pieces;

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
    pub fn _calculate_complexity(&self) -> i32 {
        let mut complexity = 0;
        for i in 0..12 {
            let count = self.bitboards[i].count_ones() as i32;
            complexity += count * match i {
                WHITE_PAWN | BLACK_PAWN => 1,
                WHITE_ROOK | BLACK_ROOK => 6,
                WHITE_KNIGHT | BLACK_KNIGHT => 3,
                WHITE_BISHOP | BLACK_BISHOP => 6,
                WHITE_QUEEN | BLACK_QUEEN => 12,
                _ => 0,
            };
        }
        complexity
    }

    /// Zobrist-Hash function for the board (used for 3-move repetition and Zobrist-Hash Table)
    pub fn hash(&self) -> u64 {
        zobrist::gen(&self)
    }

    pub fn _get_piece_idx(&self) -> Vec<usize> {
        let mut piece_idx = Vec::with_capacity(32);
        let mut temp = self.occupied;
        while temp != 0 {
            let square = temp.trailing_zeros() as usize;
            piece_idx.push(square);
            temp &= temp - 1;
        }
        piece_idx
    }
}

// Implement `PartialEq` manually for the `Board` struct, for unittests
impl PartialEq for Board {
    fn eq(&self, other: &Self) -> bool {
        self.white_possible_to_castle_long == other.white_possible_to_castle_long &&
            self.white_possible_to_castle_short == other.white_possible_to_castle_short &&
            self.black_possible_to_castle_long == other.black_possible_to_castle_long &&
            self.black_possible_to_castle_short == other.black_possible_to_castle_short &&
            self.field_for_en_passante == other.field_for_en_passante &&
            self.white_to_move == other.white_to_move &&
            self.move_count == other.move_count &&
            self.game_status == other.game_status &&
            self.bitboards == other.bitboards &&
            self.move_repetition_map == other.move_repetition_map
    }
}

#[derive(Debug, Default, Clone)]
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

    pub fn calculate(&mut self) -> &mut Self {
        if self.created_nodes > 0 {
            self.cuts = 100 - (self.calculated_nodes * 100 / self.created_nodes) as i32;
            self.capture_share = (self.created_capture_node * 100 / self.created_nodes) as i32;
        } else {
            self.cuts = 0;
            self.capture_share = 0;
        }
        self.nodes_per_ms = (self.created_nodes / (self.calc_time_ms + 1)) as i32;
        self.zobrist_hit = self.zobrist_hit * 100 / (self.eval_nodes + 1);
        self
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

    pub fn _reset_stats(&mut self) {
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

#[derive(Default, Clone)]
pub struct SearchResult {
    pub variants: Vec<Variant>,
    pub is_white_move: bool,
    pub stats: Stats,
    pub completed: bool,
    pub calculated_depth: i32,
    pub is_pv_search_result: bool,
}

#[derive(Debug, Clone)]
pub struct Variant {
    pub eval: i16,
    pub best_move: Option<Turn>,
    pub move_row: VecDeque<Option<Turn>>,
}

impl SearchResult {

    pub fn _new() -> Self {
        SearchResult{
            variants: Vec::default(),
            is_white_move: true,     
            stats: Stats::default(),   
            completed: true,
            calculated_depth: 0,
            is_pv_search_result: false,
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

    pub fn _print_debug(&self) {
        if let Some(variant) = self.variants.get(0) {
            print!("{:?}", variant);
        } else {
            println!("No variants available");
        }
    }

    pub fn _print_best_variant(&self) {
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

    pub fn _print_all_variants(&self) {
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

    pub fn get_pv_move_row(&self) -> Vec<Turn> {
        self.variants
            .get(0)
            .expect("RIP Found no PV move row")
            .move_row
            .iter()
            .map(|turn_option| {
                turn_option.clone().expect("RIP no turn in move row")
            })
            .collect()
    }

}




#[cfg(test)]
mod tests {
    use crate::notation_util::NotationUtil;
    use crate::service::Service;
    use super::UciGame;


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
        assert_eq!(20, board.field_for_en_passante);
        assert_eq!(-1, mi1.en_passante);

        let turn2 = NotationUtil::get_turn_from_notation("e7e5");
        let mi2 = board.do_move(&turn2);
        assert_eq!(44, board.field_for_en_passante);
        assert_eq!(20, mi2.en_passante);

        let turn3 = NotationUtil::get_turn_from_notation("d7d6");
        let mi3 = board.do_move(&turn3);
        assert_eq!(-1, board.field_for_en_passante);
        assert_eq!(44, mi3.en_passante);

        board.undo_move(&turn3, mi3);
        assert_eq!(44, board.field_for_en_passante);

        board.undo_move(&turn2, mi2);
        assert_eq!(20, board.field_for_en_passante);

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
        assert_eq!(46, mi.en_passante);
        assert_eq!(0, board.get_piece_at(38));

        board.undo_move(&turn, mi);
        assert_eq!(0, board.get_piece_at(46));
        assert_eq!(20, board.get_piece_at(38));
        assert_eq!(10, board.get_piece_at(39));

        // for black
        let mut board = fen_service.set_fen("rnbqkbnr/ppp1pppp/8/8/P1Pp4/8/1P1PPPPP/RNBQKBNR b KQkq c3 0 3");
        let mut turn = NotationUtil::get_turn_from_notation("d4c3");
        turn.capture = 10;
        let mi = board.do_move(&turn);
        assert_eq!(18, mi.en_passante);
        assert_eq!(0, board.get_piece_at(26));

        board.undo_move(&turn, mi);
        assert_eq!(0, board.get_piece_at(18));
        assert_eq!(10, board.get_piece_at(26));
        assert_eq!(20, board.get_piece_at(27));    
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
        assert_eq!(board.get_piece_at(6), 15);
        assert_eq!(board.get_piece_at(5), 11);
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
        assert_eq!(board.get_piece_at(4), 15);
        assert_eq!(board.get_piece_at(7), 11);
        assert_eq!(board, init_board);

        // White long castle
        let mi2 = board.do_move(castle_white_long);
        assert_eq!(board.get_piece_at(2), 15);
        assert_eq!(board.get_piece_at(3), 11);
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
        assert_eq!(board.get_piece_at(4), 15);
        assert_eq!(board.get_piece_at(7), 11);
        assert_eq!(board, init_board);

        // Black short castle
        let mi3 = board.do_move(castle_black_short);
        assert_eq!(board.get_piece_at(62), 25);
        assert_eq!(board.get_piece_at(61), 21);
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
        assert_eq!(board.get_piece_at(60), 25);
        assert_eq!(board.get_piece_at(63), 21);
        assert_eq!(board, init_board);

        // Black long castle
        let mi4 = board.do_move(castle_black_long);
        assert_eq!(board.get_piece_at(58), 25);
        assert_eq!(board.get_piece_at(59), 21);
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
        assert_eq!(board.get_piece_at(60), 25);
        assert_eq!(board.get_piece_at(63), 21);
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
        assert_eq!(20, game.board.get_piece_at(18));

        
        game.board.undo_move(&capture_move, mi);
        assert_eq!(12, game.board.get_piece_at(18));
    }


    #[test]
    fn calculate_complexity_test() {
        let fen_service = Service::new().fen;

        let board = fen_service.set_init_board();
        assert_eq!(100, board._calculate_complexity());

        // midgame. Queen + 3 light pieces each + 2 rook
        let board = fen_service.set_fen("r2q1rk1/ppp2ppp/2n5/3p1b2/3Pn3/2PB1N2/P1Q2PPP/R1B2RK1 w - - 4 12");
        assert_eq!(88, board._calculate_complexity());

        // late midgame. Queen + 1 light pieces each + 1 rook
        let board = fen_service.set_fen("3q1rk1/Q1p2pp1/3n2p1/3p4/3P1B2/2P5/P4PPP/5RK1 b - - 0 20");
        assert_eq!(56, board._calculate_complexity());

        // rook endgame + 1 light peace each
        let board = fen_service.set_fen("r5k1/2B2pp1/6p1/3p4/3P4/2n5/P4PPP/R5K1 w - - 2 25");
        assert_eq!(30, board._calculate_complexity());
    }
}