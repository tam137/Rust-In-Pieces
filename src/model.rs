use std::sync::atomic::AtomicBool;
use std::collections::{HashMap, VecDeque};

use crate::zobrist;
use crate::{notation_util::NotationUtil, zobrist::ZobristTable};

pub const INIT_BOARD_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

pub const RIP_COULDN_LOCK_MUTEX: &str = "RIP Could not lock mutex";

pub const RIP_COULDN_SEND_TO_GAME_CMD_QUEUE: &str = "RIP Could not Send commands to game command queue";
pub const RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE: &str = "RIP Could not Send msg to log buffer queue";
pub const RIP_COULDN_JOIN_THREAD: &str = "RIP Could not join thread";

pub type LoggerFn = std::sync::Arc<dyn Fn(String) + Send + Sync>;

pub struct EngineState {
    pub stop_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub debug_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub zobrist_table: std::sync::RwLock<std::sync::Arc<ZobristTable>>,
    pub pv_nodes: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<u64, Turn>>>,
    pub pv_nodes_len: std::sync::Arc<std::sync::atomic::AtomicI32>,
    pub logger: std::sync::Arc<std::sync::RwLock<LoggerFn>>,
    pub log_sender: std::sync::mpsc::Sender<String>,
}

pub struct SearchContext<'a> {
    pub zobrist_table: &'a ZobristTable,
    pub stop_flag: &'a AtomicBool,
    pub pv_nodes: &'a std::sync::Mutex<std::collections::HashMap<u64, Turn>>,
    pub killer_moves: [Option<Turn>; 2],
    pub history_table: *const [[u32; 64]; 64],
    pub counter_move: Option<Turn>,
    pub start_time: std::time::Instant,
    pub target_time: Option<i32>,
    pub root_moves_total: i32,
    pub root_moves_searched: i32,
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

#[derive(Debug, Clone, Copy)]
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

    pub fn to_algebraic(self) -> String {
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


#[derive(Clone, Copy)]
pub struct MoveList {
    pub moves: [Turn; 256],
    pub len: usize,
}

impl MoveList {
    pub fn new() -> Self {
        Self {
            moves: [Turn {
                from: 0,
                to: 0,
                capture: 0,
                promotion: 0,
                gives_check: false,
                eval: 0,
                hash: 0,
                has_hashed_eval: false,
                rank: 0,
            }; 256],
            len: 0,
        }
    }

    pub fn push(&mut self, turn: Turn) {
        if self.len < 256 {
            self.moves[self.len] = turn;
            self.len += 1;
        }
    }

    pub fn as_slice(&self) -> &[Turn] {
        &self.moves[0..self.len]
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

#[derive(Clone, Copy)]
pub struct MoveRawList {
    pub moves: [u8; 256],
    pub len: usize,
}

impl MoveRawList {
    pub fn new() -> Self {
        Self {
            moves: [0u8; 256],
            len: 0,
        }
    }

    pub fn push(&mut self, val: i32) {
        if self.len < 256 {
            self.moves[self.len] = val as u8;
            self.len += 1;
        }
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}




#[derive(Debug, Copy, Clone)]
pub struct MoveInformation {
    pub castle_information: CastleInformation,
    pub hash: u64,
    pub en_passante: i8,
    pub capture: u8,
    pub moved_piece: u8,
    pub old_pst_mg: i16,
    pub old_pst_eg: i16,
}

impl MoveInformation {
    // Constructor
    pub fn new(castle_information: CastleInformation, hash: u64, en_passante: i8, capture: u8, moved_piece: u8, old_pst_mg: i16, old_pst_eg: i16) -> Self {
        MoveInformation {
            castle_information,
            hash,
            en_passante,
            capture,
            moved_piece,
            old_pst_mg,
            old_pst_eg,
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
    pub mailbox: [u8; 64],
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
    pub pst_mg: i16,
    pub pst_eg: i16,
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

        let mut mailbox = [0u8; 64];
        for (i, &bb_val) in bitboards.iter().enumerate() {
            let mut bb = bb_val;
            let piece = match i {
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
            while bb != 0 {
                let square = bb.trailing_zeros() as usize;
                mailbox[square] = piece;
                bb &= bb - 1;
            }
        }

        let mut pst_mg: i16 = 0;
        let mut pst_eg: i16 = 0;
        for (i, &bb_val) in bitboards.iter().enumerate() {
            let mut bb = bb_val;
            while bb != 0 {
                let square = bb.trailing_zeros() as usize;
                pst_mg += crate::pst::PST_MG[i][square];
                pst_eg += crate::pst::PST_EG[i][square];
                bb &= bb - 1;
            }
        }

        Board {
            bitboards,
            mailbox,
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
            pst_mg,
            pst_eg,
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

    #[inline(always)]
    pub fn get_piece_at(&self, square: u8) -> u8 {
        self.mailbox[square as usize]
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
        let old_pst_mg = self.pst_mg;
        let old_pst_eg = self.pst_eg;

        let mut actual_capture = turn.capture;
        if actual_capture == 0 {
            let piece_at_to = self.get_piece_at(to);
            if piece_at_to != 0 && (10..=15).contains(&piece_at_to) != self.white_to_move {
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

        // Incremental PST updates
        self.pst_mg -= crate::pst::PST_MG[moved_bb_idx][from as usize];
        self.pst_eg -= crate::pst::PST_EG[moved_bb_idx][from as usize];

        if turn.is_promotion() {
            let promo_bb_idx = Board::piece_to_bb_idx(turn.promotion);
            self.pst_mg += crate::pst::PST_MG[promo_bb_idx][to as usize];
            self.pst_eg += crate::pst::PST_EG[promo_bb_idx][to as usize];
        } else {
            self.pst_mg += crate::pst::PST_MG[moved_bb_idx][to as usize];
            self.pst_eg += crate::pst::PST_EG[moved_bb_idx][to as usize];
        }

        // Update mailbox
        self.mailbox[from as usize] = 0;
        if turn.is_promotion() {
            self.mailbox[to as usize] = turn.promotion;
        } else {
            self.mailbox[to as usize] = moved_piece;
        }

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
            let capture_bb_idx = Board::piece_to_bb_idx(actual_capture);
            if is_en_passant {
                let victim_sq = if self.white_to_move { to - 8 } else { to + 8 };
                self.pst_mg -= crate::pst::PST_MG[capture_bb_idx][victim_sq as usize];
                self.pst_eg -= crate::pst::PST_EG[capture_bb_idx][victim_sq as usize];
                
                if self.white_to_move {
                    self.bitboards[BLACK_PAWN] &= !(1u64 << victim_sq);
                    self.mailbox[victim_sq as usize] = 0;
                } else {
                    self.bitboards[WHITE_PAWN] &= !(1u64 << victim_sq);
                    self.mailbox[victim_sq as usize] = 0;
                }
            } else {
                self.pst_mg -= crate::pst::PST_MG[capture_bb_idx][to as usize];
                self.pst_eg -= crate::pst::PST_EG[capture_bb_idx][to as usize];
                self.bitboards[capture_bb_idx] &= !to_mask;
            }
        }

        // Handling castling for white and black (rook movements)
        if moved_piece == 15 || moved_piece == 25 {
            let is_castling = (to as i8 - from as i8).abs() == 2;
            if is_castling {
                match to {
                    6 => { // White short
                        self.pst_mg -= crate::pst::PST_MG[WHITE_ROOK][7];
                        self.pst_eg -= crate::pst::PST_EG[WHITE_ROOK][7];
                        self.pst_mg += crate::pst::PST_MG[WHITE_ROOK][5];
                        self.pst_eg += crate::pst::PST_EG[WHITE_ROOK][5];
                        self.bitboards[WHITE_ROOK] ^= (1u64 << 7) | (1u64 << 5);
                        self.mailbox[7] = 0;
                        self.mailbox[5] = 11;
                    }
                    2 => { // White long
                        self.pst_mg -= crate::pst::PST_MG[WHITE_ROOK][0];
                        self.pst_eg -= crate::pst::PST_EG[WHITE_ROOK][0];
                        self.pst_mg += crate::pst::PST_MG[WHITE_ROOK][3];
                        self.pst_eg += crate::pst::PST_EG[WHITE_ROOK][3];
                        self.bitboards[WHITE_ROOK] ^= (1u64 << 0) | (1u64 << 3);
                        self.mailbox[0] = 0;
                        self.mailbox[3] = 11;
                    }
                    62 => { // Black short
                        self.pst_mg -= crate::pst::PST_MG[BLACK_ROOK][63];
                        self.pst_eg -= crate::pst::PST_EG[BLACK_ROOK][63];
                        self.pst_mg += crate::pst::PST_MG[BLACK_ROOK][61];
                        self.pst_eg += crate::pst::PST_EG[BLACK_ROOK][61];
                        self.bitboards[BLACK_ROOK] ^= (1u64 << 63) | (1u64 << 61);
                        self.mailbox[63] = 0;
                        self.mailbox[61] = 21;
                    }
                    58 => { // Black long
                        self.pst_mg -= crate::pst::PST_MG[BLACK_ROOK][56];
                        self.pst_eg -= crate::pst::PST_EG[BLACK_ROOK][56];
                        self.pst_mg += crate::pst::PST_MG[BLACK_ROOK][59];
                        self.pst_eg += crate::pst::PST_EG[BLACK_ROOK][59];
                        self.bitboards[BLACK_ROOK] ^= (1u64 << 56) | (1u64 << 59);
                        self.mailbox[56] = 0;
                        self.mailbox[59] = 21;
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
        MoveInformation::new(old_castle_information, self.cached_hash, old_field_for_en_passante, actual_capture, moved_piece, old_pst_mg, old_pst_eg)
    }


    pub fn undo_move(&mut self, turn: &Turn, move_information: MoveInformation) {
        self.cached_hash = 0;
        self.pst_mg = move_information.old_pst_mg;
        self.pst_eg = move_information.old_pst_eg;

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

        // Update mailbox
        self.mailbox[from as usize] = moved_piece;
        self.mailbox[to as usize] = 0;

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
                    self.mailbox[victim_sq as usize] = move_information.capture;
                } else { // Black played the EP capture
                    let victim_sq = to + 8;
                    self.bitboards[WHITE_PAWN] |= 1u64 << victim_sq;
                    self.mailbox[victim_sq as usize] = move_information.capture;
                }
            } else {
                let capture_bb_idx = Board::piece_to_bb_idx(move_information.capture);
                self.bitboards[capture_bb_idx] |= to_mask;
                self.mailbox[to as usize] = move_information.capture;
            }
        }

        // Handle castling undo (rook movements)
        if moved_piece == 15 || moved_piece == 25 {
            let is_castling = (to as i8 - from as i8).abs() == 2;
            if is_castling {
                match to {
                    6 => { // White short
                        self.bitboards[WHITE_ROOK] ^= (1u64 << 7) | (1u64 << 5);
                        self.mailbox[7] = 11;
                        self.mailbox[5] = 0;
                    }
                    2 => { // White long
                        self.bitboards[WHITE_ROOK] ^= (1u64 << 0) | (1u64 << 3);
                        self.mailbox[0] = 11;
                        self.mailbox[3] = 0;
                    }
                    62 => { // Black short
                        self.bitboards[BLACK_ROOK] ^= (1u64 << 63) | (1u64 << 61);
                        self.mailbox[63] = 21;
                        self.mailbox[61] = 0;
                    }
                    58 => { // Black long
                        self.bitboards[BLACK_ROOK] ^= (1u64 << 56) | (1u64 << 59);
                        self.mailbox[56] = 21;
                        self.mailbox[59] = 0;
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
        zobrist::gen_hash(self)
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
    pub best_score: i16,
    pub second_best_score: i16,
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
            best_score: 0,
            second_best_score: 0,
        }
    }

    pub fn add_variant(&mut self, variant: Variant) {
        self.variants.push(variant);
    }

    pub fn get_eval(&self) -> i16 {
        if let Some(variant) = self.variants.first() {
            variant.eval
        } else {
            0
        }
    }

    pub fn get_depth(&self) -> i32 {
        if let Some(variant) = self.variants.first() {
            variant.move_row.len() as i32
        } else {
            0
        }
    }

    pub fn _print_debug(&self) {
        if let Some(variant) = self.variants.first() {
            print!("{:?}", variant);
        } else {
            println!("No variants available");
        }
    }

    pub fn _print_best_variant(&self) {
        if let Some(variant) = self.variants.first() {
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
        self.variants.first()
            .and_then(|variant| variant.best_move.as_ref())
            .map(|best_move| best_move.to_algebraic())
            .unwrap_or_else(|| "0000".to_string())
    }

    pub fn get_best_move_row(&self) -> String {
        if let Some(variant) = self.variants.first() {
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
        if let Some(variant) = self.variants.first() {
            variant.move_row
                .iter()
                .filter_map(|turn_option| *turn_option)
                .collect()
        } else {
            Vec::new()
        }
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

    #[test]
    fn move_list_capacity_and_safety_test() {
        use super::MoveList;
        use super::Turn;

        let mut list = MoveList::new();
        assert!(list.is_empty());
        assert_eq!(list.len, 0);

        let turn = Turn::new(10, 20, 0, 0, false, 0);
        list.push(turn);
        assert!(!list.is_empty());
        assert_eq!(list.len, 1);
        assert_eq!(list.as_slice()[0].from, 10);

        // Fill up list beyond 256
        for i in 1..300 {
            list.push(Turn::new((i % 64) as u8, 20, 0, 0, false, 0));
        }

        assert_eq!(list.len, 256, "MoveList should cap at 256 turns and not overflow");
        
        // Pushing to full list should not panic
        list.push(Turn::new(1, 1, 0, 0, false, 0));
        assert_eq!(list.len, 256);
    }

    #[test]
    fn move_raw_list_capacity_and_safety_test() {
        use super::MoveRawList;

        let mut list = MoveRawList::new();
        assert!(list.is_empty());
        assert_eq!(list.len, 0);

        list.push(42);
        assert!(!list.is_empty());
        assert_eq!(list.len, 1);
        assert_eq!(list.moves[0], 42);

        // Fill up list beyond 256
        for i in 1..300 {
            list.push(i as i32);
        }

        assert_eq!(list.len, 256, "MoveRawList should cap at 256 elements and not overflow");
        
        // Pushing to full list should not panic
        list.push(99);
        assert_eq!(list.len, 256);
    }

    #[test]
    fn mailbox_synchronicity_test() {
        use super::*;
        let fen_service = Service::new().fen;

        let verify_mailbox = |board: &Board| {
            for sq in 0..64 {
                let mut expected_piece = 0;
                let mask = 1u64 << sq;
                for i in 0..12 {
                    if (board.bitboards[i] & mask) != 0 {
                        expected_piece = match i {
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
                        break;
                    }
                }
                assert_eq!(
                    board.mailbox[sq],
                    expected_piece,
                    "Mailbox out of sync with bitboards at square {}",
                    sq
                );
            }
        };

        // 1. Verify initial board
        let mut board = fen_service.set_init_board();
        verify_mailbox(&board);

        // 2. Verify KiwiPete board
        let board_kp = fen_service.set_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
        verify_mailbox(&board_kp);

        // 3. Verify mailbox updates after normal move and undo
        let turn_normal = Turn::new(12, 28, 0, 0, false, 0); // e2e4
        let mi = board.do_move(&turn_normal);
        verify_mailbox(&board);
        assert_eq!(board.mailbox[12], 0);
        assert_eq!(board.mailbox[28], 10);

        board.undo_move(&turn_normal, mi);
        verify_mailbox(&board);
        assert_eq!(board.mailbox[12], 10);
        assert_eq!(board.mailbox[28], 0);

        // 4. Verify castling short move and undo
        let mut board_castle = fen_service.set_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1");
        board_castle.bitboards[WHITE_KNIGHT] &= !(1u64 << 6);
        board_castle.bitboards[WHITE_BISHOP] &= !(1u64 << 5);
        board_castle.mailbox[6] = 0;
        board_castle.mailbox[5] = 0;
        board_castle.occupied = (board_castle.white_pieces | board_castle.black_pieces) & !((1u64 << 6) | (1u64 << 5));
        verify_mailbox(&board_castle);

        let turn_castle = Turn::new(4, 6, 0, 0, false, 0); // O-O
        let mi_c = board_castle.do_move(&turn_castle);
        verify_mailbox(&board_castle);
        assert_eq!(board_castle.mailbox[4], 0);
        assert_eq!(board_castle.mailbox[6], 15);
        assert_eq!(board_castle.mailbox[7], 0);
        assert_eq!(board_castle.mailbox[5], 11);

        board_castle.undo_move(&turn_castle, mi_c);
        verify_mailbox(&board_castle);
        assert_eq!(board_castle.mailbox[4], 15);
        assert_eq!(board_castle.mailbox[6], 0);
        assert_eq!(board_castle.mailbox[7], 11);
        assert_eq!(board_castle.mailbox[5], 0);

        // 5. Verify en passant capture and undo
        let mut board_ep = fen_service.set_fen("rnbqkbnr/ppp1pp1p/6p1/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3");
        verify_mailbox(&board_ep);
        
        let turn_ep = Turn::new(36, 43, 20, 0, false, 0); // e5d6 e.p.
        let mi_ep = board_ep.do_move(&turn_ep);
        verify_mailbox(&board_ep);
        assert_eq!(board_ep.mailbox[36], 0);
        assert_eq!(board_ep.mailbox[43], 10);
        assert_eq!(board_ep.mailbox[35], 0);

        board_ep.undo_move(&turn_ep, mi_ep);
        verify_mailbox(&board_ep);
        assert_eq!(board_ep.mailbox[36], 10);
        assert_eq!(board_ep.mailbox[43], 0);
        assert_eq!(board_ep.mailbox[35], 20);
    }
}