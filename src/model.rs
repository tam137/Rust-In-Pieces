use std::sync::atomic::AtomicBool;
use std::collections::VecDeque;

use crate::zobrist;
use crate::{notation_util::NotationUtil, zobrist::ZobristTable};

pub const INIT_BOARD_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

pub const RIP_COULDN_LOCK_MUTEX: &str = "RIP Could not lock mutex";

pub const RIP_COULDN_SEND_TO_GAME_CMD_QUEUE: &str = "RIP Could not Send commands to game command queue";
pub const RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE: &str = "RIP Could not Send msg to log buffer queue";
pub const RIP_COULDN_JOIN_THREAD: &str = "RIP Could not join thread";

/// Score of a checkmate delivered on ply 0. The search returns `MATE_SCORE - ply`
/// for a win and `-(MATE_SCORE - ply)` for a loss, so the distance to mate in plies
/// is recovered as `MATE_SCORE - score.abs()`.
/// Capacity of the position history. A long game plus the deepest search ply fits comfortably;
/// beyond it repetition detection degrades gracefully instead of panicking.
pub const MAX_HISTORY_PLIES: usize = 2048;

pub const MATE_SCORE: i16 = i16::MAX - 1;

/// Scores at or beyond this magnitude are mate scores rather than centipawn
/// evaluations. The margin below `MATE_SCORE` covers the deepest reachable ply.
pub const MATE_SCORE_THRESHOLD: i16 = 30000;

pub type LoggerFn = std::sync::Arc<dyn Fn(String) + Send + Sync>;

/// The three tables the search learns while it plays: the killer moves, the butterfly
/// history and the counter moves.
///
/// They used to be allocated inside `SearchService::get_moves`, which the iterative deepening
/// loop in `game_handler.rs` calls once *per depth*. Every one of them was therefore thrown
/// away at every iteration, and the depth-8 search started with an empty history -- worst
/// precisely at the deep iterations that matter most. `task.md` 23.1.
///
/// They now live for the whole game and are cleared on `ucinewgame`, which is the published
/// discipline. `age` halves the history at the entry to each search so that entries which
/// stopped earning cutoffs decay, rather than holding their rank until the global overflow
/// pass in `search_service.rs` happens to fire.
/// The three tables are boxed individually rather than held inline. Together they are about
/// 50 KB, and an inline struct that size is built as a stack temporary at every `EngineState`
/// literal. A debug build does not reuse those slots, so the temporary stays in the frame of
/// whatever function constructed the state -- and `test_qs_tt_search_consistency_and_node_reduction`
/// constructs one and then recurses, which overflowed the 2 MB test stack. Boxed, `SearchTables`
/// is three pointers and only one table is transiently on the stack at a time. The boxes are
/// dereferenced once per search, not per node, so the search path pays nothing for the
/// indirection.
pub struct SearchTables {
    pub killer_moves: Box<[[Option<Turn>; 2]; 128]>,
    /// The butterfly history, `[side][from][to]` (`task.md` 23.2).
    ///
    /// **Index 0 is White, index 1 is Black — [`crate::model::history_side`] is the only place
    /// that mapping is written down.** A quiet move is credited to the side that played it and
    /// read by the side whose moves are being ranked, which is the same side one ply apart: the
    /// cutoff writes for the node's own side to move, and the child's move generation reads for
    /// the side it is generating for.
    ///
    /// Sharing one `[from][to]` plane between the two sides, which is what this replaces, meant a
    /// quiet move that refuted for one side raised the rank of the geometrically identical move
    /// for the other.
    ///
    /// The entries are signed and bounded by [`MAX_HISTORY`] (`task.md` 23.3). Every write goes
    /// through [`history_gravity`], so an entry can never leave that range and there is no
    /// rescaling pass.
    pub history_table: Box<[[[i32; 64]; 64]; 2]>,
    pub counter_moves: Box<[[Option<Turn>; 64]; 64]>,
}

impl SearchTables {
    pub fn new() -> Self {
        SearchTables {
            killer_moves: Box::new([[None; 2]; 128]),
            history_table: Box::new([[[0i32; 64]; 64]; 2]),
            counter_moves: Box::new([[None; 64]; 64]),
        }
    }

    /// Clears every table. Called on `ucinewgame`: nothing learned about the previous game
    /// carries into the next one.
    pub fn reset(&mut self) {
        *self.killer_moves = [[None; 2]; 128];
        *self.history_table = [[[0i32; 64]; 64]; 2];
        *self.counter_moves = [[None; 64]; 64];
    }

    /// Halves the history on entry to `get_moves`.
    ///
    /// Note what that means: the iterative deepening loop calls `get_moves` once per depth, so
    /// this runs once per *iteration*, not once per `go`. A depth-10 search therefore halves
    /// nine times, and what an early iteration learned is worth 2^-8 of a late one by the end.
    /// The table still carries ordering from one iteration into the next, which is the point of
    /// `task.md` 23.1; halving once per `go` instead is an untested variant.
    ///
    /// Killers and counter moves are deliberately not aged: both are overwritten wholesale by
    /// the next cutoff at the same ply or from the same parent move, so a stale entry costs one
    /// ordering slot. A stale history entry instead biases the statistic that the Late Move
    /// Reduction, the quiet-move ranking and `lmr_history_bad_threshold` all read.
    ///
    /// Since `task.md` 23.3 the entries are signed, and integer division truncates towards zero,
    /// so a refuted move decays towards 0 at the same rate a good one does. This decay sits on
    /// top of the gravity in [`crate::model::history_gravity`], which is a second one; whether
    /// both are wanted is an open question and its own run, not part of 23.3.
    pub fn age(&mut self) {
        for side in self.history_table.iter_mut() {
            for row in side.iter_mut() {
                for entry in row.iter_mut() {
                    *entry /= 2;
                }
            }
        }
    }
}

impl Default for SearchTables {
    fn default() -> Self {
        Self::new()
    }
}

pub struct EngineState {
    pub stop_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub debug_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub zobrist_table: std::sync::RwLock<std::sync::Arc<ZobristTable>>,

    pub pv_nodes: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<u64, Turn>>>,
    pub pv_nodes_len: std::sync::Arc<std::sync::atomic::AtomicI32>,
    pub logger: std::sync::Arc<std::sync::RwLock<LoggerFn>>,
    pub log_sender: std::sync::mpsc::Sender<String>,

    /// Killers, history and counter moves, persistent across the iterative deepening loop.
    /// The engine searches on one thread -- `threads.rs` rejects `setoption Threads` -- so this
    /// lock is taken once per `get_moves` call and never on a search path.
    pub search_tables: std::sync::Mutex<SearchTables>,
}

pub struct SearchContext<'a> {
    pub zobrist_table: &'a ZobristTable,

    pub stop_flag: &'a AtomicBool,
    pub pv_nodes: &'a std::sync::Mutex<std::collections::HashMap<u64, Turn>>,
    pub killer_moves: [Option<Turn>; 2],
    /// `[side][from][to]`, indexed with [`history_side`]. See `SearchTables::history_table`.
    pub history_table: *const [[[i32; 64]; 64]; 2],
    pub counter_move: Option<Turn>,
    pub start_time: std::time::Instant,
    pub target_time: Option<i32>,
    pub root_moves_total: i32,
    pub root_moves_searched: i32,
    /// Nominal depth of the current iterative deepening iteration. Search extensions
    /// use it to derive how much of the extension budget the current path has already
    /// consumed: without extensions `depth == root_depth - ply` holds exactly.
    pub root_depth: i32,
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
    /// Index this move was generated at, stamped by `MoveList::push`. It is the tie-break of the
    /// search's move order; see `Turn::precedes`. It fits in the padding the struct already had,
    /// so `Turn` is still 16 bytes.
    pub order: u8,
    pub gives_check: bool,
    pub eval: i16,
    pub has_hashed_eval: bool,
    pub rank: i32,
}

impl Turn {
    /// The total order the search selects moves in: rank first, then the move's own identity.
    ///
    /// Rank alone is not a total order. Every quiet without a history entry ranks 0, and so does
    /// a capture whose attacker penalty exceeds its victim's value, because the ranking clamps at
    /// zero -- so the tie classes are large. The selection scans resolve a tie by array position
    /// and then `swap` the winner into place, which permutes the part of the list they have not
    /// examined yet, so the searched order is a function of the swap history rather than of the
    /// position. No picker that generates its moves in a different order can reproduce that.
    ///
    /// Breaking the tie on the generation index makes the order a property of the move set
    /// alone, and it is the tie-break the scans already intend: the first move they select at a
    /// node is the earliest-generated of the leaders, and only the swaps afterwards depart from
    /// that. A picker that generates the same moves in the same sequence reproduces this order
    /// exactly, whatever it defers.
    pub fn precedes(&self, other: &Turn) -> bool {
        self.rank > other.rank
    }
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
            order: 0,
            gives_check,
            eval,
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
            order: 0,
            gives_check: false,
            eval: 0,
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
                order: 0,
                gives_check: false,
                eval: 0,
                has_hashed_eval: false,
                rank: 0,
            }; 256],
            len: 0,
        }
    }

    pub fn push(&mut self, mut turn: Turn) {
        if self.len < 256 {
            // The generation index is the tie-break of the search's move order, so it is stamped
            // where a move enters the list and nowhere else, and folded into the low bits of the
            // rank so that comparing two ranks is comparing the whole order.
            //
            // The rank may be negative. Since `task.md` 23.3 a refuted quiet carries a negative
            // history entry and ranks below `BAND_QUIET`, and a capture the SEE demoted has been
            // reaching about -256,000,000 for longer than that. The shift is still safe: the low
            // `RANK_TIEBREAK_BITS` are zero afterwards whatever the sign, so the `|` below is an
            // addition, `rank * 256 + tiebreak` stays monotone, and the extremes -- a quiet at
            // -MAX_HISTORY and `BAND_TT` -- are both far inside `i32`.
            turn.order = self.len as u8;
            turn.rank = (turn.rank << RANK_TIEBREAK_BITS) | (u8::MAX - turn.order) as i32;
            self.moves[self.len] = turn;
            self.len += 1;
        }
    }

    /// Empties the list without touching its storage.
    ///
    /// The generators append, and the buffers are reused across nodes, so a caller that
    /// generates into a list it did not just construct has to clear it first.
    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn as_slice(&self) -> &[Turn] {
        &self.moves[0..self.len]
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// Bits of `Turn::rank` reserved for the move order's tie-break.
///
/// `MoveList::push` shifts the rank the generator computed up by this much and writes the
/// generation index into the space that opens up, so a single `rank` comparison is a total order
/// and the selection scans stay exactly the code they were. Every threshold the search compares a
/// rank against is shifted by the same amount; the raw rank is `rank >> RANK_TIEBREAK_BITS`.
pub const RANK_TIEBREAK_BITS: u32 = 8;

/// The bands of the move order, before the tie-break shift.
///
/// Each class of move owns a range, and the ranges nest, which is what lets the order be produced
/// one class at a time. The scheme they replace did not nest, in three separate places: a
/// promoting capture that gave check reached 310,000 and outranked the Transposition Table move
/// at 180,000; a killer carrying a full history entry reached 29,900 and crossed into the
/// minor-piece captures at 30,000; and a capture whose attacker penalty exceeded its victim's
/// value was clamped to zero, into the middle of the quiet moves it should have been ordered
/// against. Each band is a million wide and the widest score inside one is a queen promotion
/// capturing a queen, at 260,000.
/// The plane of the butterfly history that belongs to one side, `task.md` 23.2.
///
/// White is 0 and Black is 1. Every read and every write of `history_table` goes through this
/// function, so the two can never disagree about the convention, and the argument names the side
/// that *played* the move rather than the side to move at the reading node.
#[inline(always)]
pub const fn history_side(white: bool) -> usize {
    if white { 0 } else { 1 }
}

/// The cap the butterfly history converges towards, `task.md` 23.3.
///
/// The table is signed: a quiet move that has been refuted reaches a negative entry and is
/// distinguishable from one that has never been searched, which reads 0. That distinction is the
/// whole point of the signed table — `lmr_history_bad_threshold` is meant to fire on refuted
/// moves, and against an unsigned table that saturates at zero it fired on unseen ones.
pub const MAX_HISTORY: i32 = 16_384;

/// The gravity update, the single place a history entry is written, `task.md` 23.3.
///
/// `*entry += bonus - entry * |bonus| / MAX_HISTORY` converges towards `+/- MAX_HISTORY` instead
/// of clamping at it: the correction term grows with the entry, so a value near the cap barely
/// moves and one near zero takes the bonus almost whole. That is what removes the global
/// rescaling pass the unsigned table needed — an entry cannot leave the range on its own, so
/// nothing has to walk 4096 entries to pull it back.
///
/// `bonus` is clamped first. A single update may not exceed the cap, or the correction term
/// could overshoot past `-MAX_HISTORY` on the first malus applied to a fresh entry.
#[inline(always)]
pub fn history_gravity(entry: &mut i32, bonus: i32) {
    let bonus = bonus.clamp(-MAX_HISTORY, MAX_HISTORY);
    *entry += bonus - (*entry) * bonus.abs() / MAX_HISTORY;
}

pub const BAND_TT: i32 = 5_000_000;
pub const BAND_PROMOTION: i32 = 4_000_000;
pub const BAND_CAPTURE: i32 = 3_000_000;
pub const BAND_KILLER: i32 = 2_000_000;
pub const BAND_QUIET: i32 = 0;

/// Levels in the per-search buffer arena.
///
/// `search_service::MAX_PLY` is 128 and bounds the plies a search can reach. The levels beyond
/// it cover the re-entries that search the same node again at the same ply -- the null-move
/// verification, razoring and the singular verification -- each of which takes a level without
/// advancing `ply`. A search that runs past the end falls back to buffers on the stack, so this
/// number bounds throughput and never correctness.
pub const SEARCH_LEVELS: usize = 2 * 128;

/// The two buffers one search node fills: the move list it generates into, and the
/// principal-variation array it hands to its children.
///
/// Both used to be constructed at every node. `MoveList::new()` writes 256 `Turn` values of 16
/// bytes and the principal variation another 128 `Option<Turn>`, about 6 KB of stores per node
/// for storage that is never read back: `MoveList::len` bounds every read of the first, and
/// `minimax` clears the second on entry. Reusing one set per recursion level therefore cannot
/// move the search tree.
pub struct NodeBuffers {
    pub moves: MoveList,
    /// Sized like every `pv` parameter in the search.
    pub pv: [Option<Turn>; 128],
}

impl NodeBuffers {
    pub fn new() -> Self {
        Self {
            moves: MoveList::new(),
            pv: [None; 128],
        }
    }
}

/// Allocates the arena once per search. This is the only allocation the search makes, and it is
/// made before the first node rather than inside one.
pub fn new_search_buffers() -> Vec<NodeBuffers> {
    (0..SEARCH_LEVELS).map(|_| NodeBuffers::new()).collect()
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
    pub pawn_key: u64,
    pub en_passante: i8,
    pub capture: u8,
    pub moved_piece: u8,
    pub old_pst_mg: i16,
    pub old_pst_eg: i16,
    /// `Board::irreversible_floor` as it stood before the move, restored on undo.
    pub old_irreversible_floor: usize,
}

impl MoveInformation {
    // Constructor
    pub fn new(castle_information: CastleInformation, hash: u64, pawn_key: u64, en_passante: i8, capture: u8, moved_piece: u8, old_pst_mg: i16, old_pst_eg: i16, old_irreversible_floor: usize) -> Self {
        MoveInformation {
            castle_information,
            hash,
            pawn_key,
            en_passante,
            capture,
            moved_piece,
            old_pst_mg,
            old_pst_eg,
            old_irreversible_floor,
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
    /// Zobrist hashes of every position reached by a move, indexed by ply. Replaces the
    /// per-node `HashMap` insert and remove that `do_move`/`undo_move` used to perform.
    pub history_hashes: [u64; MAX_HISTORY_PLIES],
    /// Number of valid entries in `history_hashes`.
    pub history_len: usize,
    /// Index into `history_hashes` below which a repetition can no longer occur, because an
    /// irreversible move (capture or pawn move) lies there. The Zobrist hash covers material
    /// and pawn placement, so no earlier position can ever match again.
    pub irreversible_floor: usize,
    pub cached_hash: u64,
    pub pawn_key: u64,
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

        let mut board = Board {
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
            history_hashes: [0; MAX_HISTORY_PLIES],
            history_len: 0,
            irreversible_floor: 0,
            cached_hash: 0,
            pawn_key: 0,
            pst_mg,
            pst_eg,
            _white_king_on_board,
            _black_king_on_board,
        };
        board.cached_hash = zobrist::gen_hash(&board);
        board.pawn_key = zobrist::gen_pawn_hash(&board);
        board
    }

    #[inline(always)]
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
        let old_cached_hash = self.cached_hash;
        let new_cached_hash = crate::zobrist::calc_incremental_hash(self, turn);
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
        let old_pawn_key = self.pawn_key;

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

        // Record the position and detect a threefold repetition without touching the heap.
        self.cached_hash = new_cached_hash;
        let old_irreversible_floor = self.irreversible_floor;
        if self.history_len < MAX_HISTORY_PLIES {
            self.history_hashes[self.history_len] = self.cached_hash;
            self.history_len += 1;

            // A capture or a pawn move can never be undone by later play, so no position
            // *preceding* it can recur. The position it produces still can, so the floor is the
            // index of that position rather than one past it.
            if actual_capture != 0 || moved_piece == 10 || moved_piece == 20 {
                self.irreversible_floor = self.history_len - 1;
            }

            // Only positions with the same side to move can repeat, hence the stride of two.
            let mut repetitions = 1;
            let mut idx = self.history_len as isize - 3;
            while idx >= self.irreversible_floor as isize {
                if self.history_hashes[idx as usize] == self.cached_hash {
                    repetitions += 1;
                    if repetitions == 3 {
                        self.game_status = GameStatus::Draw;
                        break;
                    }
                }
                idx -= 2;
            }
        }
        // Update pawn key
        let is_pawn = moved_piece == 10 || moved_piece == 20;
        if is_pawn {
            self.pawn_key ^= zobrist::get_zobrist_val(from as usize, moved_bb_idx);
            if !turn.is_promotion() {
                self.pawn_key ^= zobrist::get_zobrist_val(to as usize, moved_bb_idx);
            }
        }
        if actual_capture == 10 || actual_capture == 20 {
            let is_en_passant = (moved_piece == 10 || moved_piece == 20) && (to as i8 == old_field_for_en_passante);
            let capture_sq = if is_en_passant {
                if !self.white_to_move { to - 8 } else { to + 8 }
            } else {
                to
            };
            let capture_bb_idx = Board::piece_to_bb_idx(actual_capture);
            self.pawn_key ^= zobrist::get_zobrist_val(capture_sq as usize, capture_bb_idx);
        }

        MoveInformation::new(old_castle_information, old_cached_hash, old_pawn_key, old_field_for_en_passante, actual_capture, moved_piece, old_pst_mg, old_pst_eg, old_irreversible_floor)
    }


    pub fn undo_move(&mut self, turn: &Turn, move_information: MoveInformation) {
        self.cached_hash = move_information.hash;
        self.pawn_key = move_information.pawn_key;
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

        // Pop the position from the history.
        if self.history_len > 0 {
            self.history_len -= 1;
        }
        self.irreversible_floor = move_information.old_irreversible_floor;
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
            self.pawn_key == other.pawn_key &&
            self.history_hashes[..self.history_len] == other.history_hashes[..other.history_len]
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
        assert_eq!(board.history_len, 1);
        board.undo_move(turn, mi);
        assert_eq!(org_hash, board.hash());
        assert_eq!(board.history_len, 0);
    }

    #[test]
    fn zobrist_castling_do_undo_move_hash_test() {
        let fen_service = Service::new().fen;
        let mut board = fen_service.set_init_board();
        let org_hash = board.hash();

        // Move king e1e2 -> white loses both castling rights
        let turn = &NotationUtil::get_turn_from_notation("e1e2");
        let mi = board.do_move(turn);

        let new_hash = board.hash();
        assert_ne!(org_hash, new_hash, "Hash must change when king moves and castling rights are lost");
        assert_eq!(new_hash, crate::zobrist::gen_hash(&board), "Incremental/cached hash must equal gen_hash");

        board.undo_move(turn, mi);
        assert_eq!(org_hash, board.hash(), "Hash must restore after undoing king move");
        assert_eq!(org_hash, crate::zobrist::gen_hash(&board));
    }

    #[test]
    fn zobrist_en_passant_do_undo_move_hash_test() {
        let fen_service = Service::new().fen;
        let mut board = fen_service.set_init_board();
        let org_hash = board.hash();

        // Double pawn push e2e4 -> creates en-passant target at e3 (field 20)
        let turn1 = &NotationUtil::get_turn_from_notation("e2e4");
        let mi1 = board.do_move(turn1);
        let hash_after_e4 = board.hash();

        assert_ne!(org_hash, hash_after_e4);
        assert_eq!(board.field_for_en_passante, 20);
        assert_eq!(hash_after_e4, crate::zobrist::gen_hash(&board), "Hash after double pawn push must equal gen_hash");

        // Quiet move g8f6 -> clears en-passant target (field_for_en_passante becomes -1)
        let turn2 = &NotationUtil::get_turn_from_notation("g8f6");
        let mi2 = board.do_move(turn2);
        let hash_after_nf6 = board.hash();

        assert_eq!(board.field_for_en_passante, -1);
        assert_eq!(hash_after_nf6, crate::zobrist::gen_hash(&board), "Hash after en-passant clears must equal gen_hash");

        board.undo_move(turn2, mi2);
        assert_eq!(hash_after_e4, board.hash(), "Hash after undoing g8f6 must restore e4 hash");

        board.undo_move(turn1, mi1);
        assert_eq!(org_hash, board.hash(), "Hash after undoing e2e4 must restore init hash");
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
    fn push_stamps_the_generation_index_and_turn_stays_sixteen_bytes_test() {
        use super::{MoveList, Turn, RANK_TIEBREAK_BITS};

        assert_eq!(
            std::mem::size_of::<Turn>(),
            16,
            "the generation index has to fit the padding the struct already had"
        );

        let mut list = MoveList::new();
        for from in 0..4u8 {
            list.push(Turn::new(from, 20, 0, 0, false, 0));
        }
        assert_eq!(list.as_slice().iter().map(|t| t.order).collect::<Vec<_>>(), vec![0, 1, 2, 3]);

        // The rank the generator computed survives above the tie-break lane, and the lane below
        // it carries the generation index inverted, so that a lower index is the larger value.
        for turn in list.as_slice() {
            assert_eq!(turn.rank >> RANK_TIEBREAK_BITS, 0);
            assert_eq!(turn.rank & 0xFF, (u8::MAX - turn.order) as i32);
        }

        // Equal ranks resolve to the earlier-generated move, whatever order they are compared in.
        let first = list.as_slice()[1];
        let second = list.as_slice()[2];
        assert!(first.precedes(&second));
        assert!(!second.precedes(&first));

        // A better rank still wins, however late the move was generated.
        let mut better = second;
        better.rank += 1 << RANK_TIEBREAK_BITS;
        assert!(better.precedes(&first));
    }

    #[test]
    fn push_keeps_the_total_order_when_a_rank_is_negative_test() {
        use super::{MoveList, Turn, BAND_CAPTURE, BAND_QUIET, MAX_HISTORY, RANK_TIEBREAK_BITS};

        // `task.md` 23.3: a refuted quiet carries a negative history entry, so `push` shifts a
        // negative rank for the first time. This pins the arithmetic the comment in `push` now
        // claims -- the sign survives and the order does not invert.
        let mut list = MoveList::new();
        let mut refuted = Turn::new(1, 18, 0, 0, false, 0);
        refuted.rank = BAND_QUIET - MAX_HISTORY;
        let mut unseen = Turn::new(2, 19, 0, 0, false, 0);
        unseen.rank = BAND_QUIET;
        list.push(refuted);
        list.push(unseen);

        let refuted = list.as_slice()[0];
        let unseen = list.as_slice()[1];
        assert_eq!(refuted.rank >> RANK_TIEBREAK_BITS, BAND_QUIET - MAX_HISTORY,
            "the shift keeps both the sign and the magnitude");
        assert!(unseen.precedes(&refuted),
            "a quiet that was never searched has to outrank one that was refuted");

        // The worst quiet still sits above a capture the SEE demoted, which is where negative
        // ranks in this search came from before 23.3 existed.
        const SEE_DEMOTION: i32 = (BAND_CAPTURE + 1_000_000) << RANK_TIEBREAK_BITS;
        let demoted_capture = (BAND_CAPTURE << RANK_TIEBREAK_BITS) - SEE_DEMOTION;
        assert!(refuted.rank > demoted_capture,
            "the quiet band must stay above the demoted captures: {} vs {}",
            refuted.rank, demoted_capture);
    }

    #[test]
    fn history_gravity_converges_towards_the_cap_and_never_passes_it_test() {
        use super::{history_gravity, MAX_HISTORY};

        // `task.md` 23.3. The update converges towards the cap instead of clamping at it, which
        // is what removes the rescaling pass: nothing outside can pull an entry back, so the
        // update itself has to keep it inside.
        let bonus = 8 * 8; // a depth-8 cutoff
        let mut good = 0;
        for _ in 0..1000 {
            history_gravity(&mut good, bonus);
        }
        assert!(good <= MAX_HISTORY, "an entry may never pass the cap, read {good}");
        assert!(good > MAX_HISTORY * 9 / 10,
            "a thousand cutoffs should converge close to the cap, read {good}");

        let mut refuted = 0;
        for _ in 0..1000 {
            history_gravity(&mut refuted, -bonus);
        }
        assert!(refuted >= -MAX_HISTORY, "the same bound holds downwards, read {refuted}");
        assert!(refuted < -MAX_HISTORY * 9 / 10,
            "a thousand maluses should converge close to the negative cap, read {refuted}");

        // A single oversized bonus cannot leave the range either -- that is what the clamp
        // inside the update is for, and a fresh entry is the case that would overshoot.
        let mut fresh = 0;
        history_gravity(&mut fresh, i32::MAX / 2);
        assert!(fresh <= MAX_HISTORY, "one huge bonus must not pass the cap, read {fresh}");
        let mut fresh = 0;
        history_gravity(&mut fresh, -(i32::MAX / 2));
        assert!(fresh >= -MAX_HISTORY, "one huge malus must not pass the cap, read {fresh}");

        // At the cap the update is a no-op in the direction that would leave the range.
        let mut capped = MAX_HISTORY;
        history_gravity(&mut capped, MAX_HISTORY);
        assert_eq!(capped, MAX_HISTORY);
    }

    #[test]
    fn move_list_clear_empties_the_list_and_leaves_it_reusable_test() {
        use super::{MoveList, Turn};

        let mut list = MoveList::new();
        for i in 0..10 {
            list.push(Turn::new(i, 20, 0, 0, false, 0));
        }
        assert_eq!(list.len, 10);

        list.clear();
        assert!(list.is_empty());
        assert_eq!(list.as_slice().len(), 0, "a cleared list exposes nothing to a reader");

        list.push(Turn::new(63, 1, 0, 0, false, 0));
        assert_eq!(list.len, 1, "a cleared list starts a new node at index zero");
        assert_eq!(list.as_slice()[0].from, 63);
    }

    #[test]
    fn search_buffer_arena_has_a_level_for_every_ply_test() {
        use super::{new_search_buffers, SEARCH_LEVELS};

        let buffers = new_search_buffers();
        assert_eq!(buffers.len(), SEARCH_LEVELS);
        assert!(
            buffers.len() >= crate::search_service::MAX_PLY,
            "the stack fallback in minimax is the exception, not the normal path"
        );
        assert!(
            buffers.iter().all(|level| level.moves.is_empty()),
            "every level starts empty, so the first node at it generates from index zero"
        );
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

    #[test]
    fn test_search_tables_age_halves_the_history_and_leaves_the_other_two_alone() {
        // `task.md` 23.1. A persistent history has to decay, or an entry that earned its
        // cutoffs early in the game keeps its rank for the rest of it. Killers and counter
        // moves are overwritten wholesale by the next cutoff, so they are not aged.
        let mut tables = crate::model::SearchTables::new();
        let white = crate::model::history_side(true);
        let black = crate::model::history_side(false);
        tables.history_table[white][12][28] = 900;
        tables.history_table[white][1][18] = 1;
        // `task.md` 23.2: both planes decay. The decay is a property of how old an entry is, and
        // that is the same question for either side.
        tables.history_table[black][12][28] = 700;
        let killer = crate::model::Turn::new(12, 28, 0, 0, false, 0);
        tables.killer_moves[3][0] = Some(killer);
        tables.counter_moves[6][21] = Some(killer);

        tables.age();

        assert_eq!(tables.history_table[white][12][28], 450, "the history is halved on entry");
        assert_eq!(tables.history_table[white][1][18], 0,
                   "integer division retires the last point");
        assert_eq!(tables.history_table[black][12][28], 350, "both sides are aged");
        assert_eq!(tables.killer_moves[3][0], Some(killer), "killers are not aged");
        assert_eq!(tables.counter_moves[6][21], Some(killer), "counter moves are not aged");
    }

    #[test]
    fn test_the_two_history_planes_are_independent() {
        // `task.md` 23.2, the defect in one assertion: `Ng1-f3` refuting for White used to raise
        // the rank of `Ng8-f6` for Black, because both indexed the same `[from][to]` entry.
        let mut tables = crate::model::SearchTables::new();
        let white = crate::model::history_side(true);
        let black = crate::model::history_side(false);
        assert_ne!(white, black, "the two sides must not share a plane");

        tables.history_table[white][6][21] = 4096;
        assert_eq!(tables.history_table[black][6][21], 0,
                   "a White cutoff must not raise Black's entry for the same two squares");

        tables.history_table[black][6][21] = 512;
        assert_eq!(tables.history_table[white][6][21], 4096,
                   "and the reverse: Black's write must leave White's entry alone");
    }

    #[test]
    fn test_search_tables_reset_clears_all_three() {
        // What `ucinewgame` calls. Nothing learned about the previous game may transfer.
        let mut tables = crate::model::SearchTables::new();
        tables.history_table[crate::model::history_side(true)][12][28] = 900;
        tables.history_table[crate::model::history_side(false)][12][28] = 900;
        let killer = crate::model::Turn::new(12, 28, 0, 0, false, 0);
        tables.killer_moves[3][0] = Some(killer);
        tables.counter_moves[6][21] = Some(killer);

        tables.reset();

        assert_eq!(tables.history_table[crate::model::history_side(true)][12][28], 0);
        assert_eq!(tables.history_table[crate::model::history_side(false)][12][28], 0,
                   "both planes are cleared on `ucinewgame`");
        assert_eq!(tables.killer_moves[3][0], None);
        assert_eq!(tables.counter_moves[6][21], None);
    }

}
    #[test]
    fn incremental_hash_complex_sequence_test() {
        let fen_service = crate::fen_service::FenService;
        let mut board = fen_service.set_init_board();
        let move_gen = crate::move_gen_service::MoveGenService::new();
        let mut stats = crate::model::Stats::default();
        let config = crate::config::Config::for_tests();
        let stop_flag = std::sync::atomic::AtomicBool::new(false);
        let pv_nodes = std::sync::Mutex::new(std::collections::HashMap::new());
        let history_table = [[[0i32; 64]; 64]; 2];
        let zobrist_table = crate::zobrist::ZobristTable::with_capacity(1);
        
        let context = crate::model::SearchContext {
            zobrist_table: &zobrist_table,
            stop_flag: &stop_flag,
            pv_nodes: &pv_nodes,
            killer_moves: [None; 2],
            history_table: &history_table,
            counter_move: None,
            start_time: std::time::Instant::now(),
            target_time: None,
            root_moves_total: 0,
            root_moves_searched: 0,
            root_depth: 0,
        };

        // Assert start position
        assert_eq!(board.cached_hash, crate::zobrist::gen_hash(&board));

        // Generate pseudo-random moves by repeatedly taking the first generated valid move
        for _ in 0..20 {
            let mut valid_moves = crate::model::MoveList::new();
            move_gen.generate_valid_moves_list(&mut board, &mut stats, &config, &context, false, &mut valid_moves);
            
            if valid_moves.len == 0 {
                break;
            }

            let turn = valid_moves.moves[0];
            
            // Check that move generation computed the incremental hash correctly
            // manual_incremental check removed
            // turn.hash is removed, hash calculation is tested natively in do_move now
            
            let move_info = board.do_move(&turn);
            let actual_hash = crate::zobrist::gen_hash(&board);
            
            if board.cached_hash != actual_hash {
                println!("Move {} to {} piece {} capture {}", turn.from, turn.to, board.get_piece_at(turn.to), turn.capture);
                println!("Cached hash: {}", board.cached_hash);
                println!("Actual hash: {}", actual_hash);
                println!("{:?}", board);
                assert_eq!(board.cached_hash, actual_hash, "Incremental hash after do_move is wrong");
            }
            
            assert_eq!(board.cached_hash, actual_hash, "Board cached hash must match full re-hash after do_move");

            // Check undo move
            board.undo_move(&turn, move_info);
            let hash_after_undo = crate::zobrist::gen_hash(&board);
            assert_eq!(board.cached_hash, hash_after_undo, "Incremental hash after undo_move is wrong");
            
            // Do the move again to progress the sequence
            board.do_move(&turn);
        }
    }
